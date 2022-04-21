mod global;
mod local;

use {
    crate::{download, shell, tool::Tool, version::Version},
    directories_next::ProjectDirs,
    log::*,
    octocrab::models::repos::Asset,
    serde::{Deserialize, Serialize},
    serde_json::{from_str, to_string_pretty},
    std::{
        path::{Path, PathBuf},
        process::exit,
    },
    tokio::{fs::read_to_string, task::futures},
};

pub trait Env {
    fn add_tool(&self, name: &'_ str, version: &'_ Version, asset: &'_ Asset) -> crate::Result<()>;
    fn remove_tool(&self, name: &'_ str) -> crate::Result<()>;
    fn change_tool_version(&self, name: &'_ str, new_version: &'_ Version) -> crate::Result<()>;
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(skip)]
    pub base_dir: String,
    pub tools: Vec<Tool>,
}

impl Drop for Environment {
    fn drop(&mut self) {
        let out_file = Path::new(&self.base_dir).parent().unwrap();
        let out_file = out_file.join(format!("{}.json", &self.name));
        debug!("Writing {} environment file to {:?}", &self.name, out_file);
        std::fs::write(out_file, to_string_pretty(&self).unwrap()).unwrap();
    }
}

impl Environment {
    pub async fn load<P: Into<PathBuf>>(config_dir: P, name: &'_ str) -> super::Result<Self> {
        let config_dir: PathBuf = config_dir.into();
        let env_dir = config_dir.join("envs");
        if !env_dir.exists() {
            match std::fs::create_dir_all(&env_dir) {
                Ok(_) => {}
                Err(create_dir_err) => {
                    return Err(
                        format!("Unable to create the envs directory: {}", create_dir_err).into(),
                    )
                }
            }
        }
        let env_path = env_dir.join(format!("{}.json", name));
        match read_to_string(&env_path).await {
            Ok(file_contents) => match from_str::<Self>(&file_contents) {
                Ok(mut res) => {
                    res.base_dir = env_dir
                        .join(name)
                        .to_str()
                        .expect("Unable to convert path to a string")
                        .to_string();
                    Ok(res)
                }
                Err(serde_err) => Err(format!("{:?}", serde_err).into()),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!("Environment file does not exist");
                    Environment {
                        name: name.to_string(),
                        base_dir: env_dir.to_str().unwrap_or_default().to_string(),
                        tools: Vec::new(),
                    }
                    .save()
                    .await
                }
                _ => Err(format!(
                    "Failed to read contents from file {:?}. {}",
                    env_path, read_err
                )
                .into()),
            },
        }
    }

    pub async fn save(self) -> crate::Result<Self> {
        let out_file = Path::new(&self.base_dir)
            .parent()
            .unwrap()
            .join(format!("{}.json", &self.name));
        debug!("Writing {} environment file to {:?}", &self.name, out_file);
        tokio::fs::write(out_file, to_string_pretty(&self)?).await?;
        Ok(self)
    }

    pub async fn add_tool(
        &mut self,
        name: &'_ str,
        version: Version,
        asset: Asset,
    ) -> crate::Result<()> {
        let tool_dir = Path::new(&self.base_dir)
            .parent()
            .unwrap()
            .join("tools")
            .join(name);
        let version_tag = version.as_tag();
        let tool_version_dir = tool_dir.clone().join(&version_tag);
        info!(
            "Tag: {} | tool_version_dir: {:?}",
            version_tag, &tool_version_dir
        );
        match download::download_asset(&asset, tool_version_dir).await {
            Ok(asset_path) => {
                log::info!(
                    "Download of asset {} completed.",
                    asset.browser_download_url
                );
                // extract file
                // add to the tools list
                if let Some(installed_tool) = self.tools.iter_mut().find(|t| t.name == name) {
                    installed_tool.set_current_version(version)
                }
            }
            Err(download_err) => log::error!(
                "Failed to download file {} from {}",
                asset.name,
                asset.browser_download_url
            ),
        }
        Ok(())
    }
}
