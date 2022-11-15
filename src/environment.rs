use {
    crate::{archiver, dirs, download, tool::Tool, version::Version},
    async_std::fs::read_to_string,
    octocrab::models::repos::Asset,
    serde::{Deserialize, Serialize},
    serde_json::{from_str, to_string_pretty},
    std::path::{Path, PathBuf},
    tracing::{debug, error, info},
    walkdir::{DirEntry, WalkDir},
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub base_dir: String,
    #[serde(skip)]
    base_dir_path: PathBuf,
    pub tools: Vec<Tool>,
}

impl Drop for Environment {
    fn drop(&mut self) {
        let out_file = Path::new(&self.base_dir).parent().unwrap();
        let out_file = out_file.join(format!("{}.json", &self.name));
        debug!("Writing {} environment file to {:?}", &self.name, out_file);

        match to_string_pretty(&self) {
            Ok(contents) => match std::fs::write(&out_file, contents) {
                Ok(_) => debug!("Wrote Environment to file {:?}", out_file),
                Err(e) => error!("Failed to write Environment file; {}", e),
            },
            Err(e) => error!("Failed to marshal Environment to JSON. {}", e),
        }
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
                    anyhow::bail!("Unable to create the envs directory: {}", create_dir_err)
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
                Err(serde_err) => anyhow::bail!(
                    "Failed to deserialize {:?} as an environment. {:?}",
                    env_path,
                    serde_err
                ),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!("Environment file does not exist");
                    let base_dir: String = env_dir.join(name).to_str().unwrap_or_default().into();
                    Ok(Environment {
                        name: name.to_string(),
                        base_dir: base_dir.clone(),
                        base_dir_path: Path::new(&base_dir).to_path_buf(),
                        tools: Vec::new(),
                    })
                }
                _ => anyhow::bail!(
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
        asset_pattern: &'_ str,
        file_pattern: &'_ str,
    ) -> crate::Result<()> {
        let env_base_path = Path::new(&self.base_dir);
        let tool_dir = dirs::get_tool_download_dir(env_base_path, name);
        info!("Actual tools dir: {:?}", tool_dir);

        let version_tag = version.as_tag();
        let tool_version_dir =
            dirs::get_tool_version_download_dir(env_base_path, name, &version_tag);

        match download::download_asset(&asset, &tool_version_dir).await {
            Ok(asset_path) => {
                info!("Completed downloading {}", asset.browser_download_url);
                let symlink_dest = dirs::get_tool_link_path(env_base_path, alias);
                let possible_extractors = archiver::determine_possible_extractors(&asset_path);

                for extractor in possible_extractors {
                    let extractor_name = &extractor.name();
                    match archiver::handle_file_extraction(
                        extractor,
                        &asset_path,
                        Some(tool_version_dir.clone()),
                    )
                    .await
                    {
                        Ok(_) => {
                            info!(
                                "Successfully extracted '{}' using the '{}' extractor",
                                &asset_path.display(),
                                extractor_name
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
                                        let version_tag = &version.as_tag();
                                        if !installed_tool
                                            .installed_versions
                                            .iter()
                                            .any(|v| v[..] == version_tag[..])
                                        {
                                            installed_tool
                                                .installed_versions
                                                .push(version.as_tag());
                                            info!(
                                                "Added new version {} of {} in environment {}",
                                                version.as_tag(),
                                                name,
                                                self.name
                                            );
                                        }
                                        return Ok(());
                                    }
                                    // create a new tool, and add to our list
                                    None => {
                                        self.tools.push(Tool::new(
                                            name,
                                            alias,
                                            &version,
                                            asset_pattern,
                                            file_pattern,
                                        ));
                                        info!(
                                            "Added new tool {} in environment {}",
                                            name, self.name
                                        );
                                        return Ok(());
                                    }
                                };
                            } else {
                                anyhow::bail!("Able to extract the file but not discover the required binary file")
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to extract using '{}' Error: {:?}",
                                extractor_name, e
                            )
                        }
                    }
                }

                Ok(())
            }
            Err(download_err) => {
                anyhow::bail!(
                    "Failed to download file {} from {}. {:?}",
                    asset.name,
                    asset.browser_download_url,
                    download_err
                )
            }
        }
    }

    pub fn get_env_path(&self) -> PathBuf {
        self.base_dir_path.clone()
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
                Err(e) => error!(
                    "Failed to create symlink from {:?} to {:?}. {:?}",
                    src, dest, e
                ),
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
