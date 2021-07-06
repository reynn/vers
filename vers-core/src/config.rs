use crate::errors::*;
use directories_next::ProjectDirs;
use log::*;
pub use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// TODO: write docs
pub struct Config {
    /// The directory that contains all of the environments
    pub environment_directory: PathBuf,
    /// The name of the environment we will interact with
    #[serde(skip)]
    pub environment_name: String,
    /// The path to this exact file, used primarily for reference
    #[serde(skip)]
    file_path: PathBuf,
}

impl Config {
    /// TODO: write docs
    pub fn load<P: Into<PathBuf>>(file_path: Option<P>) -> Result<Self> {
        if let Some(file_path) = file_path {
            let file_path: PathBuf = file_path.into();
            let config_file_contents = std::fs::read_to_string(&file_path)
                .map_err(|e| VersCoreError::IoError(e.to_string()))?;
            let mut config: Self = toml::from_str(&config_file_contents)?;
            config.file_path = file_path;
            Ok(config)
        } else {
            Err(VersCoreError::General(
                "No filepath to a config file provided".into(),
            ))
        }
    }

    /// TODO: write docs
    pub fn save(&self) -> Result<()> {
        if let Some(parent_dir) = self.file_path.parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir)
                    .map_err(|e| VersCoreError::IoError(e.to_string()))?
            }
        }
        debug!("Saving config to {:?}", &self.file_path);
        std::fs::write(&self.file_path, toml::to_string_pretty(self)?)
            .map_err(|e| VersCoreError::IoError(e.to_string()))
    }
}

impl Default for Config {
    fn default() -> Self {
        let project_directories = ProjectDirs::from("dev", "reynn", "vers").unwrap();
        let data_directory = project_directories.data_local_dir();
        let env_directory = data_directory.join("envs");

        Self {
            environment_directory: env_directory,
            environment_name: "default".into(),
            file_path: data_directory.join(".vers.toml"),
        }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        self.save().unwrap()
    }
}
