use {
    super::Result,
    crate::{
        environment::Environment,
        github,
        system::System,
        version::{parse_version, Version},
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

pub async fn add_new_tool(
    env: &mut Environment,
    name: &'_ str,
    system: &'_ System,
    user_pattern: Option<String>,
    file_pattern: Option<String>,
    alias: Option<String>,
    show: bool,
) -> super::Result<()> {
    let split_name: Vec<&str> = name.split('@').collect();
    trace!("Split: {:?}. Length: {}", split_name, split_name.len());
    let org_repo = if split_name.len() > 1 {
        split_name[0]
    } else {
        name
    };
    let split_org_repo: Vec<&str> = org_repo.split('/').collect();
    let owner = split_org_repo[0];
    let repo = split_org_repo[1];
    let alias = alias.unwrap_or_else(|| repo.to_string());
    let user_pattern = user_pattern.unwrap_or_else(|| "".to_string());
    let file_pattern = file_pattern.unwrap_or_else(|| alias.clone());

    let versions: Vec<String> = if split_name.len() > 1 {
        vec![split_name[1].to_string()]
    } else {
        let versions = github::get_repo_releases(owner, repo).await.unwrap();

        if show {
            let item_reader =
                SkimItemReader::default().of_bufread(Cursor::new(versions.join("\n")));
            Skim::run_with(
                &SkimOptionsBuilder::default()
                    .height(Some("75%"))
                    .multi(true)
                    .reverse(true)
                    .build()
                    .unwrap(),
                Some(item_reader),
            )
            .map(|items| {
                items
                    .selected_items
                    .iter()
                    .map(|item| item.text().to_string())
                    .collect()
            })
            .unwrap_or_default()
        } else {
            vec![versions.get(0).unwrap().to_string()]
        }
    };

    for version in versions.iter() {
        println!("Installing {} version {}", org_repo, version);

        let parsed_version = parse_version(version);
        let release =
            github::get_specific_release_for_repo(owner, repo, &parsed_version, system).await?;
        match github::get_platform_specific_asset(&release, system, &user_pattern) {
            Some(asset) => match env
                .add_tool(
                    org_repo,
                    &alias,
                    parsed_version,
                    asset,
                    &user_pattern,
                    &file_pattern,
                )
                .await
            {
                Ok(_) => println!("Successfully added {} to the environment", name),
                Err(err) => error!("Error adding tool to the environment. {:?}", err),
            },
            None => error!("No assets found for this OS and architecture combo"),
        }
    }
    Ok(())
}

pub async fn remove_tool(env: &'_ Environment, name: &'_ str) -> Result<()> {
    info!("Removing {} from environment. {:?}", name, env);
    Ok(())
}

pub async fn list_tools(env: &'_ Environment) -> Result<()> {
    info!("Listing all tools available. {:?}", env);
    let tools = &env.tools;

    if !tools.is_empty() {
        tools
            .iter()
            .for_each(|tool| println!("{}@{}", tool.name, tool.current_version));
        Ok(())
    } else {
        eyre::bail!(
            "No tools currently installed in the {} environment",
            env.name
        )
    }
}

pub enum UpdateType {
    All,
    Specific(String),
}

pub async fn update_tools(env: &mut Environment, update_type: UpdateType) -> Result<()> {
    let system = System::new();
    let tools: Vec<crate::tool::Tool> = env.tools.iter().map(|tool| tool.clone()).collect();

    match update_type {
        UpdateType::All => {
            println!("-> Updating all tools...");
            for tool in tools {
                println!("--> Updating tool {}", tool.name);
                let split_org_repo: Vec<&str> = tool.name.split('/').collect();
                let owner = split_org_repo[0];
                let repo = split_org_repo[1];

                let release =
                    github::get_specific_release_for_repo(owner, repo, &Version::Latest, &system)
                        .await
                        .unwrap();
                match github::get_platform_specific_asset(&release, &system, &tool.asset_pattern) {
                    Some(asset) => match env
                        .add_tool(
                            &tool.name,
                            &tool.alias,
                            Version::Latest,
                            asset,
                            &tool.asset_pattern,
                            &tool.file_pattern,
                        )
                        .await
                    {
                        Ok(_) => println!("---> {} has been updated.", &tool.name),
                        Err(add_tool_err) => error!(
                            "Error installing latest version of {}. {:?}",
                            &tool.name, add_tool_err
                        ),
                    },
                    None => error!(
                        "Unable to find an asset that matches '{}' for tool {}",
                        &tool.asset_pattern, &tool.name
                    ),
                }
            }
            Ok(())
        }
        UpdateType::Specific(tool_name) => {
            println!("-> Updating {}...", tool_name);
            Ok(())
        }
    }
}
