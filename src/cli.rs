use {
    clap::{Command, CommandFactory, Parser, Subcommand},
    clap_complete::Shell,
    clap_verbosity_flag::Verbosity,
};

pub fn new() -> Opts {
    Opts::parse()
}

#[derive(Debug, Clone, Parser)]
pub struct Opts {
    #[clap(flatten)]
    pub verbose: Verbosity,
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

impl Opts {
    pub fn cmd() -> Command<'static> {
        Opts::command()
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Actions {
    /// Add a tool to the designated environment
    Add {
        /// name of the tool to install to the environment
        ///
        /// To install a specific version use name@version, for example:
        /// `cli/cli@v2.4.0` version should be a release tag
        #[clap(value_parser)]
        name: String,
        /// Alias to use instead of the repository name
        ///
        /// This is how the tool will be called from the command line
        #[clap(short, long, value_parser)]
        alias: Option<String>,
        /// Pattern used to determine which file from the release to download
        ///
        /// This can be used to override the autodetect mechanism to determine which assets to
        /// download
        #[clap(short, long, value_parser)]
        pattern: Option<String>,
        /// Filter used to find the executable to link into the environment
        #[clap(short, long, value_parser)]
        filter: Option<String>,
        /// Allow install of pre-release versions of the tool
        #[clap(short = 'P', long, value_parser)]
        pre_release: bool,
        /// Show available versions
        #[clap(short = 'S', long, value_parser)]
        show: bool,
    },
    /// Remove a tool from the designated environment
    Remove {
        /// name of the tool to remove from the environment
        #[clap(value_parser)]
        name: String,
        /// Remove all versions of a tool. Default is to delete the currently installed version
        #[clap(short, long, value_parser)]
        all: bool,
    },
    /// List tools available in the designated environment
    List {
        #[clap(short, long, value_parser)]
        installed: bool,
    },
    /// sync all version information with listed in the env config file
    Sync,
    /// Update tools to the latest version
    Update {
        #[clap(value_parser)]
        name: Option<String>,
    },
    /// show the exports required for setup
    Env {
        #[clap(short, long, value_parser)]
        name: Option<String>,
        #[clap(short, long, value_parser)]
        shell: String,
    },
    Completions {
        #[clap(short, long, arg_enum, value_parser)]
        shell: Shell,
    },
}
