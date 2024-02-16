pub mod ssh;
pub mod ui;

use clap::Parser;
use std::error::Error;
use ui::{App, AppConfig};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the SSH configuration file
    #[arg(short, long, default_value = "~/.ssh/config")]
    config: String,

    /// Shows ProxyCommand
    #[arg(short, long, default_value_t = false)]
    proxy: bool,

    /// Host search filter
    #[arg(short, long)]
    search: Option<String>,

    /// Sort hosts by hostname
    #[arg(long, default_value_t = true)]
    sort: bool,

    /// Exit after ending the SSH session
    #[arg(short, long, default_value_t = false)]
    exit: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut app = App::new(&AppConfig {
        config_path: args.config,
        search_filter: args.search,
        sort_by_name: args.sort,
        display_proxy_command: args.proxy,
        exit_after_ssh: args.exit,
    })?;
    app.start()?;

    Ok(())
}
