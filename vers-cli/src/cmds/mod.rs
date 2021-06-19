pub mod change;
pub mod config;
pub mod env;
pub mod install;
pub mod list;
pub mod uninstall;

use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub enum CliSubCommands {
    /// Update $PATH variable with different environments or tool versions
    #[structopt(aliases = &["ch"])]
    Change(change::ChangeCmd),
    /// Generate a new config or change config values in an existing config file.
    #[structopt(aliases = &["cfg", "c"])]
    Config(config::ConfigCmd),
    /// Manage environments, a default environment is generated automatically
    #[structopt(aliases = &["env", "envs"])]
    Environment(env::EnvironmentCmd),
    /// Install a tool into an environment
    #[structopt(aliases = &["add", "i"])]
    Install(install::InstallCmd),
    /// Remove a tool from an environment
    #[structopt(aliases = &["remove", "rm"])]
    Uninstall(uninstall::UninstallCmd),
    /// List all tools and their current versions for the current environment
    #[structopt(aliases = &["ls"])]
    List(list::ListCmd),
}
