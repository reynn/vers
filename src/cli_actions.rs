use crate::{
    cli, dirs,
    environment::{Environment, EnvironmentError},
    github,
    system::{self, System},
    tool::Tool,
    version::parse_version,
    version::Version,
};
use clap::CommandFactory;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use skim::prelude::*;
use std::{
    io::{self, Cursor},
    path::Path,
};
use tabled::{object::Segment, Alignment, Modify, Panel, Style, Table, Tabled};
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum CliActionError {
    #[error("Failed to delete file '{file_name}'(symlink? {symlink}). {source}")]
    FileDeleteError {
        file_name: std::path::PathBuf,
        symlink: bool,
        source: std::io::Error,
    },
    #[error("Failed to delete directory '{directory}. {source}")]
    DirectoryDeleteError {
        directory: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Tool '{tool_name}' not found in the '{env_name}' environment")]
    ToolNotFound { tool_name: String, env_name: String },
    #[error("The environment {0}, does not contain any tools")]
    EmptyEnvironment(String),
    #[error("Unable to find release for {0}")]
    ReleaseNotFound(String),
    #[error("Unable to find asset for {tool_name}@{version} for OS: {os}; Arch: {arch}")]
    AssetNotFoundError {
        tool_name: String,
        version: Version,
        arch: system::PlatformArchitecture,
        os: system::OperatingSystem,
    },
    #[error("...")]
    GitHubError(#[from] github::GitHubError),
    #[error("...")]
    EnvironmentError(#[from] EnvironmentError),
}

type Result<T, E = CliActionError> = std::result::Result<T, E>;

pub struct Patterns {
    pub asset: Option<String>,
    pub file: Option<String>,
}

pub async fn add_new_tool(
    env: &mut Environment,
    name: &'_ str,
    system: &'_ System,
    patterns: Patterns,
    alias: Option<String>,
    show: bool,
    pre_release: bool,
) -> Result<()> {
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

    let asset_pattern = &patterns.asset.unwrap_or_default();
    let file_pattern = &patterns.file.unwrap_or_else(|| alias.clone());

    info!("Owner `{owner}`, Repo `{repo}`, Alias `{alias}`, Pattern `{asset_pattern}`, Filter `{file_pattern}`");

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
            match versions.get(0) {
                Some(version) => vec![version.into()],
                None => vec![],
            }
            // vec![versions.get(0).unwrap().to_string()]
        }
    };

    for version in versions.iter() {
        let parsed_version = parse_version(version);

        let tool = Tool::new(
            org_repo,
            &alias,
            &Version::Latest,
            asset_pattern,
            file_pattern,
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
    remove_all_versions: bool,
) -> Result<()> {
    if let Some(env_tool) = env.tools.iter().find(|t| t.name == name) {
        info!("Removing {name} from environment. {}", &env.name);
        let env_path = Path::new(&env.base_dir);

        let link_path = dirs::get_tool_link_path(env_path, &env_tool.name);
        if link_path.exists() {
            debug!("Removing symlink {:?}", &link_path);
            if let Err(remove_err) = std::fs::remove_file(&link_path) {
                return Err(CliActionError::FileDeleteError {
                    file_name: link_path,
                    symlink: true,
                    source: remove_err,
                });
            };
        }

        if remove_all_versions {
            let tool_path = dirs::get_tool_download_dir(env_path, &env_tool.name);
            std::fs::remove_dir_all(&tool_path).map_err(|remove_dir_err| {
                CliActionError::DirectoryDeleteError {
                    directory: tool_path,
                    source: remove_dir_err,
                }
            })?;
        } else {
            let tool_path = dirs::get_tool_version_download_dir(
                env_path,
                &env_tool.name,
                &env_tool.current_version,
            );
            debug!("Removing tool directory {:?}", &tool_path);
            std::fs::remove_dir_all(&tool_path).map_err(|remove_dir_err| {
                CliActionError::DirectoryDeleteError {
                    directory: tool_path,
                    source: remove_dir_err,
                }
            })?;
        }

        let tool_idx = env.tools.iter().position(|t| t.name == name).unwrap();
        debug!("Found {} at index {}, removing...", name, tool_idx);
        env.tools.swap_remove(tool_idx);
        Ok(())
    } else {
        // anyhow::bail!("{} is not found in the {} environment.", name, env.name)
        Err(CliActionError::ToolNotFound {
            tool_name: name.to_string(),
            env_name: env.name.to_string(),
        })
    }
}

pub async fn list_tools(
    env: &'_ Environment,
    installed: bool,
    output_type: cli::ListOutputType,
) -> Result<()> {
    info!("Listing all tools available in {}", env.name);
    let tools = &env.tools;

    if tools.is_empty() {
        return Err(CliActionError::EmptyEnvironment(env.name.to_string()));
    }

    #[derive(Tabled, Serialize, PartialEq, PartialOrd, Eq, Ord)]
    struct ListTool<'a> {
        #[tabled(rename = "Name")]
        name: &'a str,
        #[tabled(rename = "Alias")]
        alias: &'a str,
        #[tabled(rename = "Version")]
        version: &'a str,
    }
    impl<'a> std::fmt::Display for ListTool<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}@{}", self.name, self.version)
        }
    }

    let mut l: Vec<ListTool> = tools
        .iter()
        .flat_map(|t| {
            if installed {
                t.installed_versions
                    .iter()
                    .map(|tt| ListTool {
                        name: &t.name,
                        alias: &t.alias,
                        version: tt,
                    })
                    .collect()
            } else {
                vec![ListTool {
                    name: &t.name,
                    alias: &t.alias,
                    version: &t.current_version,
                }]
            }
        })
        .collect();

    l.sort();
    match output_type {
        cli::ListOutputType::Table => {
            println!(
                "{}",
                Table::new(&l)
                    .with(Panel::header(if installed {
                        "All Installed Versions"
                    } else {
                        "Current Versions Only"
                    }))
                    .with(Panel::footer(format!("{} tools installed", l.len())))
                    .with(Modify::new(Segment::all()).with(Alignment::center()))
                    .with(Style::rounded())
            );
        }
        cli::ListOutputType::Text => l.iter().for_each(|t| println!("{}", t)),
        cli::ListOutputType::Json => {
            println!("{}", serde_json::to_string_pretty(&l).unwrap())
        }
    }

    Ok(())
}

pub enum UpdateType {
    All,
    Specific(String),
}

pub async fn update_tools(
    env: &mut Environment,
    system: &'_ System,
    update_type: UpdateType,
) -> Result<()> {
    match update_type {
        UpdateType::All => {
            let tools: Vec<Tool> = env.tools.to_vec();
            let progress_bar = ProgressBar::new(tools.len() as u64);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{bar:75.cyan/blue} {pos:>7}/{len:7} {msg}")
                    .unwrap(),
            );

            let mut failed_tools = Vec::new();
            for tool in tools {
                progress_bar.set_message(tool.name.clone());
                match handle_tool_install(env, &tool, system, None).await {
                    Ok(_) => info!("Tool {} complete.", &tool.name),
                    Err(install_err) => failed_tools.push(install_err.to_string()),
                }
                progress_bar.inc(1);
            }
            error!("{}", failed_tools.join("\n"));

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

pub async fn sync_tools(env: &mut Environment, system: &'_ System) -> Result<()> {
    let tools: Vec<Tool> = env.tools.to_vec();
    let progress_bar = ProgressBar::new(tools.len() as u64);

    for tool in tools.iter() {
        let parsed_version = parse_version(&tool.current_version);
        match handle_tool_install(env, tool, system, Some(parsed_version)).await {
            Ok(_) => info!(
                "Tool {} has been installed at version {}",
                &tool.name, tool.current_version
            ),
            Err(install_err) => error!("Failed to install {}. {:?}", &tool.name, install_err),
        }
        progress_bar.inc(1);
    }
    Ok(())
}

pub fn generate_completions(shell: clap_complete::Shell) {
    let mut cmd = cli::Opts::command();
    let cmd_name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, cmd_name, &mut io::stdout())
}

async fn handle_tool_install(
    env: &mut Environment,
    tool: &'_ Tool,
    system: &'_ System,
    version: Option<Version>,
) -> Result<()> {
    let split_org_repo: Vec<&str> = tool.name.split('/').collect();
    let owner = split_org_repo[0];
    let repo = split_org_repo[1];

    let version = match version {
        Some(v) => v,
        None => match github::get_latest_release_tag(owner, repo).await {
            Some(rel) => rel,
            None => return Err(CliActionError::ReleaseNotFound(tool.name.to_string())),
        },
    };

    if tool.current_version != version.as_tag() {
        let release = github::get_specific_release_for_repo(owner, repo, &version).await?;

        let asset = github::get_platform_specific_asset(&release, system, &tool.asset_pattern);
        if asset.is_none() {
            return Err(CliActionError::AssetNotFoundError {
                tool_name: tool.name.to_string(),
                version,
                arch: system.architecture.clone(),
                os: system.os.clone(),
            });
        }
        let asset = asset.unwrap();
        match env
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
            Err(add_tool_err) => return Err(add_tool_err.into()),
        };
    };
    Ok(())
}
