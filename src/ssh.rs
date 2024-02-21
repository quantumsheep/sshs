use itertools::Itertools;
use regex::Regex;
use std::collections::VecDeque;
use std::error::Error;
use std::process::Command;

use crate::ssh_config;

#[derive(Debug, Clone)]
pub struct Host {
    pub hostname: String,
    pub aliases: String,
    pub user: Option<String>,
    pub target: String,
    pub port: Option<String>,
}

/// # Errors
///
/// Will return `Err` if the SSH command cannot be executed.
pub fn connect(host: &Host) -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("ssh");

    if let Some(user) = &host.user {
        command.arg(format!("{}@{}", user, host.target));
    } else {
        command.arg(host.target.clone());
    }

    if let Some(port) = &host.port {
        command.arg("-p").arg(port);
    }

    command.spawn()?.wait()?;

    Ok(())
}

/// # Format
/// - %h - Hostname
/// - %u - User
/// - %p - Port
///
/// Use %% to escape the % character.
///
/// # Errors
///
/// Will return `Err` if the command cannot be executed.
///
/// # Panics
///
/// Will panic if the regex cannot be compiled.
pub fn run_with_pattern(pattern: &str, host: &Host) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"(?P<skip>%%)|(?P<h>%h)|(?P<u>%u)|(?P<p>%p)").unwrap();
    let command = re.replace_all(pattern, |caps: &regex::Captures| {
        if let Some(p) = caps.name("skip") {
            p.as_str().to_string()
        } else if caps.name("h").is_some() {
            host.hostname.clone()
        } else if caps.name("u").is_some() {
            host.user.clone().unwrap_or_default()
        } else if caps.name("p").is_some() {
            host.port.clone().unwrap_or_default()
        } else {
            String::new()
        }
    });

    let mut args = shlex::split(&command)
        .ok_or(format!("Failed to parse command: {command}"))?
        .into_iter()
        .collect::<VecDeque<String>>();
    let command = args.pop_front().ok_or("Failed to get command")?;

    Command::new(command).args(args).spawn()?.wait()?;

    Ok(())
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
            hostname: host
                .get_patterns()
                .first()
                .unwrap_or(&String::new())
                .clone(),
            aliases: host.get_patterns().iter().skip(1).join(", "),
            user: host.get(&ssh_config::EntryType::User),
            target: host
                .get(&ssh_config::EntryType::Hostname)
                .unwrap_or_default(),
            port: host.get(&ssh_config::EntryType::Port),
        })
        .collect();

    Ok(hosts)
}
