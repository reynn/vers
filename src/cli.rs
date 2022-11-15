use {
    crate::{
        cli_actions::{self, Patterns, UpdateType},
        environment::Environment,
        system::System,
    },
    clap::{Parser, Subcommand, ValueEnum},
    clap_verbosity_flag::Verbosity,
    std::{fmt::Display, path::PathBuf},
    tracing::debug,
};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Opts {
    #[command(flatten)]
    pub verbose: Verbosity,
    /// Where to store the data application data
    #[arg(short, long, value_hint = clap::ValueHint::DirPath)]
    pub data_dir: Option<PathBuf>,
    /// Environment where the tool will be installed to
    #[arg(short, long, default_value = "global")]
    pub env: String,
    /// A GitHub API token to use authenticated requests to the API
    #[arg(long)]
    pub github_api_token: Option<String>,
    /// Use a local environment
    ///
    /// Files will be stored in the current directory under a "hidden" folder
    #[arg(short, long)]
    pub local: bool,
    #[command(subcommand)]
    pub action: Actions,
}

impl Default for Opts {
    fn default() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Actions {
    /// Add a tool to the designated environment
    Add {
        /// name of the tool to install to the environment.
        ///
        /// To install a specific version use name@version, for example:
        /// `cli/cli@v2.4.0` version should be a release tag.
        name: String,
        /// Alias to use instead of the repository name.
        ///
        /// This is how you will call the tool on the command line.
        #[arg(short = 'A', long)]
        alias: Option<String>,
        /// Pattern used to determine which file from the release to download.
        ///
        /// This can be used to override the autodetect mechanism to determine which assets to
        /// download.
        #[arg(short, long)]
        asset_pattern: Option<String>,
        /// Filter used to find the executable to link into the environment.
        #[arg(short, long)]
        file_filter: Option<String>,
        /// Allow install of pre-release versions of the tool.
        ///
        /// When `show` is provided this includes pre-release versions in the list,
        /// When it is not the most recent version is selected, that could be a pre-release or
        /// official release depending on release date.
        #[arg(short = 'P', long)]
        pre_release: bool,
        /// Show available versions
        ///
        /// Shows a list of versions available to install, multiple versions can be selected, the
        /// first selected will be set up to use in the environment.
        #[arg(short = 'S', long)]
        show: bool,
    },
    /// Remove a tool from the designated environment
    Remove {
        /// name of the tool to remove from the environment
        name: String,
        /// Remove all versions of a tool. Default is to delete the version used by the environment
        /// only.
        #[arg(short, long)]
        all: bool,
        /// Removes the symlink only while leaving the downloaded assets in tact for reuse later
        #[arg(short, long)]
        link_only: bool,
    },
    /// List tools available in the designated environment
    List {
        /// List all installed versions of tools available to the environment instead of just the
        /// currently used one.
        #[arg(short, long)]
        installed: bool,
        /// Control how the list is output to the console
        #[arg(short, long, default_value_t = ListOutputType::Table)]
        output: ListOutputType,
    },
    /// sync all version information with listed in the env config file.
    Sync,
    /// Update tools to the latest version available from GitHub.
    Update {
        /// Which tool to upgrade, when omitted all tools in the environment will be upgraded.
        name: Option<String>,
    },
    /// Generate shell completions for Vers to enable tab completions.
    Completions {
        /// the shell to generate completions for.
        #[arg(short, long)]
        shell: clap_complete::Shell,
    },
    /// show the exports required for setup.
    Env {
        /// Name of the environment.
        #[arg(short, long)]
        name: Option<String>,
        /// Prints out a command to set the environment path in the shells environment.
        #[arg(short, long)]
        shell: Option<clap_complete::Shell>,
        /// Output just the bath to the environment rather than a setup string.
        #[arg(short, long, default_value_t = false)]
        bare_path: bool,
    },
}

impl Actions {
    pub async fn execute(&self, config_dir: PathBuf, env: String) -> crate::Result<()> {
        match self {
            Actions::Add {
                name,
                alias,
                asset_pattern,
                file_filter,
                pre_release,
                show,
            } => {
                debug!("CLI: Name `{name}`, alias `{:?}`, pattern `{:?}`, filter `{:?}`, pre_release `{pre_release}`, show `{show}`", alias, asset_pattern, file_filter);
                let system = System::default();
                let mut env = Environment::load(&config_dir, &env).await?;
                cli_actions::add_new_tool(
                    &mut env,
                    name,
                    &system,
                    Patterns {
                        asset: asset_pattern.to_owned(),
                        file: file_filter.to_owned(),
                    },
                    alias.to_owned(),
                    *show,
                    *pre_release,
                )
                .await
            }
            Actions::Remove {
                name,
                all,
                link_only: _link_only,
            } => {
                let mut env = Environment::load(&config_dir, &env).await?;
                cli_actions::remove_tool(&mut env, name, *all).await
            }
            Actions::List { installed, output } => {
                let env = Environment::load(&config_dir, &env).await?;
                cli_actions::list_tools(&env, *installed, output.to_owned()).await
            }
            Actions::Update { name } => {
                let system = System::default();
                let mut env = Environment::load(&config_dir, &env).await?;
                cli_actions::update_tools(
                    &mut env,
                    &system,
                    if let Some(name) = name {
                        UpdateType::Specific(name.to_string())
                    } else {
                        UpdateType::All
                    },
                )
                .await
            }
            Actions::Completions { shell } => {
                cli_actions::generate_completions(*shell);
                Ok(())
            }
            Actions::Env {
                name,
                shell,
                bare_path,
            } => {
                let name: String = match name {
                    Some(name) => name.to_string(),
                    None => env,
                };
                let env = Environment::load(&config_dir, &name).await?;
                if *bare_path {
                    println!("{}", env.base_dir)
                } else if let Some(shell) = shell {
                    match shell {
                        clap_complete::Shell::Bash | clap_complete::Shell::Zsh => {
                            println!("export PATH=\"{}:$PATH\"", env.base_dir)
                        }
                        clap_complete::Shell::Elvish => todo!(),
                        clap_complete::Shell::Fish => println!("set -p PATH \"{}\"", env.base_dir),
                        clap_complete::Shell::PowerShell => {
                            println!("$env:Path += ';{}' ", env.base_dir)
                        }
                        _ => (),
                    }
                };
                Ok(())
            }
            Actions::Sync => {
                let system = System::default();
                let mut env = Environment::load(&config_dir, &env).await?;
                println!("Syncing versions with {} configuration.", env.name);
                cli_actions::sync_tools(&mut env, &system).await
            }
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum ListOutputType {
    Table,
    Text,
    Json,
}

impl Display for ListOutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            ListOutputType::Table => write!(f, "table"),
            ListOutputType::Text => write!(f, "text"),
            ListOutputType::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Shells {
    Fish,
    Zsh,
    Bash,
}
