use {
    crate::{
        environment::Environment, manager::Manager, system::System, tool::Tool, version,
        version::parse_version, version::Version,
    },
    log::*,
    skim::prelude::*,
    std::io::Cursor,
};

pub struct Patterns {
    pub asset: Option<String>,
    pub file: Option<String>,
}

pub async fn add_new_tool(
    env: &mut Environment,
    name: &'_ str,
    version: Option<Version>,
    system: &'_ System,
    patterns: &'_ Patterns,
    alias: Option<String>,
    show: bool,
    manager: Arc<impl Manager>,
) -> super::Result<()> {
    let alias = alias.unwrap_or_else(
        || match name.clone().split('/').collect::<Vec<_>>().get(0) {
            Some(n) => n.to_string(),
            None => "".to_string(),
        },
    );
    let user_pattern = patterns.asset.clone().unwrap_or_else(|| "".to_string());
    let file_pattern = patterns.file.clone().unwrap_or_else(|| alias.clone());
    debug!("Name `{name}`, Alias `{alias}`, Pattern `{user_pattern}`, Filter `{file_pattern}`");

    let versions: Vec<Version> = if let Some(v) = version {
        vec![v]
    } else {
        match manager.get_all_versions(name).await {
            Ok(versions) => {
                // if the user wants a list of the releases show that, otherwise just get the first result
                if show {
                    let version_list: Vec<String> = versions.iter().map(|v| v.as_tag()).collect();
                    let item_reader =
                        SkimItemReader::default().of_bufread(Cursor::new(version_list.join("\n")));
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
                            .map(|item| version::parse_version(item.text().to_string().as_str()))
                            .collect()
                    })
                    .unwrap_or_default()
                } else {
                    vec![versions[0].clone()]
                }
            }
            Err(v_err) => {
                eyre::bail!("Failed to get versions for {name}. {:?}", v_err)
            }
        }
    };

    // let manager = Arc::clone(&manager);
    for version in versions.iter() {
        let tool = Tool::new(name, &alias, &Version::Latest, &user_pattern, &file_pattern);

        match handle_tool_install(env, &tool, system, Some(version.clone()), manager.clone()).await
        {
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
    manager: Arc<impl Manager>,
) -> crate::Result<()> {
    match update_type {
        UpdateType::All => {
            let tools: Vec<Tool> = env.tools.to_vec();
            println!("-> Updating all tools...");
            for tool in tools {
                match handle_tool_install(env, &tool, system, None, manager.clone()).await {
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

                match handle_tool_install(env, tool, system, version, manager.clone()).await {
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

pub async fn sync_tools(
    env: &mut Environment,
    system: &'_ System,
    manager: Arc<impl Manager>,
) -> crate::Result<()> {
    let tools: Vec<Tool> = env.tools.to_vec();

    for tool in tools.iter() {
        let parsed_version = parse_version(&tool.current_version);
        match handle_tool_install(env, tool, system, Some(parsed_version), manager.clone()).await {
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
    manager: Arc<impl Manager>,
) -> crate::Result<()> {
    let version = if let Some(version) = version {
        info!("Using provided version {}", version.as_tag());
        version
    } else {
        info!("Getting version from release tags");
        manager.get_latest_version(&tool.name).await?
    };

    info!(
        "Comparing `{}` to `{}`",
        tool.current_version,
        version.as_tag()
    );
    if tool.current_version != version.as_tag() {
        println!("--> Installing tool {}@{}", &tool.name, version.as_tag());
        match manager
            .get_assets_for_version(&tool.name, &version, system)
            .await
        {
            Ok(asset) => {
                match env
                    .add_tool(
                        &tool.name,
                        &tool.alias,
                        version,
                        &asset[0],
                        &tool.asset_pattern,
                        &tool.file_pattern,
                    )
                    .await
                {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(assets_err) => {
                error!(
                    "Failed to get assets for system for {}. Error: {:?}",
                    tool.name, assets_err
                )
            }
        };
    }

    Ok(())
}
