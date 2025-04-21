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
        let mut parent_host = Host::new(Vec::new());
        let mut hosts = Vec::new();

        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            // We separate parts that contain comments with #
            line = line.split('#').next().unwrap().trim().to_string();
            if line.is_empty() {
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
                    let patterns = parse_patterns(&entry.1);
                    hosts.push(Host::new(patterns));

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
                        let (included_parent_host, included_hosts) = self.parse_raw(&mut file)?;

                        if hosts.is_empty() {
                            parent_host.extend_entries(&included_parent_host);
                        } else {
                            hosts
                                .last_mut()
                                .unwrap()
                                .extend_entries(&included_parent_host);
                        }

                        hosts.extend(included_hosts);
                    }

                    continue;
                }
                _ => {}
            }

            if hosts.is_empty() {
                parent_host.update(entry);
            } else {
                hosts.last_mut().unwrap().update(entry);
            }
        }

        Ok((parent_host, hosts))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufReader, Write};
    use tempdir::TempDir;

    #[test]
    fn test_basic_host_parsing() {
        let config = r#"
            Host example
              User testuser
              Port 22
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();
        let result = parser.parse(&mut reader).unwrap();

        assert_eq!(result.len(), 1);
        let patterns = result[0].get_patterns();
        assert!(patterns.contains(&"example".to_string()));
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "testuser");
        assert_eq!(result[0].get(&EntryType::Port).unwrap(), "22");
    }

    #[test]
    fn test_global_settings_applied_to_all_hosts() {
        let config = r#"
            User globaluser

            Host server1
              Port 22

            Host server2
              Port 2200
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();
        let result = parser.parse(&mut reader).unwrap();

        assert_eq!(result.len(), 2);
        for host in result {
            assert_eq!(host.get(&EntryType::User).unwrap(), "globaluser");
        }
    }

    #[test]
    fn test_include_file_parsing() {
        let include_content = r#"
            Host included
              Port 2222
        "#;

        let temp_dir = TempDir::new("sshs").unwrap();
        let temp_file_path = temp_dir.path().join("included_config");
        let mut temp_file = File::create(&temp_file_path).unwrap();
        write!(temp_file, "{}", include_content).unwrap();

        let config = format!(
            r#"
                Include {}
                Host main
                  Port 22
            "#,
            temp_file_path.display()
        );

        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();
        let result = parser.parse(&mut reader).unwrap();

        assert_eq!(result.len(), 2);
        let all_patterns: Vec<String> = result
            .iter()
            .flat_map(|host| host.get_patterns())
            .cloned()
            .collect();
        assert!(all_patterns.contains(&"included".to_string()));
        assert!(all_patterns.contains(&"main".to_string()));
    }

    #[test]
    fn test_unknown_entry_error_when_not_ignored() {
        let config = r#"
            BogusEntry something
            Host test
              Port 22
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let mut parser = Parser::new();
        parser.ignore_unknown_entries = false;

        let result = parser.parse(&mut reader);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::UnknownEntry(_)));
    }

    #[test]
    fn test_unknown_entry_ignored_when_flag_set() {
        let config = r#"
            BogusEntry something
            Host test
              Port 22
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();

        let result = parser.parse(&mut reader);
        assert!(result.is_ok());
        let hosts = result.unwrap();
        assert_eq!(hosts.len(), 1);
    }

    #[test]
    fn test_comment_lines_ignored() {
        let config = r#"
            # This is a comment
            Host test # trailing comment
              User testuser # inline comment
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();

        let result = parser.parse(&mut reader).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "testuser");
    }

    #[test]
    fn test_unparseable_line_error() {
        let config = r#"
            UnparseableLineWithoutValue
        "#;
        let mut reader = BufReader::new(config.as_bytes());
        let parser = Parser::new();

        let result = parser.parse(&mut reader);
        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnparseableLine(_)
        ));
    }

    #[test]
    fn test_parse_patterns_handles_quotes() {
        let patterns = parse_patterns(r#""host one" host2 "host three""#);
        assert_eq!(patterns, vec!["host one", "host2", "host three"]);
    }

    #[test]
    fn test_parse_file_from_path() {
        let content = r#"
            Host fromfile
              Port 2222
        "#;

        let temp_dir = TempDir::new("sshs").unwrap();
        let temp_file_path = temp_dir.path().join("included_config");
        let mut temp_file = File::create(&temp_file_path).unwrap();
        write!(temp_file, "{}", content).unwrap();

        let parser = Parser::new();
        let result = parser.parse_file(temp_file_path).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].get_patterns().contains(&"fromfile".to_string()));
    }
}
