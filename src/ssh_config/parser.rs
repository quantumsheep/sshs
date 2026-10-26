use glob::glob;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use super::host::Entry;
use super::parser_error::InvalidIncludeError;
use super::parser_error::InvalidIncludeErrorDetails;
use super::parser_error::ParseError;
use super::parser_error::UnknownEntryError;
use super::{EntryType, Host};

const MAX_INCLUDE_DEPTH: usize = 16;

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
        let include_base = include_base_for(path.as_ref());

        let mut reader = BufReader::new(File::open(path)?);
        self.parse_with_base(&mut reader, &include_base)
    }

    /// # Errors
    ///
    /// Will return `Err` if the SSH configuration cannot be parsed.
    pub fn parse(&self, reader: &mut impl BufRead) -> Result<Vec<Host>, ParseError> {
        self.parse_with_base(reader, &shellexpand::tilde("~/.ssh"))
    }

    fn parse_with_base(
        &self,
        reader: &mut impl BufRead,
        include_base: &str,
    ) -> Result<Vec<Host>, ParseError> {
        let (global_host, mut hosts) = self.parse_raw(reader, 0, include_base)?;

        if !global_host.is_empty() {
            for host in &mut hosts {
                host.extend_if_not_contained(&global_host);
            }
        }

        Ok(hosts)
    }

    fn parse_raw(
        &self,
        reader: &mut impl BufRead,
        depth: usize,
        include_base: &str,
    ) -> Result<(Host, Vec<Host>), ParseError> {
        let mut parent_host = Host::new(Vec::new());
        let mut hosts = Vec::new();
        let mut in_match_block = false;

        let mut buf = String::new();
        while reader.read_line(&mut buf)? > 0 {
            let line = strip_comment(&buf).trim().to_string();
            buf.clear();

            if line.is_empty() {
                continue;
            }

            let entry = parse_line(&line)?;

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
                    in_match_block = false;

                    let patterns = parse_patterns(&entry.1);
                    hosts.push(Host::new(patterns));

                    continue;
                }
                EntryType::Match => {
                    in_match_block = true;
                    continue;
                }
                EntryType::Include => {
                    if in_match_block {
                        continue;
                    }

                    if depth >= MAX_INCLUDE_DEPTH {
                        return Err(InvalidIncludeError {
                            line,
                            details: InvalidIncludeErrorDetails::MaxDepthExceeded,
                        }
                        .into());
                    }

                    let mut include_path = shellexpand::tilde(&entry.1).to_string();

                    if !include_path.starts_with('/') {
                        include_path = format!("{include_base}/{include_path}");
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
                        let (included_parent_host, included_hosts) =
                            self.parse_raw(&mut file, depth + 1, include_base)?;

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

            if in_match_block {
                continue;
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

/// OpenSSH resolves relative Include paths against /etc/ssh for the
/// system configuration file, and against ~/.ssh otherwise.
fn include_base_for(path: &Path) -> String {
    if path.starts_with("/etc/ssh") {
        "/etc/ssh".to_string()
    } else {
        shellexpand::tilde("~/.ssh").to_string()
    }
}

/// Returns the files pulled in by `Include` directives (outside `Match` blocks)
/// of the configuration file at `path`, recursively, resolved the same way as
/// [`Parser::parse_file`]. Unreadable files and invalid patterns are skipped.
#[must_use]
pub fn included_files<P>(path: P) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    let include_base = include_base_for(path.as_ref());
    let mut files = Vec::new();
    collect_included_files(path.as_ref(), &include_base, 0, &mut files);
    files
}

fn collect_included_files(path: &Path, include_base: &str, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(file) = File::open(path) else {
        return;
    };

    let mut in_match_block = false;
    for line in BufReader::new(file).lines() {
        let Ok(line) = line else {
            return;
        };
        let line = strip_comment(&line).trim().to_string();
        if line.is_empty() {
            continue;
        }

        let Ok(entry) = parse_line(&line) else {
            continue;
        };

        match entry.0 {
            EntryType::Host => in_match_block = false,
            EntryType::Match => in_match_block = true,
            EntryType::Include if !in_match_block && depth < MAX_INCLUDE_DEPTH => {
                let mut include_path = shellexpand::tilde(&entry.1).to_string();
                if !include_path.starts_with('/') {
                    include_path = format!("{include_base}/{include_path}");
                }

                let Ok(paths) = glob(&include_path) else {
                    continue;
                };

                for included in paths.flatten() {
                    collect_included_files(&included, include_base, depth + 1, files);
                    files.push(included);
                }
            }
            _ => {}
        }
    }
}

/// Strips a trailing comment, keeping any `#` inside double quotes,
/// matching OpenSSH's tokenizer.
fn strip_comment(line: &str) -> &str {
    let mut in_double_quotes = false;

    for (i, c) in line.char_indices() {
        match c {
            '"' => in_double_quotes = !in_double_quotes,
            '#' if !in_double_quotes => return &line[..i],
            _ => {}
        }
    }

    line
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

    let entry_type = EntryType::from_str(key).unwrap_or(EntryType::Unknown(key.to_string()));

    // A value fully wrapped in double quotes (e.g. to preserve spaces) should
    // have the quotes stripped, matching ssh_config's own behavior. Host
    // patterns are excluded: they can list multiple quoted patterns and are
    // unquoted individually by `parse_patterns`.
    if entry_type != EntryType::Host
        && value.len() >= 2
        && value.starts_with('"')
        && value.ends_with('"')
    {
        value = &value[1..value.len() - 1];
    }

    Ok((entry_type, value.to_string()))
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
    use crate::test_support::testdata;

    #[test]
    fn test_basic_host_parsing() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("basic.conf")).unwrap();

        assert_eq!(result.len(), 1);
        let patterns = result[0].get_patterns();
        assert!(patterns.contains(&"example".to_string()));
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "testuser");
        assert_eq!(result[0].get(&EntryType::Port).unwrap(), "22");
    }

    #[test]
    fn test_global_settings_applied_to_all_hosts() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("global_settings.conf")).unwrap();

        assert_eq!(result.len(), 2);
        for host in result {
            assert_eq!(host.get(&EntryType::User).unwrap(), "globaluser");
        }
    }

    #[test]
    fn test_include_file_parsing() {
        let included_path = testdata("include/included.conf");
        let config = format!(
            "Include {}\nHost main\n  Port 22\n",
            included_path.display()
        );

        let mut reader = std::io::BufReader::new(config.as_bytes());
        let parser = Parser::new();
        let result = parser.parse(&mut reader).unwrap();

        assert_eq!(result.len(), 2);
        let all_patterns: Vec<String> = result
            .iter()
            .flat_map(Host::get_patterns)
            .cloned()
            .collect();
        assert!(all_patterns.contains(&"included".to_string()));
        assert!(all_patterns.contains(&"main".to_string()));
    }

    #[test]
    fn test_included_files_follows_nested_includes() {
        let dir = std::env::temp_dir().join("sshs_test_included_files");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("config.d")).unwrap();

        let work = dir.join("config.d/work.conf");
        let nested = dir.join("nested.conf");
        let in_match = dir.join("in_match.conf");
        std::fs::write(&work, format!("Include {}\nHost demo\n", nested.display())).unwrap();
        std::fs::write(&nested, "Host nested\n").unwrap();
        std::fs::write(&in_match, "Host matched\n").unwrap();

        let main = dir.join("config");
        std::fs::write(
            &main,
            format!(
                "Include {}/config.d/*.conf\nMatch tagged work\n  Include {}\n",
                dir.display(),
                in_match.display()
            ),
        )
        .unwrap();

        let files = included_files(&main);
        assert!(files.contains(&work));
        assert!(files.contains(&nested));
        assert!(!files.contains(&in_match));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_unknown_entry_error_when_not_ignored() {
        let mut parser = Parser::new();
        parser.ignore_unknown_entries = false;

        let result = parser.parse_file(testdata("unknown_entry.conf"));
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::UnknownEntry(err) => {
                assert!(
                    !err.line.is_empty(),
                    "error should carry the offending line"
                );
            }
            other => panic!("expected UnknownEntry error, got {other:?}"),
        }
    }

    #[test]
    fn test_unknown_entry_ignored_when_flag_set() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("unknown_entry.conf"));

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_comment_lines_ignored() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("comments.conf")).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "testuser");
    }

    #[test]
    fn test_unparseable_line_error() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("unparseable.conf"));

        assert!(matches!(
            result.unwrap_err(),
            ParseError::UnparseableLine(_)
        ));
    }

    #[test]
    fn test_match_block_entries_do_not_leak_into_previous_host() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("match_block.conf")).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "serveruser");
        assert_eq!(result[0].get(&EntryType::Port), None);
        assert_eq!(result[1].get(&EntryType::User), None);
        assert_eq!(result[1].get(&EntryType::Port), None);
    }

    #[test]
    fn test_self_including_file_errors_instead_of_overflowing() {
        let path = std::env::temp_dir().join("sshs_test_self_include.conf");
        std::fs::write(&path, format!("Include {}\nHost a\n", path.display())).unwrap();

        let parser = Parser::new();
        let result = parser.parse_file(&path);
        std::fs::remove_file(&path).ok();

        match result.unwrap_err() {
            ParseError::InvalidInclude(err) => {
                assert!(matches!(
                    err.details,
                    InvalidIncludeErrorDetails::MaxDepthExceeded
                ));
            }
            other => panic!("expected InvalidInclude error, got {other:?}"),
        }
    }

    #[test]
    fn test_hash_inside_quoted_value_is_kept() {
        let config = "Host test\n  ProxyCommand \"connect # not a comment\" # real comment\n";
        let mut reader = std::io::BufReader::new(config.as_bytes());

        let parser = Parser::new();
        let result = parser.parse(&mut reader).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].get(&EntryType::ProxyCommand).unwrap(),
            "connect # not a comment"
        );
    }

    #[test]
    fn test_parse_patterns_handles_quotes() {
        let patterns = parse_patterns(r#""host one" host2 "host three""#);
        assert_eq!(patterns, vec!["host one", "host2", "host three"]);
    }

    #[test]
    fn test_parse_file_from_path() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("basic.conf")).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].get_patterns().contains(&"example".to_string()));
    }

    #[test]
    fn test_identity_agent_with_quoted_path_with_spaces() {
        let parser = Parser::new();
        let result = parser
            .parse_file(testdata("identity_agent_with_spaces.conf"))
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].get(&EntryType::IdentityAgent).unwrap(),
            "~/Library/My Agent Dir/agent.sock"
        );
    }

    #[test]
    fn test_host_with_spaces_in_name() {
        let parser = Parser::new();
        let result = parser.parse_file(testdata("spaces_in_name.conf")).unwrap();

        assert_eq!(result.len(), 1);
        let patterns = result[0].get_patterns();
        assert!(patterns.contains(&"my Lab".to_string()));
        assert_eq!(result[0].get(&EntryType::Hostname).unwrap(), "192.168.1.2");
        assert_eq!(result[0].get(&EntryType::User).unwrap(), "root");
        assert_eq!(result[0].get(&EntryType::Port).unwrap(), "22");
    }
}
