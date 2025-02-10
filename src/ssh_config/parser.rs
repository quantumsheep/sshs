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
        let mut is_in_host_block = false;
        let mut hosts = Vec::new();

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
                    let patterns = parse_patterns(&entry.1);
                    if !patterns.contains(&"*".to_string()) {
                        is_in_host_block = true;
                        hosts.push(Host::new(patterns));
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

                        if is_in_host_block {
                            // Can't include hosts inside a host block
                            if !included_hosts.is_empty() {
                                return Err(InvalidIncludeError {
                                    line,
                                    details: InvalidIncludeErrorDetails::HostsInsideHostBlock,
                                }
                                .into());
                            }

                            hosts
                                .last_mut()
                                .unwrap()
                                .extend_entries(&included_global_host);
                        } else {
                            if !included_global_host.is_empty() {
                                global_host.extend_entries(&included_global_host);
                            }

                            hosts.extend(included_hosts);
                        }
                    }

                    continue;
                }
                _ => {}
            }

            if is_in_host_block {
                hosts.last_mut().unwrap().update(entry);
            } else {
                global_host.update(entry);
            }
        }

        Ok((global_host, hosts))
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
    use std::io::Cursor;
    use std::fs::{remove_file, write};
    use std::env::temp_dir;

    #[test]
    fn test_one_host() {
        let config = "\
        Host example\n\
            User test\n\
            Port 2222\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (_, hosts) = result.unwrap();
        assert_eq!(hosts.len(), 1);
        let host = hosts.first().unwrap();
        assert_eq!(host.get(&EntryType::User).unwrap(), "test");
        assert_eq!(host.get(&EntryType::Port).unwrap(), "2222");
    }

    #[test]
    fn test_multiple_host() {
        let config = "\
        Host example\n\
            User test\n\
            Port 2221\n\
        Host example2\n\
            User test2\n\
            Port 2222\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (_, hosts) = result.unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].get_patterns()[0], "example");
        assert_eq!(hosts[0].get(&EntryType::User).unwrap(), "test");
        assert_eq!(hosts[0].get(&EntryType::Port).unwrap(), "2221");
        assert_eq!(hosts[1].get_patterns()[0], "example2");
        assert_eq!(hosts[1].get(&EntryType::User).unwrap(), "test2");
        assert_eq!(hosts[1].get(&EntryType::Port).unwrap(), "2222");
    }

    #[test]
    fn test_include_host() {
        let include_file = temp_dir().join("parser_test_include_host");
        write(&include_file, "\
        Host included\n\
            User test_included").unwrap();

        let config = format!(
            "Include {}\n\
            Host main\n\
                User test_main\n",
            include_file.display()
        );

        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        remove_file(&include_file).unwrap();
        assert!(result.is_ok());

        let (_, hosts) = result.unwrap();
        assert_eq!(hosts.len(), 2);

        let main_host = hosts.iter().find(|host| host.get_patterns()[0] == "main").unwrap();
        assert_eq!(main_host.get(&EntryType::User).unwrap(), "test_main");

        let included_host = hosts.iter().find(|host| host.get_patterns()[0] == "included").unwrap();
        assert_eq!(included_host.get(&EntryType::User).unwrap(), "test_included");
    }

    #[test]
    fn test_include_global_options() {
        let include_file = temp_dir().join("parser_test_include_global_options");
        write(&include_file, "User test_included").unwrap();

        let config = format!(
            "Include {}\n\
            Host main\n\
                User test_main\n",
            include_file.display()
        );

        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        remove_file(&include_file).unwrap();
        assert!(result.is_ok());

        let (global_host, hosts) = result.unwrap();
        assert!(!global_host.is_empty());
        assert_eq!(global_host.get(&EntryType::User).unwrap(), "test_included");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].get_patterns()[0], "main");
        assert_eq!(hosts[0].get(&EntryType::User).unwrap(), "test_main");
    }

    #[test]
    fn test_include_inside_host() {
        let include_file = temp_dir().join("parser_test_include_inside_host");
        write(&include_file, "User test_included").unwrap();

        let config = format!(
            "Host main\n\
                Port 2222\n\
                Include {}\n",
            include_file.display()
        );

        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        remove_file(&include_file).unwrap();
        assert!(result.is_ok());

        let (global_host, hosts) = result.unwrap();
        assert!(global_host.is_empty());
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].get_patterns()[0], "main");
        assert_eq!(hosts[0].get(&EntryType::User).unwrap(), "test_included");
        assert_eq!(hosts[0].get(&EntryType::Port).unwrap(), "2222");
    }

    // TODO: include is allowed inside a host block, but it should not override the previous values
    // #[test]
    // fn test_include_inside_host_no_override() {
    //     let include_file = temp_dir().join("parser_test_include_inside_host_no_override");
    //     write(&include_file, "User test_included").unwrap();
    //
    //     let config = format!(
    //         "Host main\n\
    //             User test_main\n\
    //             Include {}\n",
    //         include_file.display()
    //     );
    //
    //     let mut reader = Cursor::new(config);
    //     let result = Parser::new().parse_raw(&mut reader);
    //     remove_file(&include_file).unwrap();
    //     assert!(result.is_ok());
    //
    //     let (global_host, hosts) = result.unwrap();
    //     assert!(global_host.is_empty());
    //     assert_eq!(hosts.len(), 1);
    //     assert_eq!(hosts[0].get_patterns()[0], "main");
    //     assert_eq!(hosts[0].get(&EntryType::User).unwrap(), "test_main");
    // }

    #[test]
    fn test_include_host_inside_host_error() {
        let include_file = temp_dir().join("parser_test_include_host_inside_host_error");
        write(&include_file, "\
        Host included\n\
            User test_included").unwrap();

        let config = format!(
            "Host main\n\
                User test_main\n\
                Include {}\n",
            include_file.display()
        );

        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        remove_file(&include_file).unwrap();
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ParseError::InvalidInclude(InvalidIncludeError {
                details: InvalidIncludeErrorDetails::HostsInsideHostBlock,
                ..
            }))
        ));
    }

    #[test]
    fn test_host_wildcard() {
        let config = "\
        Host *\n\
            Compression yes\n\
            Port 22\n\
        Host test\n \
            User test-user\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (global_host, hosts) = result.unwrap();
        assert!(!global_host.is_empty());
        assert_eq!(global_host.get(&EntryType::Port).unwrap(), "22");
        assert_eq!(global_host.get(&EntryType::Compression).unwrap(), "yes");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].get(&EntryType::User).unwrap(), "test-user");
    }

    #[test]
    fn test_host_wildcard_only() {
        let config = "\
        Host *\n\
            Compression yes\n\
            Port 22\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (global_host, hosts) = result.unwrap();
        assert!(!global_host.is_empty());
        assert_eq!(global_host.get(&EntryType::Port).unwrap(), "22");
        assert_eq!(global_host.get(&EntryType::Compression).unwrap(), "yes");
        assert_eq!(hosts.len(), 0);
    }

    #[test]
    fn test_host_wildcard_multiple() {
        let config = "\
        Host *\n\
            Compression yes\n\
            Port 22\n\
        Host *\n\
            User global-user\n\
            ProxyCommand command\n";
        let mut reader = Cursor::new(config);
        let result = Parser::new().parse_raw(&mut reader);
        assert!(result.is_ok());
        let (global_host, hosts) = result.unwrap();
        assert!(!global_host.is_empty());
        assert_eq!(global_host.get(&EntryType::Port).unwrap(), "22");
        assert_eq!(global_host.get(&EntryType::Compression).unwrap(), "yes");
        assert_eq!(global_host.get(&EntryType::User).unwrap(), "global-user");
        assert_eq!(global_host.get(&EntryType::ProxyCommand).unwrap(), "command");
        assert_eq!(hosts.len(), 0);
    }
}
