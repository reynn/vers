use {
    clap::{ArgGroup, Parser, Subcommand, ValueEnum},
    clap_verbosity_flag::Verbosity,
    std::{fmt::Display, path::PathBuf},
};

#[derive(Debug, Clone, Parser)]
#[clap(author, version, about)]
pub struct Opts {
    #[clap(flatten)]
    pub verbose: Verbosity,
    /// Where to store the data application data
    #[clap(short,long, value_parser, value_hint = clap::ValueHint::DirPath)]
    pub data_dir: Option<PathBuf>,
    /// Environment where the tool will be installed to
    #[clap(short, long, value_parser, default_value = "global")]
    pub env: String,
    /// A GitHub API token to use authenticated requests to the API
    #[clap(long, value_parser)]
    pub github_api_token: Option<String>,
    /// Use a local environment
    ///
    /// Files will be stored in the current directory under a "hidden" folder
    #[clap(short, long, value_parser)]
    pub local: bool,
    #[clap(subcommand)]
    pub action: Actions,
}

impl Default for Opts {
    fn default() -> Self {
        Opts::parse()
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
        #[clap(value_parser)]
        name: String,
        /// Alias to use instead of the repository name.
        ///
        /// This is how you will call the tool on the command line.
        #[clap(short = 'A', long, value_parser)]
        alias: Option<String>,
        /// Pattern used to determine which file from the release to download.
        ///
        /// This can be used to override the autodetect mechanism to determine which assets to
        /// download.
        #[clap(short, long, value_parser)]
        asset_pattern: Option<String>,
        /// Filter used to find the executable to link into the environment.
        #[clap(short, long, value_parser)]
        file_filter: Option<String>,
        /// Allow install of pre-release versions of the tool.
        ///
        /// When `show` is provided this includes pre-release versions in the list,
        /// When it is not the most recent version is selected, that could be a pre-release or
        /// official release depending on release date.
        #[clap(short = 'P', long, value_parser)]
        pre_release: bool,
        /// Show available versions
        ///
        /// Shows a list of versions available to install, multiple versions can be selected, the
        /// first selected will be set up to use in the environment.
        #[clap(short = 'S', long, value_parser)]
        show: bool,
    },
    /// Remove a tool from the designated environment
    Remove {
        /// name of the tool to remove from the environment
        #[clap(value_parser)]
        name: String,
        /// Remove all versions of a tool. Default is to delete the version used by the environment
        /// only.
        #[clap(short, long, value_parser)]
        all: bool,
        /// Removes the symlink only while leaving the downloaded assets in tact for reuse later
        #[clap(short, long, value_parser)]
        link_only: bool,
    },
    /// List tools available in the designated environment
    List {
        /// List all installed versions of tools available to the environment instead of just the
        /// currently used one.
        #[clap(short, long, value_parser)]
        installed: bool,
        /// Control how the list is output to the console
        #[clap(short, long, value_parser, default_value_t = ListOutputType::Table)]
        output: ListOutputType,
    },
    /// sync all version information with listed in the env config file.
    Sync,
    /// Update tools to the latest version available from GitHub.
    Update {
        /// Which tool to upgrade, when omitted all tools in the environment will be upgraded.
        #[clap(value_parser)]
        name: Option<String>,
    },
    /// Generate shell completions for Vers to enable tab completions.
    Completions {
        /// the shell to generate completions for.
        #[clap(short, long, value_parser)]
        shell: clap_complete::Shell,
    },
    /// show the exports required for setup.
    #[clap(group(ArgGroup::new("output").required(true).args(&["shell", "bare-path"])))]
    Env {
        /// Name of the environment.
        #[clap(short, long, value_parser)]
        name: Option<String>,
        /// Prints out a command to set the environment path in the shells environment.
        #[clap(short, long, value_parser)]
        shell: Option<Shells>,
        /// Output just the bath to the environment rather than a setup string.
        #[clap(short, long, value_parser)]
        bare_path: bool,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Shells {
    Fish,
    Zsh,
    Bash,
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
