use crate::{archiver, dirs, download, tool::Tool, version::Version};
use async_std::fs::read_to_string;
use octocrab::models::repos::Asset;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::{debug, error, info};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Error)]
pub enum EnvironmentFileError {
    #[error("Failed to create the directory '{path}'. {source}")]
    FailedToCreateDirectory {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Unable to read file '{file_path}'. {source:?}")]
    FileReadError {
        file_path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to deserialize file: {file_path}, using {format}. {msg}")]
    DeserializationError {
        file_path: std::path::PathBuf,
        format: String,
        msg: String,
        // source: Box<dyn serde::de::Error>,
    },
    #[error("Failed to acquire the environment lock file '{path}'. {source}")]
    LockFileError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to serialize the '{name}' environment. {source}")]
    SerializationError {
        name: String,
        source: serde_json::Error,
    },
    #[error("Unable to determine the environment directory from '{base_dir}'")]
    InvalidBaseDir { base_dir: String },
    #[error("Failed to write the environment file '{path}'. {source}")]
    FileWriteError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to replace the environment file '{path}'. {source}")]
    FilePersistError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("Unable to find binary '{expected_file_name}' within folder {search_base_path}")]
    UnableToFindBinaryError {
        expected_file_name: String,
        search_base_path: std::path::PathBuf,
    },
    #[error("Failed to download file '{asset_name}' from '{asset_uri}'")]
    AssetDownloadError {
        asset_uri: reqwest::Url,
        asset_name: String,
    },
}

type Result<T, E = EnvironmentFileError> = std::result::Result<T, E>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub base_dir: String,
    // #[serde(skip)]
    // base_dir_path: PathBuf,
    pub tools: Vec<Tool>,
    /// Exclusive lock held for the lifetime of an environment loaded for update.
    #[serde(skip)]
    lock_file: Option<File>,
}

impl Environment {
    pub async fn load<P: Into<PathBuf>>(config_dir: P, name: &'_ str) -> Result<Self> {
        // Read-only loads must not create the environment directory; `read`
        // treats a missing directory like a missing environment file.
        let env_dir = config_dir.into().join("envs");
        Self::read(env_dir, name, None).await
    }

    /// Load the environment while holding an exclusive lock on its lock file.
    ///
    /// Use this for commands that may mutate and later [`save`](Self::save) the
    /// environment. The lock is held until the returned `Environment` is dropped,
    /// so concurrent writers cannot interleave their read/modify/write cycles.
    pub async fn load_for_update<P: Into<PathBuf>>(config_dir: P, name: &'_ str) -> Result<Self> {
        let env_dir = Self::ensure_env_dir(config_dir.into())?;
        let lock_file = acquire_lock(&env_dir, name)?;
        Self::read(env_dir, name, Some(lock_file)).await
    }

    fn ensure_env_dir(config_dir: PathBuf) -> Result<PathBuf> {
        let env_dir = config_dir.join("envs");
        if !env_dir.exists() {
            std::fs::create_dir_all(&env_dir).map_err(|source| {
                EnvironmentFileError::FailedToCreateDirectory {
                    path: env_dir.clone(),
                    source,
                }
            })?;
        }
        Ok(env_dir)
    }

    async fn read(env_dir: PathBuf, name: &'_ str, lock_file: Option<File>) -> Result<Self> {
        let env_path = env_dir.join(format!("{}.json", name));
        match read_to_string(&env_path).await {
            Ok(file_contents) => match from_str::<Self>(&file_contents) {
                Ok(mut res) => {
                    res.base_dir = env_dir
                        .join(name)
                        .to_str()
                        .expect("Unable to convert path to a string")
                        .to_string();
                    res.lock_file = lock_file;
                    Ok(res)
                }
                Err(serde_err) => Err(EnvironmentFileError::DeserializationError {
                    file_path: env_path,
                    format: "json".into(),
                    msg: serde_err.to_string(),
                }),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!("Environment file does not exist");
                    let base_dir: String = env_dir.join(name).to_str().unwrap_or_default().into();
                    Ok(Environment {
                        name: name.to_string(),
                        base_dir,
                        tools: Vec::new(),
                        lock_file,
                    })
                }
                _ => Err(EnvironmentFileError::FileReadError {
                    file_path: env_path,
                    source: read_err,
                }),
            },
        }
    }

    /// Atomically persist the environment to disk.
    ///
    /// The JSON is written to a temporary file next to the environment file,
    /// flushed to disk, and then renamed over the real file. A lock file guards
    /// against concurrent writers; mutating commands should load with
    /// [`load_for_update`](Self::load_for_update) so the lock also covers the
    /// read/modify/write cycle.
    pub fn save(&self) -> Result<()> {
        let env_dir = Path::new(&self.base_dir).parent().ok_or_else(|| {
            EnvironmentFileError::InvalidBaseDir {
                base_dir: self.base_dir.clone(),
            }
        })?;

        std::fs::create_dir_all(env_dir).map_err(|source| {
            EnvironmentFileError::FailedToCreateDirectory {
                path: env_dir.to_path_buf(),
                source,
            }
        })?;

        // Mutating commands hold the lock already, via `load_for_update`. Any
        // other caller still gets a consistent, non-torn write.
        let _transient_lock = if self.lock_file.is_none() {
            Some(acquire_lock(env_dir, &self.name)?)
        } else {
            None
        };

        let contents =
            to_string_pretty(self).map_err(|source| EnvironmentFileError::SerializationError {
                name: self.name.clone(),
                source,
            })?;

        let out_file = env_dir.join(format!("{}.json", self.name));
        let temp_file = env_dir.join(format!(".{}.json.tmp.{}", self.name, std::process::id()));

        if let Err(source) = write_and_sync(&temp_file, contents.as_bytes()) {
            let _ = std::fs::remove_file(&temp_file);
            return Err(EnvironmentFileError::FileWriteError {
                path: temp_file,
                source,
            });
        }

        if let Err(source) = std::fs::rename(&temp_file, &out_file) {
            let _ = std::fs::remove_file(&temp_file);
            return Err(EnvironmentFileError::FilePersistError {
                path: out_file,
                source,
            });
        }

        // Make the rename itself durable. Not every platform permits syncing a
        // directory, so failure here is not fatal.
        if let Ok(dir) = File::open(env_dir) {
            let _ = dir.sync_all();
        }

        debug!("Saved environment '{}' to {:?}", self.name, out_file);
        Ok(())
    }

    pub async fn add_tool(
        &mut self,
        name: &'_ str,
        alias: &'_ str,
        version: Version,
        asset: Asset,
        asset_pattern: &'_ str,
        file_pattern: &'_ str,
    ) -> std::result::Result<(), EnvironmentError> {
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
                            let binary_file_name = if !file_pattern.is_empty() {
                                file_pattern
                            } else {
                                alias
                            };
                            if let Some(bin_file) = find_binary(&tool_version_dir, binary_file_name)
                            {
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
                                return Err(EnvironmentError::UnableToFindBinaryError {
                                    expected_file_name: binary_file_name.to_string(),
                                    search_base_path: tool_version_dir,
                                });
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to extract using '{}' Error: {:?}",
                                extractor_name, e,
                            );
                        }
                    }
                }

                Ok(())
            }
            Err(_) => Err(EnvironmentError::AssetDownloadError {
                asset_uri: asset.browser_download_url,
                asset_name: asset.name,
            }),
        }
    }
}

/// Open (creating if necessary) and exclusively lock the environment's lock file.
///
/// The returned `File` owns the lock; it is released when the file is dropped.
fn acquire_lock(env_dir: &'_ Path, name: &'_ str) -> Result<File> {
    let lock_path = env_dir.join(format!("{}.lock", name));
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|source| EnvironmentFileError::LockFileError {
            path: lock_path.clone(),
            source,
        })?;

    lock_file
        .lock()
        .map_err(|source| EnvironmentFileError::LockFileError {
            path: lock_path,
            source,
        })?;

    Ok(lock_file)
}

fn write_and_sync(path: &'_ Path, contents: &'_ [u8]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[async_std::test]
    async fn load_does_not_write_environment_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");

        let env = Environment::load(&config_dir, "global").await.unwrap();

        assert!(env.tools.is_empty());
        assert!(!config_dir.join("envs").join("global.json").exists());
        assert!(!config_dir.join("envs").exists());
    }

    #[async_std::test]
    async fn save_writes_and_round_trips_environment() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let mut env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();
        env.tools
            .push(Tool::new("cli/cli", "gh", &Version::Latest, "", "gh"));

        env.save().unwrap();

        let reloaded = Environment::load(&config_dir, "global").await.unwrap();
        assert_eq!(reloaded.tools.len(), 1);
        assert_eq!(reloaded.tools[0].name, "cli/cli");
        assert_eq!(reloaded.tools[0].alias, "gh");
    }

    #[async_std::test]
    async fn save_replaces_the_existing_file_without_leaving_temporaries() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let mut env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();

        env.tools
            .push(Tool::new("cli/cli", "gh", &Version::Latest, "", "gh"));
        env.save().unwrap();

        env.tools.push(Tool::new(
            "neovim/neovim",
            "nvim",
            &Version::Latest,
            "",
            "nvim",
        ));
        env.save().unwrap();

        let env_dir = config_dir.join("envs");
        let temporary_files: Vec<String> = std::fs::read_dir(&env_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp."))
            .collect();

        assert!(temporary_files.is_empty(), "{temporary_files:?}");
        let reloaded = Environment::load(&config_dir, "global").await.unwrap();
        assert_eq!(reloaded.tools.len(), 2);
    }

    #[async_std::test]
    async fn load_for_update_creates_a_lock_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");

        let _env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();

        assert!(config_dir.join("envs").join("global.lock").exists());
    }
}
