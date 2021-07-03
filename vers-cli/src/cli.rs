use crate::{cmds::*, prelude::*};
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
