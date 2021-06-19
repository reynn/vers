use crate::{cmds::*, errors::*};
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
    #[structopt(short, long, env)]
    pub env_name: Option<String>,
    /// A path to a Vers config file
    #[structopt(short, long, parse(from_os_str), env)]
    pub config_file: Option<PathBuf>,
    /// Subcommands to interact with your Vers environments
    #[structopt(subcommand)]
    pub subcommand: Option<CliSubCommands>,
    /// Add debug logging
    #[structopt(short, long)]
    pub verbose: bool,
}

impl Cli {
    pub fn new() -> Result<Cli> {
        Ok(Cli::from_args())
    }
}
