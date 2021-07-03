mod cli;
mod cmds;
mod errors;
mod prelude;

use crate::{cli::*, cmds::*};
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
    let env_name = &cli.env_name.unwrap_or_else(|| "default".into());
    let environment = Environment::find_env_by_name(env_name, &config.environment_directory)
        .unwrap_or_else(|e| {
            warn!(
                "Environment, {}, doesn't exist, initializing default environment. [{}]",
                env_name, e
            );
            Default::default()
        });

    // Handle subcommand if provided
    if let Some(subcommand) = cli.subcommand {
        match subcommand {
            CliSubCommands::Change(args) => {
                change::execute_change_cmd(&args, &environment, &config)?
            }
            CliSubCommands::Config(args) => config::execute_config_cmd(&args, &config)?,
            CliSubCommands::Environment(cmds) => {
                env::execute_env_subcommand(&cmds.sub_cmd, &config)?
            }
            CliSubCommands::Install(args) => {
                install::execute_install_cmd(&args, &environment, &config)?
            }
            CliSubCommands::Uninstall(args) => {
                uninstall::execute_uninstall_cmd(&args, &environment, &config)?
            }
            CliSubCommands::List(args) => list::execute_list_command(&args, &environment, &config)?,
            CliSubCommands::Completions(args) => {
                completions::execute_completion_cmd(&args, &config)?
            }
        }
    } else {
        error!("No commands provided");
    }
    Ok(())
}
