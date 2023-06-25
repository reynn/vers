mod add;
mod completions;
mod env;
mod list;
mod remove;
mod sync;
mod update;

pub use add::{add_new_tool, Patterns};
pub use completions::generate_completions;
pub use env::show_env_config;
pub use list::list_tools;
pub use remove::remove_tool;
pub use sync::sync_tools;
pub use update::{update_tools, UpdateType};

use crate::{
    environment::{Environment, EnvironmentError},
    github::{self, GitHubError},
    system::{OperatingSystem, PlatformArchitecture, System},
    tool::Tool,
    version::Version,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionsError {
    #[error("Failed to delete file '{file_name}'(symlink? {symlink}). {source}")]
    FileDelete {
        file_name: std::path::PathBuf,
        symlink: bool,
        source: std::io::Error,
    },
    #[error("Failed to delete directory '{directory}. {source}")]
    DirectoryDelete {
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
    AssetNotFound {
        tool_name: String,
        version: Version,
        arch: PlatformArchitecture,
        os: OperatingSystem,
    },
    #[error("Error with the GitHub API {0}")]
    GitHub(#[from] GitHubError),
    #[error("Environment error {0}")]
    Environment(#[from] EnvironmentError),
}

type Result<T, E = ActionsError> = std::result::Result<T, E>;

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
            None => return Err(ActionsError::ReleaseNotFound(tool.name.to_string())),
        },
    };

    if tool.current_version != version.as_tag() {
        let release = github::get_specific_release_for_repo(owner, repo, &version).await?;

        let asset = github::get_platform_specific_asset(&release, system, &tool.asset_pattern);
        if asset.is_none() {
            return Err(ActionsError::AssetNotFound {
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
