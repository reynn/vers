use {
    crate::{
        environment::Environment,
        github,
        system::System,
        version::{parse_version, Version},
    },
    bpaf::*,
    log::*,
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Opts {
    #[bpaf(external(verbose))]
    pub verbose: usize,
    /// Environment where the tool will be installed to
    #[bpaf(short, long, fallback("global".to_string()))]
    pub env: String,
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
        /// Filter used to find the executable to link into the environment
        #[bpaf(short, long)]
        filter: Option<String>,
        /// Allow install of pre-release versions of the tool
        #[bpaf(short, long, fallback(false))]
        pre_release: bool,
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
        known: bool,
        #[bpaf(short, long, fallback(false))]
        current: bool,
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

pub async fn add_new_tool(
    name: &'_ str,
    system: &'_ System,
    env: &mut Environment,
    user_pattern: Option<String>,
) -> super::Result<()> {
    let split_name: Vec<&str> = name.split('@').collect();
    debug!("Split: {:?}. Length: {}", split_name, split_name.len());
    let org_repo = if split_name.len() > 1 {
        split_name[0]
    } else {
        name
    };
    let split_org_repo: Vec<&str> = org_repo.split('/').collect();
    let owner = split_org_repo[0];
    let repo = split_org_repo[1];

    let version = if split_name.len() > 1 {
        parse_version(split_name[1]).unwrap()
    } else {
        github::get_latest_release_tag(owner, repo).await?
    };

    info!("Installing {} version {}", org_repo, version);

    let release = github::get_specific_release_for_repo(owner, repo, &version, system).await?;

    match github::get_platform_specific_asset(&release, system, user_pattern) {
        Some(asset) => match env.add_tool(org_repo, version, asset).await {
            Ok(_) => info!("Successfully added {} to the environment", name),
            Err(err) => error!("Error adding tool to the environment. {:?}", err),
        },
        None => error!("No assets found for this OS and architecture combo"),
    }

    Ok(())
}

pub async fn remove_tool(name: &'_ str, env: &'_ Environment) -> super::Result<()> {
    info!("Removing {} from environment. {:?}", name, env);
    Ok(())
}

pub async fn list_tools(env: &'_ crate::environment::Environment) -> super::Result<()> {
    info!("Listing all tools available. {:?}", env);
    let tools = &env.tools;
    if !tools.is_empty() {
        tools.iter().for_each(|tool| println!("{}", tool.name));
    } else {
        println!(
            "No tools currently installed in the {} environment",
            env.name
        );
    }
    Ok(())
}
