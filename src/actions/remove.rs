use crate::{
    dirs,
    environment::Environment,
    tool::Tool,
    validation::{parse_tool_name, validate_alias, validate_tool_name, validate_version},
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};
use tracing::{debug, info};

/// Options controlling how a tool is removed from an environment.
#[derive(Debug, Clone, Copy, Default)]
pub struct RemoveOptions {
    /// Forget every recorded version of the tool in this environment.
    pub all: bool,
    /// Synonym of default non-purge removal: update environment records without
    /// deleting shared downloads. Kept as an explicit flag for CLI compatibility
    /// and rejected together with [`RemoveOptions::purge`].
    pub link_only: bool,
    /// Delete selected downloads from the shared cache after environment records
    /// no longer reference them.
    pub purge: bool,
}

pub async fn remove_tool(
    env: &mut Environment,
    name: &'_ str,
    options: RemoveOptions,
) -> super::Result<()> {
    let (tool_name, explicit_version) = parse_remove_spec(name)?;

    if options.purge && options.link_only {
        return Err(super::ActionsError::PurgeLinkOnlyConflict);
    }
    if options.all && explicit_version.is_some() {
        return Err(super::ActionsError::AllWithVersionConflict);
    }

    let matching_indices: Vec<usize> = env
        .tools
        .iter()
        .enumerate()
        .filter(|(_, tool)| tool.name == tool_name)
        .map(|(idx, _)| idx)
        .collect();
    if matching_indices.is_empty() {
        return Err(super::ActionsError::ToolNotFound {
            tool_name: tool_name.to_string(),
            env_name: env.name.clone(),
        });
    }

    // Operate on every duplicate entry for this owner/repo so purge reference
    // checks and record updates stay consistent within the current environment.
    let mut plans = Vec::with_capacity(matching_indices.len());
    for &idx in &matching_indices {
        plans.push(plan_removal(
            env,
            idx,
            explicit_version.as_deref(),
            options,
        )?);
    }

    // All preflights complete before any link, metadata, or purge side effects.
    for plan in &mut plans {
        preflight_removal(env, plan)?;
    }
    if options.purge {
        let mut purge_versions = BTreeSet::new();
        for plan in &plans {
            for version in &plan.versions_to_remove {
                purge_versions.insert(version.clone());
            }
        }
        preflight_purge(
            env,
            &tool_name,
            options.all,
            if options.all {
                None
            } else {
                Some(purge_versions)
            },
            &plans,
        )?;
    }

    for plan in &plans {
        apply_removal(env, plan)?;
    }

    if options.purge {
        let mut purge_versions = BTreeSet::new();
        for plan in &plans {
            for version in &plan.versions_to_remove {
                purge_versions.insert(version.clone());
            }
        }
        perform_purge(
            env,
            &tool_name,
            options.all,
            if options.all {
                None
            } else {
                Some(purge_versions)
            },
        )?;
    }

    Ok(())
}

#[derive(Debug)]
struct RemovalPlan {
    tool_index: usize,
    tool_name: String,
    alias: String,
    versions_to_remove: Vec<String>,
    /// When set, switch the active link to this remaining version before
    /// forgetting the removed versions.
    fallback_version: Option<String>,
    /// Pre-resolved executable for [`Self::fallback_version`].
    fallback_binary: Option<PathBuf>,
    remove_link: bool,
    remove_entry: bool,
    touching_link: bool,
}

fn parse_remove_spec(name: &str) -> super::Result<(String, Option<String>)> {
    let (tool_name, version) = match name.split_once('@') {
        Some((tool_name, version)) => (tool_name, Some(version)),
        None => (name, None),
    };
    // Keep the tag exactly as provided; do not strip a leading `v`.
    parse_tool_name(tool_name)?;
    if let Some(version) = version {
        validate_version(version)?;
        Ok((tool_name.to_string(), Some(version.to_string())))
    } else {
        Ok((tool_name.to_string(), None))
    }
}

fn plan_removal(
    env: &Environment,
    tool_index: usize,
    explicit_version: Option<&str>,
    options: RemoveOptions,
) -> super::Result<RemovalPlan> {
    let tool = &env.tools[tool_index];
    validate_tool_name(&tool.name)?;
    validate_alias(&tool.alias)?;
    validate_version(&tool.current_version)?;
    for version in &tool.installed_versions {
        validate_version(version)?;
    }

    let recorded_versions = recorded_versions(tool);
    let versions_to_remove = if options.all {
        recorded_versions.clone()
    } else {
        let target = match explicit_version {
            Some(version) => version.to_string(),
            None => tool.current_version.clone(),
        };
        if !recorded_versions.iter().any(|version| version == &target) {
            return Err(super::ActionsError::VersionNotFound {
                tool_name: tool.name.clone(),
                version: target,
                env_name: env.name.clone(),
            });
        }
        vec![target]
    };

    let removing_active = versions_to_remove
        .iter()
        .any(|version| version == &tool.current_version);
    let remaining: Vec<String> = recorded_versions
        .into_iter()
        .filter(|version| !versions_to_remove.iter().any(|remove| remove == version))
        .collect();

    let (fallback_version, remove_link, remove_entry, touching_link) =
        if options.all || remaining.is_empty() {
            (None, true, true, true)
        } else if removing_active {
            // Select the last remaining recorded version as the new active version.
            let fallback = remaining[remaining.len() - 1].clone();
            (Some(fallback), false, false, true)
        } else {
            // Inactive version removal leaves the active link unchanged.
            (None, false, false, false)
        };

    Ok(RemovalPlan {
        tool_index,
        tool_name: tool.name.clone(),
        alias: tool.alias.clone(),
        versions_to_remove,
        fallback_version,
        fallback_binary: None,
        remove_link,
        remove_entry,
        touching_link,
    })
}

fn recorded_versions(tool: &Tool) -> Vec<String> {
    let mut versions = Vec::new();
    for version in &tool.installed_versions {
        if !versions.iter().any(|existing| existing == version) {
            versions.push(version.clone());
        }
    }
    if !tool.current_version.is_empty()
        && !versions
            .iter()
            .any(|existing| existing == &tool.current_version)
    {
        versions.push(tool.current_version.clone());
    }
    versions
}

fn preflight_removal(env: &Environment, plan: &mut RemovalPlan) -> super::Result<()> {
    let env_path = Path::new(&env.base_dir);
    let link_path = dirs::get_tool_link_path(env_path, &plan.alias);

    if plan.touching_link {
        assert_alias_link_removable(&link_path)?;
    }

    if let Some(fallback) = &plan.fallback_version {
        let tool = &env.tools[plan.tool_index];
        let fallback_dir = dirs::get_tool_version_download_dir(env_path, &tool.name, fallback);
        assert_safe_managed_delete_path(&tools_root(env_path)?, &fallback_dir).map_err(
            |error| super::ActionsError::NoSafeFallback {
                tool_name: plan.tool_name.clone(),
                version: tool.current_version.clone(),
                fallback: fallback.clone(),
                reason: error.to_string(),
            },
        )?;
        match env.resolve_cached_binary(tool, fallback) {
            Ok(binary) => {
                plan.fallback_binary = Some(binary);
            }
            Err(error) => {
                return Err(super::ActionsError::NoSafeFallback {
                    tool_name: plan.tool_name.clone(),
                    version: tool.current_version.clone(),
                    fallback: fallback.clone(),
                    reason: error.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn assert_alias_link_removable(link_path: &Path) -> super::Result<()> {
    match fs::symlink_metadata(link_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(super::ActionsError::AliasNotSymlink {
            path: link_path.to_path_buf(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(super::ActionsError::FileDelete {
            file_name: link_path.to_path_buf(),
            symlink: false,
            source: error,
        }),
    }
}

fn remove_alias_link(link_path: &Path) -> super::Result<()> {
    match fs::symlink_metadata(link_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            debug!("Removing symlink {:?}", link_path);
            fs::remove_file(link_path).map_err(|source| super::ActionsError::FileDelete {
                file_name: link_path.to_path_buf(),
                symlink: true,
                source,
            })
        }
        Ok(_) => Err(super::ActionsError::AliasNotSymlink {
            path: link_path.to_path_buf(),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(super::ActionsError::FileDelete {
            file_name: link_path.to_path_buf(),
            symlink: false,
            source: error,
        }),
    }
}

fn apply_removal(env: &mut Environment, plan: &RemovalPlan) -> super::Result<()> {
    let env_path = Path::new(&env.base_dir);
    let link_path = dirs::get_tool_link_path(env_path, &plan.alias);

    // Always re-resolve the entry by name+alias so earlier swap_removes from
    // duplicate-tool plans cannot leave a stale index.
    let tool_index = env
        .tools
        .iter()
        .position(|tool| tool.name == plan.tool_name && tool.alias == plan.alias);

    if let (Some(fallback), Some(binary)) = (&plan.fallback_version, &plan.fallback_binary) {
        let Some(tool_index) = tool_index else {
            return Ok(());
        };
        let tool = env.tools[tool_index].clone();
        let relative = env
            .activate_cached_binary(&tool, fallback, binary)
            .map_err(super::ActionsError::from)?;
        let tool = &mut env.tools[tool_index];
        tool.current_version = fallback.clone();
        tool.binary_path = Some(relative);
        tool.installed_versions.retain(|version| {
            !plan
                .versions_to_remove
                .iter()
                .any(|remove| remove == version)
        });
        if !tool
            .installed_versions
            .iter()
            .any(|version| version == fallback)
        {
            tool.installed_versions.push(fallback.clone());
        }
        info!(
            "Switched {} to remaining version {} after removal",
            tool.name, fallback
        );
        return Ok(());
    }

    if plan.remove_link {
        remove_alias_link(&link_path)?;
    }

    if plan.remove_entry {
        info!(
            "Removing {} from environment '{}'",
            plan.tool_name, env.name
        );
        if let Some(idx) = tool_index {
            env.tools.swap_remove(idx);
        }
        return Ok(());
    }

    if let Some(tool_index) = tool_index {
        let tool = &mut env.tools[tool_index];
        tool.installed_versions.retain(|version| {
            !plan
                .versions_to_remove
                .iter()
                .any(|remove| remove == version)
        });
        info!(
            "Forgot version(s) [{}] of {} from environment '{}'",
            plan.versions_to_remove.join(", "),
            plan.tool_name,
            env.name
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct EnvReferenceFile {
    // Required: missing/misspelled `tools` must fail closed as malformed config.
    tools: Vec<ToolReference>,
}

#[derive(Debug, Deserialize)]
struct ToolReference {
    name: String,
    #[serde(default)]
    current_version: String,
    #[serde(default)]
    installed_versions: Vec<String>,
}

fn preflight_purge(
    env: &Environment,
    tool_name: &str,
    purge_all: bool,
    versions: Option<BTreeSet<String>>,
    plans: &[RemovalPlan],
) -> super::Result<()> {
    let env_path = Path::new(&env.base_dir);
    let tools_root = tools_root(env_path)?;
    if purge_all {
        let target = dirs::get_tool_download_dir(env_path, tool_name);
        assert_safe_managed_delete_path(&tools_root, &target)?;
    } else if let Some(ref versions) = versions {
        for version in versions {
            let path = dirs::get_tool_version_download_dir(env_path, tool_name, version);
            assert_safe_managed_delete_path(&tools_root, &path)?;
        }
    }

    // Duplicate entries in the current environment that still reference a
    // targeted version after this removal must block purge.
    for (idx, tool) in env.tools.iter().enumerate() {
        if tool.name != tool_name {
            continue;
        }
        let planned = plans.iter().find(|plan| plan.tool_index == idx);
        let remaining_refs = match planned {
            Some(plan) if plan.remove_entry => Vec::new(),
            Some(plan) => recorded_versions(tool)
                .into_iter()
                .filter(|version| {
                    !plan
                        .versions_to_remove
                        .iter()
                        .any(|remove| remove == version)
                })
                .collect::<Vec<_>>(),
            None => recorded_versions(tool),
        };
        if purge_all && !remaining_refs.is_empty() {
            return Err(super::ActionsError::PurgeBlocked {
                tool_name: tool_name.to_string(),
                detail: "repository still referenced after removal".to_string(),
                env_file: format!("{}.json", env.config_key()),
            });
        }
        if let Some(ref versions) = versions {
            for version in &remaining_refs {
                if versions.contains(version) {
                    return Err(super::ActionsError::PurgeBlocked {
                        tool_name: tool_name.to_string(),
                        detail: format!("version '{version}' still referenced after removal"),
                        env_file: format!("{}.json", env.config_key()),
                    });
                }
            }
        }
    }

    let env_dir = env_path
        .parent()
        .ok_or_else(|| super::ActionsError::UnsafePurgePath {
            path: env_path.to_path_buf(),
            reason: "unable to determine environments directory".to_string(),
        })?;
    let current_file = env_dir.join(format!("{}.json", env.config_key()));

    let entries = match fs::read_dir(env_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(super::ActionsError::PurgeScanIo {
                path: env_dir.to_path_buf(),
                source: error,
            });
        }
    };

    for entry in entries {
        let entry = entry.map_err(|source| super::ActionsError::PurgeScanIo {
            path: env_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = match path.file_name().map(|name| name.to_string_lossy()) {
            Some(name) => name.into_owned(),
            None => continue,
        };
        // Environment names may start with '.'; only require a `.json` suffix.
        // Temp files use `.name.json.tmp.<pid>` and therefore do not match.
        if !file_name.ends_with(".json") {
            continue;
        }
        if path == current_file {
            continue;
        }

        let contents =
            fs::read_to_string(&path).map_err(|source| super::ActionsError::PurgeScanIo {
                path: path.clone(),
                source,
            })?;
        let parsed: EnvReferenceFile = serde_json::from_str(&contents).map_err(|error| {
            super::ActionsError::PurgeScanMalformed {
                path: path.clone(),
                msg: error.to_string(),
            }
        })?;

        for tool in &parsed.tools {
            if tool.name != tool_name {
                continue;
            }

            // --all: any same-repo record blocks, even without version fields.
            if purge_all {
                return Err(super::ActionsError::PurgeBlocked {
                    tool_name: tool_name.to_string(),
                    detail: "repository referenced by another environment".to_string(),
                    env_file: file_name.clone(),
                });
            }

            let mut refs = BTreeSet::new();
            if !tool.current_version.is_empty() {
                validate_version(&tool.current_version).map_err(|_| {
                    super::ActionsError::PurgeScanMalformed {
                        path: path.clone(),
                        msg: format!(
                            "invalid current_version '{}' for tool '{tool_name}'",
                            tool.current_version
                        ),
                    }
                })?;
                refs.insert(tool.current_version.clone());
            }
            for version in &tool.installed_versions {
                if version.is_empty() {
                    continue;
                }
                validate_version(version).map_err(|_| super::ActionsError::PurgeScanMalformed {
                    path: path.clone(),
                    msg: format!("invalid installed version '{version}' for tool '{tool_name}'"),
                })?;
                refs.insert(version.clone());
            }

            // Matching tool with no verifiable version refs cannot be treated as
            // unreferenced; fail closed so a corrupt entry cannot authorize purge.
            if refs.is_empty() {
                return Err(super::ActionsError::PurgeBlocked {
                    tool_name: tool_name.to_string(),
                    detail: "tool record has no verifiable version references".to_string(),
                    env_file: file_name.clone(),
                });
            }

            if let Some(ref versions) = versions {
                for version in versions {
                    if refs.contains(version) {
                        return Err(super::ActionsError::PurgeBlocked {
                            tool_name: tool_name.to_string(),
                            detail: format!(
                                "version '{version}' referenced by another environment"
                            ),
                            env_file: file_name.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

fn perform_purge(
    env: &Environment,
    tool_name: &str,
    purge_all: bool,
    versions: Option<BTreeSet<String>>,
) -> super::Result<()> {
    let env_path = Path::new(&env.base_dir);
    if purge_all {
        let path = dirs::get_tool_download_dir(env_path, tool_name);
        delete_cache_path(&path)?;
        if let Some(owner_dir) = path.parent() {
            let _ = fs::remove_dir(owner_dir);
        }
        return Ok(());
    }

    if let Some(versions) = versions {
        for version in versions {
            let path = dirs::get_tool_version_download_dir(env_path, tool_name, &version);
            delete_cache_path(&path)?;
        }
        let repo_dir = dirs::get_tool_download_dir(env_path, tool_name);
        let _ = fs::remove_dir(&repo_dir);
        if let Some(owner_dir) = repo_dir.parent() {
            let _ = fs::remove_dir(owner_dir);
        }
    }
    Ok(())
}

fn delete_cache_path(path: &Path) -> super::Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(super::ActionsError::DirectoryDelete {
            directory: path.to_path_buf(),
            source: error,
        }),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(super::ActionsError::UnsafePurgePath {
                path: path.to_path_buf(),
                reason: "cache path is a symlink".to_string(),
            })
        }
        Ok(metadata) if metadata.is_dir() => {
            fs::remove_dir_all(path).map_err(|source| super::ActionsError::DirectoryDelete {
                directory: path.to_path_buf(),
                source,
            })
        }
        Ok(_) => fs::remove_file(path).map_err(|source| super::ActionsError::FileDelete {
            file_name: path.to_path_buf(),
            symlink: false,
            source,
        }),
    }
}

fn tools_root(env_base: &Path) -> super::Result<PathBuf> {
    env_base
        .parent()
        .and_then(|envs| envs.parent())
        .map(|root| root.join("tools"))
        .ok_or_else(|| super::ActionsError::UnsafePurgePath {
            path: env_base.to_path_buf(),
            reason: "unable to determine shared tools root".to_string(),
        })
}

fn assert_safe_managed_delete_path(tools_root: &Path, target: &Path) -> super::Result<()> {
    let relative = match target.strip_prefix(tools_root) {
        Ok(relative) => relative,
        Err(_) => {
            return Err(super::ActionsError::UnsafePurgePath {
                path: target.to_path_buf(),
                reason: "path is outside the managed tools store".to_string(),
            });
        }
    };

    if relative.as_os_str().is_empty()
        || !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(super::ActionsError::UnsafePurgePath {
            path: target.to_path_buf(),
            reason: "path components are not confined to the managed tools store".to_string(),
        });
    }

    walk_components_reject_symlinks(tools_root, target)?;

    if target.exists() {
        let canon_root = fs::canonicalize(tools_root).map_err(|source| {
            super::ActionsError::UnsafePurgePath {
                path: tools_root.to_path_buf(),
                reason: format!("unable to canonicalize tools root: {source}"),
            }
        })?;
        let canon_target =
            fs::canonicalize(target).map_err(|source| super::ActionsError::UnsafePurgePath {
                path: target.to_path_buf(),
                reason: format!("unable to canonicalize purge target: {source}"),
            })?;
        if !canon_target.starts_with(&canon_root) {
            return Err(super::ActionsError::UnsafePurgePath {
                path: target.to_path_buf(),
                reason: "canonical path escapes the managed tools store".to_string(),
            });
        }
    }

    Ok(())
}

fn walk_components_reject_symlinks(tools_root: &Path, target: &Path) -> super::Result<()> {
    match fs::symlink_metadata(tools_root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(super::ActionsError::UnsafePurgePath {
                path: tools_root.to_path_buf(),
                reason: "tools root is a symlink".to_string(),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(super::ActionsError::UnsafePurgePath {
                path: tools_root.to_path_buf(),
                reason: format!("unable to inspect tools root: {error}"),
            });
        }
    }

    let relative = match target.strip_prefix(tools_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => {
            return Err(super::ActionsError::UnsafePurgePath {
                path: target.to_path_buf(),
                reason: "path is outside the managed tools store".to_string(),
            });
        }
    };

    let mut current = tools_root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(super::ActionsError::UnsafePurgePath {
                path: target.to_path_buf(),
                reason: "path components are not confined to the managed tools store".to_string(),
            });
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(super::ActionsError::UnsafePurgePath {
                    path: current,
                    reason: "managed cache path contains a symlink ancestor".to_string(),
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(super::ActionsError::UnsafePurgePath {
                    path: current,
                    reason: format!("unable to inspect cache path: {error}"),
                });
            }
        }
    }
    Ok(())
}
