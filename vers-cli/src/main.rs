mod cli;
mod cmds;
mod errors;

use crate::cli::*;
use anyhow::Result;
pub use log::*;
use simplelog::*;
use vers_core::{config::Config, machine::Details};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::new()?;
    info!("{:?}", &cli);

    let verbose_logging = cli.verbose;

    CombinedLogger::init(vec![TermLogger::new(
        if verbose_logging {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        simplelog::Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    // Load the configuration file
    let config = Config::load(cli.config_file).unwrap_or_default();
    info!("{:?}", &config);

    // Describe the machine we are currently running on
    let os_details = Details::get()?;
    info!("{:?}", &os_details);

    // Handle subcommand if provided
    if let Some(subcommand) = cli.subcommand {
        info!("Handling subcommand {:?}", subcommand);
    }

    Ok(())
}
