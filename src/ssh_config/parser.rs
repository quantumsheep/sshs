use glob::glob;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

use super::host::Entry;
use super::parser_error::InvalidIncludeError;
use super::parser_error::InvalidIncludeErrorDetails;
use super::parser_error::ParseError;
use super::parser_error::UnknownEntryError;
use super::{EntryType, Host};

#[derive(Debug)]
pub struct Parser {
    ignore_unknown_entries: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    #[must_use]
    pub fn new() -> Parser {
        Parser {
            ignore_unknown_entries: true,
        }
    }

    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration cannot be parsed.
    pub fn parse_file<P>(&self, path: P) -> Result<Vec<Host>, ParseError>
    where
        P: AsRef<Path>,
    {
        let mut reader = BufReader::new(File::open(path)?);
        self.parse(&mut reader)
    }

    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration cannot be parsed.
    pub fn parse(&self, reader: &mut impl BufRead) -> Result<Vec<Host>, ParseError> {
        let (global_host, mut hosts) = self.parse_raw(reader)?;

        if !global_host.is_empty() {
            for host in &mut hosts {
                host.extend_if_not_contained(&global_host);
            }
        }

        Ok(hosts)
    }

    fn parse_raw(&self, reader: &mut impl BufRead) -> Result<(Host, Vec<Host>), ParseError> {
        let mut global_host = Host::new(Vec::new());
        let mut hosts = Vec::new();
        let mut current_host: Option<Host> = None;

        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            line = line.trim().to_string();
            if line.is_empty() || line.starts_with('#') {
                line.clear();
                continue;
            }

            let entry = parse_line(&line)?;
            line.clear();

            match entry.0 {
                EntryType::Unknown(_) => {
                    if !self.ignore_unknown_entries {
                        return Err(UnknownEntryError {
                            line,
                            entry: entry.0.to_string(),
                        }
                        .into());
                    }
                }
                EntryType::Host => {
                    if let Some(host) = current_host.take() {
                        hosts.push(host);
                    }
                    let patterns = parse_patterns(&entry.1);
                    if patterns.contains(&"*".to_string()) {
                        global_host = Host::new(patterns.clone());
                        hosts.push(global_host.clone());
                    } else {
                        current_host = Some(Host::new(patterns));
                    }
                    continue;
                }
                EntryType::Include => {
                    let mut include_path = shellexpand::tilde(&entry.1).to_string();

                    if !include_path.starts_with('/') {
                        let ssh_config_directory = shellexpand::tilde("~/.ssh").to_string();
                        include_path = format!("{ssh_config_directory}/{include_path}");
                    }

                    let paths = match glob(&include_path) {
                        Ok(paths) => paths,
                        Err(e) => {
                            return Err(InvalidIncludeError {
                                line,
                                details: InvalidIncludeErrorDetails::Pattern(e),
                            }
                            .into())
                        }
                    };

                    for path in paths {
                        let path = match path {
                            Ok(path) => path,
                            Err(e) => {
                                return Err(InvalidIncludeError {
                                    line,
                                    details: InvalidIncludeErrorDetails::Glob(e),
                                }
                                .into())
                            }
                        };

                        let mut file = BufReader::new(File::open(path)?);
                        let (included_global_host, included_hosts) = self.parse_raw(&mut file)?;

                        global_host.extend_entries(&included_global_host);
                        hosts.extend(included_hosts);
                    }
                    continue;
                }
                _ => {}
            }

            if let Some(host) = current_host.as_mut() {
                host.update(entry);
            } else {
                global_host.update(entry);
            }
        }

        if let Some(host) = current_host {
            hosts.push(host);
        }

        Ok((global_host, hosts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_basic_host_parsing() {
        let config = "\nHost example\n  User test\n  Port 2222\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (_, hosts) = result.unwrap();
        assert_eq!(hosts.len(), 1);
        let host = hosts.first().unwrap();
        assert!(host.get(&EntryType::User).is_some());
        assert_eq!(host.get(&EntryType::User).unwrap(), "test");
        assert!(host.get(&EntryType::Port).is_some());
        assert_eq!(host.get(&EntryType::Port).unwrap(), "2222");
    }

    #[test]
    fn test_include_directive() {
        let config = "\nInclude other_config\nHost example\n  User test\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_host_wildcard() {
        let config = "\nHost *\n  Compression yes\nHost test\n  Compression no\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (global_host, hosts) = result.unwrap();
        assert!(!global_host.is_empty());
        assert!(global_host.get(&EntryType::Compression).is_some());
        assert_eq!(global_host.get(&EntryType::Compression).unwrap(), "yes");
        assert_eq!(hosts.len(), 2);
    }
}

fn parse_line(line: &str) -> Result<Entry, ParseError> {
    let (mut key, mut value) = line
        .trim()
        .split_once([' ', '\t', '='])
        .map(|(k, v)| (k.trim_end(), v.trim_start()))
        .ok_or(ParseError::UnparseableLine(line.to_string()))?;

    // Format can be key=value with whitespaces around the equal sign, strip the equal sign and whitespaces
    if key.ends_with('=') {
        key = key.trim_end_matches('=').trim_end();
    }
    if value.starts_with('=') {
        value = value.trim_start_matches('=').trim_start();
    }

    Ok((
        EntryType::from_str(key).unwrap_or(EntryType::Unknown(key.to_string())),
        value.to_string(),
    ))
}

fn parse_patterns(entry_value: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    let mut pattern = String::new();
    let mut in_double_quotes = false;

    for c in entry_value.chars() {
        if c == '"' {
            if in_double_quotes {
                patterns.push(pattern.trim().to_string());
                pattern.clear();

                in_double_quotes = false;
            } else {
                in_double_quotes = true;
            }
        } else if c.is_whitespace() {
            if in_double_quotes {
                pattern.push(c);
            } else if !pattern.is_empty() {
                patterns.push(pattern.trim().to_string());
                pattern.clear();
            }
        } else {
            pattern.push(c);
        }
    }

    if !pattern.is_empty() {
        patterns.push(pattern.trim().to_string());
    }

    patterns
}
