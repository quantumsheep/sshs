use itertools::Itertools;
use ssh2_config::{ParseRule, SshConfig};
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
