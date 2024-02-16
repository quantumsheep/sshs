use itertools::Itertools;
use regex::Regex;
use ssh2_config::{ParseRule, SshConfig};
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::{io::BufReader, process::Command};

#[derive(Debug, Clone)]
pub struct Host {
    pub hostname: String,
    pub aliases: String,
    pub user: String,
    pub target: String,
    pub port: String,
}

/// # Errors
///
/// Will return `Err` if the SSH command cannot be executed.
pub fn connect(host: &Host) -> Result<(), Box<dyn Error>> {
    Command::new("ssh")
        .arg(format!("{}@{}", host.user, host.target))
        .arg("-p")
        .arg(&host.port)
        .spawn()?
        .wait()?;

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
pub fn run_with_pattern(pattern: &String, host: &Host) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"(?P<skip>%%)|(?P<h>%h)|(?P<u>%u)|(?P<p>%p)").unwrap();
    let command = re.replace_all(pattern, |caps: &regex::Captures| {
        if let Some(p) = caps.name("skip") {
            p.as_str().to_string()
        } else if caps.name("h").is_some() {
            host.hostname.clone()
        } else if caps.name("u").is_some() {
            host.user.clone()
        } else if caps.name("p").is_some() {
            host.port.clone()
        } else {
            String::new()
        }
    });

    let mut args = shlex::split(&command)
        .ok_or(format!("Failed to parse command: {}", command))?
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

    let mut reader = BufReader::new(File::open(path)?);
    let config = SshConfig::default().parse(&mut reader, ParseRule::STRICT)?;

    let hosts = config
        .get_hosts()
        .iter()
        .filter(|host| host.params.host_name.is_some())
        .map(|host| Host {
            hostname: host.pattern[0].pattern.clone(),
            aliases: host.pattern[1..]
                .iter()
                .map(|p| p.pattern.clone())
                .join(", "),
            user: host.params.user.clone().unwrap_or_default(),
            target: host.params.host_name.clone().unwrap_or_default(),
            port: host.params.port.unwrap_or(22).to_string(),
        })
        .collect();

    Ok(hosts)
}
