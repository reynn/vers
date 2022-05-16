use {
    crate::{
        environment::Environment,
        github,
        system::System,
        tool::Tool,
        version::{parse_version, Version},
        Result,
    },
    bpaf::*,
    log::*,
    skim::prelude::*,
    std::{io::Cursor, sync::Arc},
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Opts {
    #[bpaf(external(verbose))]
    pub verbose: usize,
    /// Environment where the tool will be installed to
    #[bpaf(short, long, fallback("global".to_string()))]
    pub env: String,
    /// A GitHub API token to use authenticated requests to the API
    #[bpaf(long)]
    pub github_api_token: Option<String>,
    /// Use a local environment
    ///
    /// Files will be stored in the current directory under a "hidden" folder
    #[bpaf(short, long, fallback(false))]
    pub local: bool,
    #[bpaf(external(actions))]
    pub action: Actions,
}

#[derive(Debug, Clone, Bpaf)]
pub enum Actions {
    /// Add a tool to the designated environment
    #[bpaf(command("add"))]
    Add {
        /// name of the tool to install to the environment
        ///
        /// To install a specific version use name@version, for example:
        /// `cli/cli@v2.4.0` version should be a release tag
        #[bpaf(positional("NAME"))]
        name: String,
        /// Alias to use instead of the repository name
        ///
        /// This is how the tool will be called from the command line
        #[bpaf(short, long)]
        alias: Option<String>,
        /// Pattern used to determine which file from the release to download
        ///
        /// This can be used to override the autodetect mechanism to determine which assets to
        /// download
        #[bpaf(short, long)]
        pattern: Option<String>,
        /// file pattern used to search for the binary once extracted
        #[bpaf(short('F'), long)]
        file_pattern: Option<String>,
        /// Filter used to find the executable to link into the environment
        #[bpaf(short, long)]
        filter: Option<String>,
        /// Allow install of pre-release versions of the tool
        #[bpaf(short('P'), long, fallback(false))]
        pre_release: bool,
        /// Show available versions
        #[bpaf(short('S'), long, fallback(false))]
        show: bool,
    },
    /// Remove a tool from the designated environment
    #[bpaf(command("remove"))]
    Remove {
        /// name of the tool to remove from the environment
        #[bpaf(positional("NAME"))]
        name: String,
        /// Remove all versions of a tool. Default is to delete the currently installed version
        #[bpaf(short, long, fallback(false))]
        all: bool,
    },
    /// List tools available in the designated environment
    #[bpaf(command("list"))]
    List {
        #[bpaf(short, long, fallback(false))]
        installed: bool,
        #[bpaf(short, long, fallback(false))]
        current: bool,
    },
    /// sync all version information with listed in the env config file
    #[bpaf(command("sync"))]
    Sync,
    /// Update tools to the latest version
    #[bpaf(command("update"))]
    Update {
        #[bpaf(positional("NAME"))]
        name: Option<String>,
    },
    /// show the exports required for setup
    #[bpaf(command("env"))]
    Env {
        #[bpaf(short, long)]
        name: Option<String>,
        #[bpaf(short, long)]
        shell: String,
    },
}

fn verbose() -> Parser<usize> {
    short('v')
        .long("verbose")
        .help("Increase the verbosity of output\nSpecify no more than 3 times\n-v -v -v or -vvv")
        .req_flag(())
        .many()
        .map(|xs| xs.len())
        .guard(|&x| x <= 3, "Cannot have more than 3 levels of verbosity")
}
