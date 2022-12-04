use crate::{dirs, environment::Environment};
use std::path::Path;
use tracing::{debug, info};

pub async fn remove_tool(
    env: &mut Environment,
    name: &'_ str,
    remove_all_versions: bool,
) -> super::Result<()> {
    if let Some(env_tool) = env.tools.iter().find(|t| t.name == name) {
        info!("Removing {name} from environment. {}", &env.name);
        let env_path = Path::new(&env.base_dir);

        let link_path = dirs::get_tool_link_path(env_path, &env_tool.name);
        if link_path.exists() {
            debug!("Removing symlink {:?}", &link_path);
            if let Err(remove_err) = std::fs::remove_file(&link_path) {
                return Err(super::ActionsError::FileDelete {
                    file_name: link_path,
                    symlink: true,
                    source: remove_err,
                });
            };
        }

        if remove_all_versions {
            let tool_path = dirs::get_tool_download_dir(env_path, &env_tool.name);
            std::fs::remove_dir_all(&tool_path).map_err(|remove_dir_err| {
                super::ActionsError::DirectoryDelete {
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
                super::ActionsError::DirectoryDelete {
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
        Err(super::ActionsError::ToolNotFound {
            tool_name: name.to_string(),
            env_name: env.name.to_string(),
        })
    }
}
