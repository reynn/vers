use {
    crate::{
        environment::Environment, github, system::System, tool::Tool, version::parse_version,
        version::Version,
    },
    log::*,
    skim::prelude::*,
    std::io::Cursor,
};

pub async fn add_new_tool(
    env: &mut Environment,
    name: &'_ str,
    system: &'_ System,
    user_pattern: Option<String>,
    file_pattern: Option<String>,
    alias: Option<String>,
    show: bool,
    pre_release: bool,
) -> super::Result<()> {
    let split_name: Vec<&str> = name.split('@').collect();
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
    info!("Owner `{owner}`, Repo `{repo}`, Alias `{alias}`, Pattern `{user_pattern}`, Filter `{file_pattern}`");

    let versions: Vec<String> = if split_name.len() > 1 {
        vec![split_name[1].to_string()]
    } else {
        let versions = github::get_repo_releases(owner, repo, pre_release)
            .await
            .unwrap();

        // if the user wants a list of the releases show that, otherwise just get the first result
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
        let parsed_version = parse_version(version);

        let tool = Tool::new(
            org_repo,
            &alias,
            &Version::Latest,
            &user_pattern,
            &file_pattern,
        );

        match handle_tool_install(env, &tool, system, Some(parsed_version)).await {
            Ok(_) => println!("Installation of tool {} complete.", &tool.name),
            Err(install_err) => error!("{:?}", install_err),
        }
    }
    Ok(())
}

pub async fn remove_tool(
    env: &mut Environment,
    name: &'_ str,
    _remove_all_versions: bool,
) -> crate::Result<()> {
    info!("Removing {name} from environment. {:?}", env);
    Ok(())
}

pub async fn list_tools(env: &'_ Environment, installed: bool) -> crate::Result<()> {
    info!("Listing all tools available. {:?}", env);
    let tools = &env.tools;

    if !tools.is_empty() {
        if installed {
            tools.iter().for_each(|tool| {
                tool.installed_versions
                    .iter()
                    .for_each(|installed_version| println!("{}@{}", tool.name, installed_version));
            })
        } else {
            tools
                .iter()
                .for_each(|tool| println!("{}@{}", tool.name, tool.current_version));
        }
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

pub async fn update_tools(
    env: &mut Environment,
    system: &'_ System,
    update_type: UpdateType,
) -> crate::Result<()> {
    match update_type {
        UpdateType::All => {
            let tools: Vec<Tool> = env.tools.to_vec();
            println!("-> Updating all tools...");
            for tool in tools {
                match handle_tool_install(env, &tool, system, None).await {
                    Ok(_) => info!("Tool {} complete.", &tool.name),
                    Err(install_err) => error!("{:?}", install_err),
                }
            }
            Ok(())
        }
        UpdateType::Specific(tool_name) => {
            println!("-> Updating {tool_name}...");
            let tools = env.tools.to_vec();
            let split_name: Vec<&str> = tool_name.split('@').collect();
            let version = if split_name.len() == 2 {
                Some(parse_version(split_name[1]))
            } else {
                None
            };
            if let Some(tool) = tools.iter().find(|t| t.name == split_name[0]) {
                info!("Tool: {:?}", tool);

                match handle_tool_install(env, tool, system, version).await {
                    Ok(_) => info!("Tool {} has been updated.", &tool.name),
                    Err(install_err) => error!("{:?}", install_err),
                }
            } else {
                error!("{} is not found in the environment.", tool_name);
            }
            Ok(())
        }
    }
}

pub async fn sync_tools(env: &mut Environment, system: &'_ System) -> crate::Result<()> {
    let tools: Vec<Tool> = env.tools.to_vec();

    for tool in tools.iter() {
        let parsed_version = parse_version(&tool.current_version);
        match handle_tool_install(env, tool, system, Some(parsed_version)).await {
            Ok(_) => info!(
                "Tool {} has been installed at version {}",
                &tool.name, tool.current_version
            ),
            Err(install_err) => error!("{:?}", install_err),
        }
    }
    Ok(())
}

async fn handle_tool_install(
    env: &mut Environment,
    tool: &'_ Tool,
    system: &'_ System,
    version: Option<Version>,
) -> crate::Result<()> {
    let split_org_repo: Vec<&str> = tool.name.split('/').collect();
    let owner = split_org_repo[0];
    let repo = split_org_repo[1];

    let version = if let Some(version) = version {
        info!("Using provided version {}", version.as_tag());
        version
    } else {
        info!("Getting version from release tags");
        github::get_latest_release_tag(owner, repo).await.unwrap()
    };

    info!(
        "Comparing `{}` to `{}`",
        tool.current_version,
        version.as_tag()
    );
    if tool.current_version != version.as_tag() {
        println!("--> Installing tool {owner}/{repo}@{}", version.as_tag());
        let release = match github::get_specific_release_for_repo(owner, repo, &version).await {
            Ok(release) => release,
            Err(release_err) => {
                eyre::bail!(
                    "Unable to get release {} for {owner}/{repo}. {:?}",
                    version.as_tag(),
                    release_err
                )
            }
        };

        match github::get_platform_specific_asset(&release, system, &tool.asset_pattern) {
            Some(asset) => match env
                .add_tool(
                    &tool.name,
                    &tool.alias,
                    version,
                    asset,
                    &tool.asset_pattern,
                    &tool.file_pattern,
                )
                .await
            {
                Ok(_) => (),
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
