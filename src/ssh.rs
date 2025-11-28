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

/// Process raw host configurations, apply empty hostname logic and convert to Host structs
/// 
/// # Arguments
/// * `raw_hosts` - List of raw host configurations parsed from SSH config file
/// 
/// # Returns
/// List of processed Host structs
pub fn process_hosts(raw_hosts: Vec<ssh_config::Host>) -> Vec<Host> {
    // Apply configuration processing in optimal order and convert to Host structs
    raw_hosts
        .apply_name_to_empty_hostname()
        .apply_patterns()
        .merge_same_hosts()
        .iter()
        .map(|host| Host {
            name: host.get_patterns().first().unwrap_or(&String::new()).clone(),
            aliases: host.get_patterns().iter().skip(1).join(", "),
            user: host.get(&ssh_config::EntryType::User),
            destination: host.get(&ssh_config::EntryType::Hostname).unwrap_or_default(),
            port: host.get(&ssh_config::EntryType::Port),
            proxy_command: host.get(&ssh_config::EntryType::ProxyCommand),
        })
        .collect()
}

/// # Errors
///
/// Will return `Err` if the SSH configuration file cannot be parsed.
pub fn parse_config(raw_path: &String) -> Result<Vec<Host>, ParseConfigError> {
    let normalized_path = shellexpand::tilde(&raw_path).to_string();
    let path = std::fs::canonicalize(normalized_path)?;

    // Parse the raw configuration file
    let raw_hosts = ssh_config::Parser::new().parse_file(path)?;
    
    // Call the extracted processing logic
    let hosts = process_hosts(raw_hosts);

    Ok(hosts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_config::{EntryType, Host};

    #[test]
    fn test_process_hosts_with_aliases() {
        // 直接创建包含多个模式的Host对象
        let mut ssh_config_host = Host::new(vec![
            "server1".to_string(),
            "server2".to_string(),
            "dev-server".to_string()
        ]);
        
        // 添加配置项
        ssh_config_host.update((EntryType::Hostname, "example.com".to_string()));
        ssh_config_host.update((EntryType::User, "testuser".to_string()));
        ssh_config_host.update((EntryType::Port, "2222".to_string()));
        
        // 创建原始主机列表
        let raw_hosts = vec![ssh_config_host];
        
        // 调用process_hosts函数
        let hosts = process_hosts(raw_hosts);
        
        // 验证结果
        assert_eq!(hosts.len(), 1, "Should have one host entry");
        assert_eq!(hosts[0].name, "server1", "First pattern should be the name");
        assert_eq!(hosts[0].aliases, "server2, dev-server", "Remaining patterns should be aliases");
        assert_eq!(hosts[0].destination, "example.com", "Hostname should be correct");
        assert_eq!(hosts[0].user, Some("testuser".to_string()), "User should be correct");
        assert_eq!(hosts[0].port, Some("2222".to_string()), "Port should be correct");
    }
    
    #[test]
    fn test_process_hosts_with_empty_hostname() {
        // 测试没有设置Hostname的情况
        let mut ssh_config_host = Host::new(vec!["server1".to_string(), "server2".to_string()]);
        
        // 不设置Hostname，这样会应用第一个模式作为Hostname
        ssh_config_host.update((EntryType::User, "testuser".to_string()));
        
        let raw_hosts = vec![ssh_config_host];
        let hosts = process_hosts(raw_hosts);
        
        // 验证结果 - Hostname应该被设置为第一个模式
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "server1");
        assert_eq!(hosts[0].aliases, "server2");
        assert_eq!(hosts[0].destination, "server1", "Destination should be set to first pattern");
    }
}
