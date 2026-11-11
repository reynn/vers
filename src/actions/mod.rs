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
pub use remove::{remove_tool, RemoveOptions};
pub use sync::sync_tools;
pub use update::{update_tools, UpdateType};

use crate::{
    environment::{Environment, EnvironmentError},
    github::{self, GitHubError},
    system::{OperatingSystem, PlatformArchitecture, System},
    tool::Tool,
    validation::{parse_tool_name, validate_version, ValidationError},
    version::Version,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionsError {
    #[error("Failed to {operation} tools: {failures}")]
    ToolFailures {
        operation: &'static str,
        failures: String,
    },
    #[error("Failed to delete file '{file_name}'(symlink? {symlink}). {source}")]
    FileDelete {
        file_name: std::path::PathBuf,
        symlink: bool,
        source: std::io::Error,
    },
    #[error("Failed to delete directory '{directory}'. {source}")]
    DirectoryDelete {
        directory: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Tool '{tool_name}' not found in the '{env_name}' environment")]
    ToolNotFound { tool_name: String, env_name: String },
    #[error(
        "Version '{version}' of tool '{tool_name}' is not recorded in the '{env_name}' environment"
    )]
    VersionNotFound {
        tool_name: String,
        version: String,
        env_name: String,
    },
    #[error("Cannot combine --purge with --link-only")]
    PurgeLinkOnlyConflict,
    #[error("Cannot combine --all with an explicit version")]
    AllWithVersionConflict,
    #[error(
        "Refusing to modify '{path}': it exists and is not a symlink. Move it before removing this tool"
    )]
    AliasNotSymlink { path: std::path::PathBuf },
    #[error(
        "Cannot switch away from {tool_name}@{version}: no safe cached executable for remaining version '{fallback}'. {reason}. Use --all to detach the tool entirely"
    )]
    NoSafeFallback {
        tool_name: String,
        version: String,
        fallback: String,
        reason: String,
    },
    #[error("Cannot purge '{tool_name}' ({detail}): still referenced by '{env_file}'")]
    PurgeBlocked {
        tool_name: String,
        detail: String,
        env_file: String,
    },
    #[error("Cannot purge path '{path}': {reason}")]
    UnsafePurgePath {
        path: std::path::PathBuf,
        reason: String,
    },
    #[error("Failed to read environment config '{path}' while checking purge safety. {source}")]
    PurgeScanIo {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Malformed environment config '{path}' while checking purge safety: {msg}")]
    PurgeScanMalformed {
        path: std::path::PathBuf,
        msg: String,
    },
    #[error("Unable to find release for {0}")]
    ReleaseNotFound(String),
    #[error("Unable to find asset for {tool_name}@{version} for OS: {os}; Arch: {arch}")]
    AssetNotFound {
        tool_name: String,
        version: Version,
        arch: PlatformArchitecture,
        os: OperatingSystem,
    },
    #[error("GitHub API error: {0}")]
    GitHub(#[from] GitHubError),
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

type Result<T, E = ActionsError> = std::result::Result<T, E>;

async fn handle_tool_install(
    env: &mut Environment,
    tool: &'_ Tool,
    system: &'_ System,
    version: Option<Version>,
) -> Result<()> {
    let (owner, repo) = parse_tool_name(&tool.name)?;

    let version = match version {
        Some(v) => v,
        None => match github::get_latest_release_tag(owner, repo).await {
            Some(rel) => rel,
            None => return Err(ActionsError::ReleaseNotFound(tool.name.to_string())),
        },
    };
    validate_version(&version.as_tag())?;

    // Check the real environment entry, not the temporary Tool passed by add.
    // A matching version alone does not mean its binary or link still exists.
    if let Some(installed) = env.tools.iter().find(|installed| {
        installed.name == tool.name
            && installed.current_version == version.as_tag()
            && installed.alias == tool.alias
            && installed.asset_pattern == tool.asset_pattern
            && installed.file_pattern == tool.file_pattern
    }) {
        if env.repair_tool_link(installed)? {
            return Ok(());
        }
    }

    let release = github::get_specific_release_for_repo(owner, repo, &version).await?;
    let asset = github::get_platform_specific_asset(&release, system, &tool.asset_pattern)
        .ok_or_else(|| ActionsError::AssetNotFound {
            tool_name: tool.name.to_string(),
            version: version.clone(),
            arch: system.architecture.clone(),
            os: system.os.clone(),
        })?;
    // Persist the actual tag, including any v prefix, rather than "latest".
    let resolved_version = Version::Simple(release.tag_name);
    validate_version(&resolved_version.as_tag())?;
    env.add_tool(
        &tool.name,
        &tool.alias,
        resolved_version,
        asset,
        &tool.asset_pattern,
        &tool.file_pattern,
    )
    .await?;
    Ok(())
}
