use anyhow::anyhow;
use handlebars::Handlebars;
use itertools::Itertools;
use serde::Serialize;
use std::collections::VecDeque;
use std::process::Command;

use crate::searchable::SearchableItem;
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

impl SearchableItem for Host {
    fn search_text(&self) -> &str {
        &self.name
    }
}

impl Host {
    /// Renders a Handlebars template using the host's fields.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the template cannot be rendered.
    pub fn render_command_template(&self, pattern: &str) -> anyhow::Result<String> {
        let handlebars = Handlebars::new();
        Ok(handlebars.render_template(pattern, &self)?)
    }

    /// Renders the given template and spawns it as a command, waiting for it to exit.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the command cannot be parsed or executed.
    pub fn spawn_command_template(
        &self,
        pattern: &str,
    ) -> anyhow::Result<std::process::ExitStatus> {
        let rendered_command = self.render_command_template(pattern)?;

        println!("Running command: {rendered_command}");

        let mut args = shlex::split(&rendered_command)
            .ok_or(anyhow!("Failed to parse command: {rendered_command}"))?
            .into_iter()
            .collect::<VecDeque<String>>();
        let command = args.pop_front().ok_or(anyhow!("Failed to get command"))?;

        Ok(Command::new(command).args(args).spawn()?.wait()?)
    }

    /// Uses the provided Handlebars template to run a command.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the command cannot be executed.
    pub fn run_command_template(&self, pattern: &str) -> anyhow::Result<()> {
        let status = self.spawn_command_template(pattern)?;
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

    const DEFAULT_TEMPLATE: &str = r#"ssh "{{{name}}}""#;

    fn testdata(name: &str) -> String {
        crate::test_support::testdata(name)
            .to_string_lossy()
            .into_owned()
    }

    fn load(config: &str) -> Vec<Host> {
        parse_config(&testdata(config)).expect("failed to parse config")
    }

    #[test]
    fn test_render_basic() {
        let hosts = load("basic.conf");
        assert_eq!(hosts.len(), 1);
        assert_eq!(
            hosts[0].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "example""#
        );
    }

    #[test]
    fn test_render_global_settings() {
        let hosts = load("global_settings.conf");
        assert_eq!(hosts.len(), 2);
        assert_eq!(
            hosts[0].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "server1""#
        );
        assert_eq!(
            hosts[1].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "server2""#
        );
    }

    #[test]
    fn test_render_comments() {
        let hosts = load("comments.conf");
        assert_eq!(hosts.len(), 1);
        assert_eq!(
            hosts[0].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "test""#
        );
    }

    #[test]
    fn test_render_unknown_entry() {
        let hosts = load("unknown_entry.conf");
        assert_eq!(hosts.len(), 1);
        assert_eq!(
            hosts[0].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "test""#
        );
    }

    #[test]
    fn test_render_spaces_in_name() {
        let hosts = load("spaces_in_name.conf");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "my Lab");
        assert_eq!(hosts[0].destination, "192.168.1.2");
        // Default template embeds the name — the space causes SSH to reject it.
        // The destination-based template is the correct workaround.
        assert_eq!(
            hosts[0].render_command_template(DEFAULT_TEMPLATE).unwrap(),
            r#"ssh "my Lab""#
        );
        assert_eq!(
            hosts[0]
                .render_command_template(r#"ssh "{{{destination}}}""#)
                .unwrap(),
            r#"ssh "192.168.1.2""#
        );
    }

    #[test]
    fn test_render_custom_template_fields() {
        let hosts = load("basic.conf");
        assert_eq!(hosts.len(), 1);
        assert_eq!(
            hosts[0]
                .render_command_template(r"ssh -p {{{port}}} {{{user}}}@{{{destination}}}")
                .unwrap(),
            "ssh -p 22 testuser@example"
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn testdata(name: &str) -> String {
        crate::test_support::testdata(name)
            .to_string_lossy()
            .into_owned()
    }

    /// Builds an SSH template that uses the test key and disables host key checking.
    /// The Handlebars fields `{{{port}}}`, `{{{user}}}` and `{{{destination}}}` are
    /// filled in at render time from the parsed host.
    fn docker_ssh_template() -> String {
        let key = crate::test_support::testdata("ssh/test_key");

        // Git only tracks the executable bit, not full file modes, so the key's
        // permissions are not guaranteed to survive a checkout. OpenSSH refuses
        // to use a private key that is group/other readable, so pin it down here.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
                .expect("failed to set test key permissions");
        }

        let key = key.to_string_lossy().into_owned();
        format!("ssh -i {key}")
            + " -o StrictHostKeyChecking=no"
            + " -o UserKnownHostsFile=/dev/null"
            + r" -p {{{port}}} {{{user}}}@{{{destination}}} echo connected"
    }

    #[test]
    fn test_run_command_template_docker() {
        let hosts = parse_config(&testdata("docker.conf")).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "Test Server");
        assert_eq!(hosts[0].destination, "127.0.0.1");

        // Use spawn_command_template rather than run_command_template: the latter
        // calls std::process::exit() on a non-zero exit status, which would abort
        // the whole test binary instead of just failing this test.
        let status = hosts[0]
            .spawn_command_template(&docker_ssh_template())
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn test_run_command_template_spaces_in_name() {
        let hosts = parse_config(&testdata("spaces_docker.conf")).unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "my Lab");
        assert_eq!(hosts[0].destination, "127.0.0.1");

        // The default template (`ssh "{{{name}}}"`) would pass "my Lab" as the
        // SSH target, which SSH rejects with "hostname contains invalid characters"
        // (issue #60). The destination-based template correctly uses the HostName.
        let status = hosts[0]
            .spawn_command_template(&docker_ssh_template())
            .unwrap();
        assert!(status.success());
    }
}
