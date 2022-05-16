use {
    crate::{archiver, download, tool::Tool, version::Version},
    directories_next::ProjectDirs,
    log::*,
    octocrab::models::repos::Asset,
    serde::{Deserialize, Serialize},
    serde_json::{from_str, to_string_pretty},
    std::{
        fs::FileType,
        os::unix::prelude::PermissionsExt,
        path::{Path, PathBuf},
        process::exit,
    },
    tokio::{fs::read_to_string, process::Command, task::futures},
    walkdir::{DirEntry, WalkDir},
};

pub trait Env {
    fn add_tool(&self, name: &'_ str, version: &'_ Version, asset: &'_ Asset) -> crate::Result<()>;
    fn remove_tool(&self, name: &'_ str) -> crate::Result<()>;
    fn change_tool_version(&self, name: &'_ str, new_version: &'_ Version) -> crate::Result<()>;
}

#[derive(Debug, Default, Serialize, Deserialize)]
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
        info!("Writing {} environment file to {:?}", &self.name, out_file);
        let contents = to_string_pretty(&self).unwrap();
        std::fs::write(out_file, contents).unwrap();
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
                    eyre::bail!("Unable to create the envs directory: {}", create_dir_err)
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
                Err(serde_err) => eyre::bail!(
                    "Failed to deserialize {:?} as an environment. {:?}",
                    env_path,
                    serde_err
                ),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!("Environment file does not exist");
                    Ok(Environment {
                        name: name.to_string(),
                        base_dir: env_dir.join(name).to_str().unwrap_or_default().to_string(),
                        tools: Vec::new(),
                    })
                }
                _ => eyre::bail!(
                    "Failed to read contents from file {:?}. {}",
                    env_path,
                    read_err
                ),
            },
        }
    }

    pub async fn add_tool(
        &mut self,
        name: &'_ str,
        alias: &'_ str,
        version: Version,
        asset: Asset,
        user_pattern: &'_ str,
        file_pattern: &'_ str,
    ) -> crate::Result<()> {
        let tool_dir = Path::new(&self.base_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tools")
            .join(name);
        let version_tag = version.as_tag();
        let tool_version_dir = tool_dir.clone().join(&version_tag);

        match download::download_asset(&asset, &tool_version_dir).await {
            Ok(asset_path) => {
                info!("Completed download: {}", asset.browser_download_url);
                let symlink_dest = Path::new(&self.base_dir).join(alias);
                let extractor = archiver::determine_extractor(&asset_path);

                match extractor {
                    Some(extractor) => {
                        match archiver::handle_file_extraction(
                            extractor,
                            &asset_path,
                            Some(tool_version_dir.clone()),
                        )
                        .await
                        {
                            Ok(_) => {
                                info!(
                                    "Extracted file {}",
                                    &asset_path.to_str().unwrap_or_default()
                                );

                                if let Some(bin_file) = find_binary(
                                    &tool_version_dir,
                                    if !file_pattern.is_empty() {
                                        file_pattern
                                    } else {
                                        alias
                                    },
                                ) {
                                    create_symlink(&bin_file.into_path(), &symlink_dest);
                                    match self.tools.iter_mut().find(|t| t.name == name) {
                                        // add to the tools list
                                        Some(installed_tool) => {
                                            installed_tool.set_current_version(&version);
                                            installed_tool.installed_versions.push(version.as_tag());
                                            info!(
                                                "Added new version {} of {} in environment {}",
                                                version.as_tag(),
                                                name,
                                                self.name
                                            );
                                            Ok(())
                                        }
                                        // create a new tool, and add to our list
                                        None => {
                                            self.tools.push(Tool::new(
                                                name,
                                                alias,
                                                &version,
                                                user_pattern,
                                                file_pattern,
                                            ));
                                            info!(
                                                "Added new tool {} in environment {}",
                                                name, self.name
                                            );
                                            Ok(())
                                        }
                                    }
                                } else {
                                    eyre::bail!(
                                        "Could not find a binary named '{}' in {:?}",
                                        alias,
                                        tool_version_dir
                                    )
                                }
                            }
                            Err(extractor_err) => eyre::bail!(
                                "Failed to extract the file {:?}. {:?}",
                                asset_path,
                                extractor_err
                            ),
                        }
                    }
                    None => {
                        let cmd = Command::new("file")
                            .arg(&asset_path.to_str().unwrap_or_default())
                            .output();
                        let executable_file: bool = match cmd.await {
                            Ok(output) => {
                                let is_exec =
                                    String::from_utf8(output.stdout)?.contains("executable");

                                info!("Setting {:?} as executable", &asset_path);
                                match std::fs::set_permissions(
                                    &asset_path,
                                    std::fs::Permissions::from_mode(0o0755),
                                ) {
                                    Ok(_) => {
                                        debug!("Successfully set permissions for {:?}", asset_path)
                                    }
                                    Err(perm_err) => {
                                        error!("Unable to set permissions on {:?}", asset_path)
                                    }
                                }

                                is_exec
                            }
                            Err(cmd_err) => {
                                eyre::bail!("Unable to run `file` on {:?}", &asset_path)
                            }
                        };

                        if executable_file {
                            create_symlink(&asset_path, &symlink_dest);
                            match self.tools.iter_mut().find(|t| t.name == name) {
                                // add to the tools list
                                Some(installed_tool) => {
                                    installed_tool.set_current_version(&version);
                                    installed_tool.installed_versions.push(version.as_tag());
                                    info!(
                                        "Added new version {} of {} to environment {}",
                                        version.as_tag(),
                                        name,
                                        self.name
                                    );
                                }
                                // create a new tool, and add to our list
                                None => {
                                    info!("Added new tool {} to environment {}", name, self.name);
                                    self.tools.push(Tool::new(
                                        name,
                                        alias,
                                        &version,
                                        user_pattern,
                                        file_pattern,
                                    ));
                                }
                            };
                            Ok(())
                        } else {
                            eyre::bail!("Failed to determine the file type for {:?}", asset_path)
                        }
                    }
                }
            }
            Err(download_err) => {
                eyre::bail!(
                    "Failed to download file {} from {}",
                    asset.name,
                    asset.browser_download_url
                )
            }
        }
    }
}

fn create_symlink(src: &'_ Path, dest: &'_ Path) {
    match std::env::consts::OS {
        "windows" => unimplemented!(),
        "linux" | "macos" => {
            if dest.exists() {
                match std::fs::read_link(dest) {
                    Ok(read_link) => {
                        if *read_link != *dest {
                            // delete the symlink if it isn't pointing to the same file we are trying
                            // to use
                            info!("Removing existing symlink pointing at {:?}", read_link);
                            std::fs::remove_file(dest).unwrap()
                        }
                    }
                    Err(read_err) => panic!("Failed to read symlink. {:?}", read_err),
                };
            }
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            info!("Creating symlink from {:?} to {:?}", src, dest);
            match std::os::unix::fs::symlink(src, dest) {
                Ok(_) => (),
                Err(e) => error!("Failed to create symlink from {:?} to {:?}", src, dest),
            }
        }
        _ => panic!("unknown operating system"),
    }
}

fn find_binary(folder: &'_ Path, bin_name: &'_ str) -> Option<DirEntry> {
    WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name() == bin_name)
}
