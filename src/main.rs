pub mod searchable;
pub mod ssh;
pub mod ssh_config;
#[cfg(test)]
mod test_support;
pub mod ui;

use anyhow::Result;
use clap::Parser;
use ui::{App, AppConfig};

const DEFAULT_COMMAND_TEMPLATE: &str = "ssh \"{{{name}}}\"";
const DEFAULT_CONFIG_PATHS: &[&str] = &["/etc/ssh/ssh_config", "~/.ssh/config"];

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
struct Args {
    /// Path to the SSH configuration file
    #[arg(
        short,
        long,
        num_args = 1..,
        default_values_t = [
            "/etc/ssh/ssh_config".to_string(),
            "~/.ssh/config".to_string(),
        ],
    )]
    config: Vec<String>,

    /// Shows `ProxyCommand`
    #[arg(long, default_value_t = false)]
    show_proxy_command: bool,

    /// Host search filter
    #[arg(short, long)]
    search: Option<String>,

    /// Color of the interface (a Tailwind palette name, for example blue, green, rose)
    #[arg(long, default_value = "blue")]
    color: String,

    /// Sort hosts by hostname (default)
    #[arg(long, default_value_t = true)]
    sort: bool,

    /// Do not sort hosts, keep the configuration file order
    #[arg(long, default_value_t = false)]
    no_sort: bool,

    /// Sort search results by fuzzy match score, best match first
    #[arg(long, default_value_t = false)]
    sort_fancy: bool,

    /// Handlebars template of the command to execute
    #[arg(short, long, default_value = DEFAULT_COMMAND_TEMPLATE)]
    template: String,

    /// Handlebars template of the command to execute when an SSH session starts
    #[arg(long, value_name = "TEMPLATE")]
    on_session_start_template: Option<String>,

    /// Handlebars template of the command to execute when an SSH session ends
    #[arg(long, value_name = "TEMPLATE")]
    on_session_end_template: Option<String>,

    /// Exit after ending the SSH session
    #[arg(short, long, default_value_t = false)]
    exit: bool,
}

/// When `-c/--config` points at a non-default file and the user left the default
/// command template alone, pass that file to `ssh` via `-F` so Host aliases
/// (and HostName/User/Port) from the custom config still resolve.
///
/// OpenSSH accepts a single `-F`; if several custom paths were given, use the
/// last non-system path so aliases from the user-supplied file win.
fn resolve_command_template(config_paths: &[String], template: &str) -> String {
    if template != DEFAULT_COMMAND_TEMPLATE {
        return template.to_string();
    }

    let using_defaults = config_paths.len() == DEFAULT_CONFIG_PATHS.len()
        && config_paths
            .iter()
            .zip(DEFAULT_CONFIG_PATHS.iter())
            .all(|(path, default)| path == default);
    if using_defaults {
        return template.to_string();
    }

    let path = config_paths
        .iter()
        .rev()
        .find(|path| path.as_str() != "/etc/ssh/ssh_config")
        .or_else(|| config_paths.last())
        .expect("clap guarantees at least one config path");

    let quoted = shlex::try_quote(path)
        .map(std::borrow::Cow::into_owned)
        .unwrap_or_else(|_| format!("\"{path}\""));

    format!("ssh -F {quoted} \"{}\"", "{{{name}}}")
}

fn main() -> Result<()> {
    let args = Args::parse();
    let command_template = resolve_command_template(&args.config, &args.template);

    let mut app = App::new(&AppConfig {
        config_paths: args.config,
        search_filter: args.search,
        color: args.color,
        sort_by_name: args.sort && !args.no_sort,
        sort_by_score: args.sort_fancy,
        show_proxy_command: args.show_proxy_command,
        command_template,
        command_template_on_session_start: args.on_session_start_template,
        command_template_on_session_end: args.on_session_end_template,
        exit_after_ssh_session_ends: args.exit,
    })?;
    app.start()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configs_keep_default_template() {
        let paths = DEFAULT_CONFIG_PATHS
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(
            resolve_command_template(&paths, DEFAULT_COMMAND_TEMPLATE),
            DEFAULT_COMMAND_TEMPLATE
        );
    }

    #[test]
    fn custom_config_injects_ssh_f_into_default_template() {
        let paths = vec!["ssh.config".to_string()];
        assert_eq!(
            resolve_command_template(&paths, DEFAULT_COMMAND_TEMPLATE),
            "ssh -F ssh.config \"{{{name}}}\""
        );
    }

    #[test]
    fn custom_config_with_spaces_is_shell_quoted() {
        let paths = vec!["my ssh.config".to_string()];
        assert_eq!(
            resolve_command_template(&paths, DEFAULT_COMMAND_TEMPLATE),
            "ssh -F 'my ssh.config' \"{{{name}}}\""
        );
    }

    #[test]
    fn explicit_template_is_left_alone() {
        let paths = vec!["ssh.config".to_string()];
        let custom = r#"ssh -o Foo=bar "{{{name}}}""#;
        assert_eq!(resolve_command_template(&paths, custom), custom);
    }

    #[test]
    fn multiple_custom_configs_use_last_non_system_path() {
        let paths = vec![
            "/etc/ssh/ssh_config".to_string(),
            "first.conf".to_string(),
            "second.conf".to_string(),
        ];
        assert_eq!(
            resolve_command_template(&paths, DEFAULT_COMMAND_TEMPLATE),
            "ssh -F second.conf \"{{{name}}}\""
        );
    }
}
