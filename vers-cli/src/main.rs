mod cli;
mod commands;
mod errors;
mod prelude;

use crate::cli::*;
use anyhow::Result;
use log::*;
use prelude::*;
use simplelog::{ColorChoice, CombinedLogger, Config as log_cfg, TermLogger, TerminalMode};

fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::new()?;
    info!("{:?}", cli);

    let log_cfg = log_cfg::default();

    CombinedLogger::init(vec![TermLogger::new(
        match cli.verbose {
            0 => LevelFilter::Error,
            1 => LevelFilter::Info,
            _ => LevelFilter::Debug,
        },
        log_cfg,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    // Load the configuration file
    let config: Config = cli.clone().into();
    info!("{:?}", &config);
    let env_name = cli.clone().env_name.unwrap_or_default();
    let environment = Environment::find_env_by_name(&env_name, &config.environment_directory)
        .unwrap_or_else(|e| {
            warn!(
                "Environment, {}, doesn't exist, initializing environment. [{}]",
                env_name, e
            );
            Default::default()
        });

    // Handle subcommand if provided
    cli.handle_subcommand(&environment, &config)?;
    Ok(())
}
