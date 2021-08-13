use crate::{commands::*, prelude::*};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
#[structopt(
    name = "vers",
    about = "A general purpose tool manager similar to NVM, GVM and ASDF"
)]
pub struct Cli {
    /// The name of the environment to interact with
    ///
    /// If not provided will check for a `VERS_ENVIRONMENT` environment variable
    /// than the current working directory for a `.vers.toml` file.
    ///     If the file exists environment is determined by the file
    ///     If the file doesn't exist uses the default environment
    #[structopt(long, global = true, env = "VERS_ENV_NAME")]
    pub env_name: Option<String>,
    /// A path to a Vers config file
    #[structopt(
        short,
        long,
        global = true,
        parse(from_os_str),
        env = "VERS_CONFIG_FILE"
    )]
    pub config_file: Option<PathBuf>,
    /// Subcommands to interact with your Vers environments
    #[structopt(subcommand)]
    pub subcommand: Option<CliSubCommands>,
    /// Add debug logging
    #[structopt(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

impl Cli {
    pub fn new() -> Result<Cli> {
        Ok(Cli::from_args())
    }

    pub fn handle_subcommand(&self, env: &'_ Environment, cfg: &'_ Config) -> Result<()> {
        if let Some(subcommand) = &self.subcommand {
            match subcommand {
                CliSubCommands::Change(args) => change::execute_change_cmd(&args, &env, &cfg)?,
                CliSubCommands::Config(args) => config::execute_config_cmd(&args, &cfg)?,
                CliSubCommands::Environment(cmds) => env::execute_env_subcommand(&cmds.sub_cmd, &cfg)?,
                CliSubCommands::Install(args) => install::execute_install_cmd(&args, &env, &cfg)?,
                CliSubCommands::Uninstall(args) => uninstall::execute_uninstall_cmd(&args, &env, &cfg)?,
                CliSubCommands::List(args) => list::execute_list_command(&args, &env, &cfg)?,
                CliSubCommands::Completions(args) => completions::execute_completion_cmd(&args, &cfg)?,
                CliSubCommands::ExternalCommands(cmd_names) => handle_external_commands(cmd_names, &cfg)?,
            }
        } else {
            log::error!("No commands provided");
        }
        Ok(())
    }
}

fn handle_external_commands(cmd_names: &Vec<String>, cfg: &'_ Config) -> Result<()> {
    debug!("Names: {:?}", cmd_names);
    Ok(())
}

impl From<Cli> for Config {
    fn from(c: Cli) -> Self {
        let mut config = Config::load(c.config_file).unwrap_or_default();

        config.environment_name = if let Some(env) = c.env_name {
            env
        } else {
            "default".into()
        };

        config
    }
}
