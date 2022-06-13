use clap::{Parser, Subcommand};
use clap_verbosity_flag::Verbosity;

pub fn new() -> Opts {
    Opts::parse()
}

#[derive(Debug, Clone, Parser)]
pub struct Opts {
    #[clap(flatten)]
    pub verbose: Verbosity,
    /// Environment where the tool will be installed to
    #[clap(short, long, default_value = "global")]
    pub env: String,
    /// A GitHub API token to use authenticated requests to the API
    #[clap(long)]
    pub github_api_token: Option<String>,
    /// Use a local environment
    ///
    /// Files will be stored in the current directory under a "hidden" folder
    #[clap(short, long)]
    pub local: bool,
    #[clap(subcommand)]
    pub action: Actions,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Actions {
    /// Add a tool to the designated environment
    Add {
        /// name of the tool to install to the environment
        ///
        /// To install a specific version use name@version, for example:
        /// `cli/cli@v2.4.0` version should be a release tag
        name: String,
        /// Alias to use instead of the repository name
        ///
        /// This is how the tool will be called from the command line
        #[clap(short, long)]
        alias: Option<String>,
        /// Pattern used to determine which file from the release to download
        ///
        /// This can be used to override the autodetect mechanism to determine which assets to
        /// download
        #[clap(short, long)]
        pattern: Option<String>,
        /// Filter used to find the executable to link into the environment
        #[clap(short, long)]
        filter: Option<String>,
        /// Allow install of pre-release versions of the tool
        #[clap(short = 'P', long)]
        pre_release: bool,
        /// Show available versions
        #[clap(short = 'S', long)]
        show: bool,
    },
    /// Remove a tool from the designated environment
    Remove {
        /// name of the tool to remove from the environment
        name: String,
        /// Remove all versions of a tool. Default is to delete the currently installed version
        #[clap(short, long)]
        all: bool,
    },
    /// List tools available in the designated environment
    List {
        #[clap(short, long)]
        installed: bool,
    },
    /// sync all version information with listed in the env config file
    Sync,
    /// Update tools to the latest version
    Update { name: Option<String> },
    /// show the exports required for setup
    Env {
        #[clap(short, long)]
        name: Option<String>,
        #[clap(short, long)]
        shell: String,
    },
}
