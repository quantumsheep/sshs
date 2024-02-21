use handlebars::Handlebars;
use itertools::Itertools;
use serde::Serialize;
use std::collections::VecDeque;
use std::error::Error;
use std::process::Command;

use crate::ssh_config::{self, HostVecExt};

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
    pub fn run_command_template(&self, pattern: &str) -> Result<(), Box<dyn Error>> {
        let handlebars = Handlebars::new();
        let command = handlebars.render_template(pattern, &self)?;

        let mut args = shlex::split(&command)
            .ok_or(format!("Failed to parse command: {command}"))?
            .into_iter()
            .collect::<VecDeque<String>>();
        let command = args.pop_front().ok_or("Failed to get command")?;

        Command::new(command).args(args).spawn()?.wait()?;

        Ok(())
    }
}

/// # Errors
///
/// Will return `Err` if the SSH configuration file cannot be parsed.
pub fn parse_config(raw_path: &String) -> Result<Vec<Host>, Box<dyn Error>> {
    let mut path = shellexpand::tilde(&raw_path).to_string();
    path = std::fs::canonicalize(path)?
        .to_str()
        .ok_or("Failed to convert path to string")?
        .to_string();

    let hosts = ssh_config::Parser::new()
        .parse_file(path)?
        .apply_patterns()
        .merge_same_hosts()
        .iter()
        .filter(|host| host.get(&ssh_config::EntryType::Hostname).is_some())
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
