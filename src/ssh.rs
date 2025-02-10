use anyhow::anyhow;
use handlebars::Handlebars;
use itertools::Itertools;
use serde::Serialize;
use std::collections::VecDeque;
use std::process::Command;

use crate::ssh_config::{self, parser_error::ParseError, HostVecExt};

#[derive(Debug, Serialize, Clone)]
pub struct Host {
    pub name: String,
    pub aliases: String,
    pub user: Option<String>,
    pub destination: String,
    pub port: Option<String>,
    pub proxy_command: Option<String>,
}

impl Host {
    /// Uses the provided Handlebars template to run a command.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the command cannot be executed.
    ///
    /// # Panics
    ///
    /// Will panic if the regex cannot be compiled.
    pub fn run_command_template(&self, pattern: &str) -> anyhow::Result<()> {
        let handlebars = Handlebars::new();
        let rendered_command = handlebars.render_template(pattern, &self)?;

        println!("Running command: {rendered_command}");

        let mut args = shlex::split(&rendered_command)
            .ok_or(anyhow!("Failed to parse command: {rendered_command}"))?
            .into_iter()
            .collect::<VecDeque<String>>();
        let command = args.pop_front().ok_or(anyhow!("Failed to get command"))?;

        let status = Command::new(command).args(args).spawn()?.wait()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum ParseConfigError {
    Io(std::io::Error),
    SshConfig(ParseError),
}

impl From<std::io::Error> for ParseConfigError {
    fn from(e: std::io::Error) -> Self {
        ParseConfigError::Io(e)
    }
}

impl From<ParseError> for ParseConfigError {
    fn from(e: ParseError) -> Self {
        ParseConfigError::SshConfig(e)
    }
}

/// # Errors
///
/// Will return `Err` if the SSH configuration file cannot be parsed.
pub fn parse_config(raw_path: &String) -> Result<Vec<Host>, ParseConfigError> {
    let normalized_path = shellexpand::tilde(&raw_path).to_string();
    let path = std::fs::canonicalize(normalized_path)?;

    let hosts = ssh_config::Parser::new()
        .parse_file(path)?
        .apply_patterns()
        .apply_name_to_empty_hostname()
        .merge_same_hosts()
        .iter()
        .map(|host| Host {
            name: host
                .get_patterns()
                .first()
                .unwrap_or(&String::new())
                .clone(),
            aliases: host.get_patterns().iter().skip(1).join(", "),
            user: host.get(&ssh_config::EntryType::User),
            destination: host
                .get(&ssh_config::EntryType::Hostname)
                .unwrap_or_default(),
            port: host.get(&ssh_config::EntryType::Port),
            proxy_command: host.get(&ssh_config::EntryType::ProxyCommand),
        })
        .collect();

    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, write};
    use std::env::temp_dir;

    #[test]
    fn test_host_wildcard() {
        let config_file_path = temp_dir().join("ssh_test_host_wildcard");
        let config_contents = "\
        Host *\n\
            User global\n\
            Port 22\n\
        Host test\n\
            Hostname test-host\n";
        write(&config_file_path, config_contents).unwrap();

        let parsed_hosts = parse_config(&config_file_path.display().to_string());
        remove_file(&config_file_path).unwrap();
        assert!(parsed_hosts.is_ok());

        let hosts = parsed_hosts.unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "test");
        assert_eq!(hosts[0].user.as_deref(), Some("global"));
        assert_eq!(hosts[0].port.as_deref(), Some("22"));
        assert_eq!(hosts[0].destination, "test-host");
    }

    #[test]
    fn test_wildcard_in_include() {
        let config_file_path = temp_dir().join("ssh_test_wildcard_in_include");
        let include_file_path = temp_dir().join("ssh_test_wildcard_in_include_include");
        let include_contents = "\
        Host *\n\
            User fallback\n\
            Port 2022";
        write(&include_file_path, include_contents).unwrap();
        let config_contents = "\
        Host jumpbox\n\
            ProxyJump user@proxy.example.com\n\
        Host db\n\
            ProxyCommand ssh -W %h:%p jumpbox";
        let config = format!(
            "Include {}\n{}",
            include_file_path.display(),
            config_contents
        );
        write(&config_file_path, config).unwrap();

        let parsed_hosts = parse_config(&config_file_path.display().to_string());
        remove_file(&config_file_path).unwrap();
        remove_file(&include_file_path).unwrap();
        assert!(parsed_hosts.is_ok());

        let hosts = parsed_hosts.unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "jumpbox");
        assert_eq!(hosts[0].user.as_deref(), Some("fallback"));
        assert_eq!(hosts[0].port.as_deref(), Some("2022"));
        assert_eq!(hosts[1].name, "db");
        assert_eq!(hosts[1].user.as_deref(), Some("fallback"));
        assert_eq!(hosts[1].port.as_deref(), Some("2022"));
        assert_eq!(hosts[1].proxy_command.as_deref(), Some("ssh -W %h:%p jumpbox"));
    }

    #[test]
    fn test_include_inside_host() {
        let config_file_path = temp_dir().join("ssh_test_include_inside_host");
        let include_file_path = temp_dir().join("ssh_test_include_inside_host_include");
        write(&include_file_path, "User test_included").unwrap();

        let config = format!("\
            Host main\n\
                Port 2222\n\
                Include {}\n\
            Host second\n\
                Port 2223\n",
            include_file_path.display()
        );
        write(&config_file_path, config).unwrap();

        let parsed_hosts = parse_config(&config_file_path.display().to_string());
        remove_file(&config_file_path).unwrap();
        remove_file(&include_file_path).unwrap();
        assert!(parsed_hosts.is_ok());

        let hosts = parsed_hosts.unwrap();
        assert_eq!(hosts.len(), 2);
        assert_eq!(hosts[0].name, "main");
        assert_eq!(hosts[0].user.as_deref(), Some("test_included"));
        assert_eq!(hosts[0].port.as_deref(), Some("2222"));
        assert_eq!(hosts[1].name, "second");
        assert_eq!(hosts[1].user, None);
        assert_eq!(hosts[1].port.as_deref(), Some("2223"));
    }

    #[test]
    fn test_include_global_options() {
        let config_file_path = temp_dir().join("ssh_test_include_global_options");
        let include_file_path = temp_dir().join("ssh_test_include_global_options_include");
        write(&include_file_path, "User test_included").unwrap();

        let config = format!("\
            Include {}\n\
            Host main\n\
                Port 2222\n",
            include_file_path.display()
        );
        write(&config_file_path, config).unwrap();

        let parsed_hosts = parse_config(&config_file_path.display().to_string());
        remove_file(&config_file_path).unwrap();
        remove_file(&include_file_path).unwrap();
        assert!(parsed_hosts.is_ok());

        let hosts = parsed_hosts.unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "main");
        assert_eq!(hosts[0].user.as_deref(), Some("test_included"));
        assert_eq!(hosts[0].port.as_deref(), Some("2222"));
    }
}
