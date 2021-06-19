use crate::errors::*;
use directories_next::ProjectDirs;
use log::*;
pub use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    environment_directory: PathBuf,
    #[serde(skip)]
    file_path: PathBuf,
}

impl Config {
    pub fn load<P: Into<PathBuf>>(file_path: Option<P>) -> Result<Self> {
        if let Some(file_path) = file_path {
            let file_path: PathBuf = file_path.into();
            let config_file_contents = std::fs::read_to_string(file_path)?;
            Ok(toml::from_str(&config_file_contents)?)
        } else {
            error!("No filepath to a config file provided");
            Err(VersCoreError::General("".into()))
        }
    }

    pub fn save(&self) -> Result<()> {
        Ok(std::fs::write(
            &self.file_path,
            toml::to_string_pretty(self)?,
        )?)
    }
}

impl Default for Config {
    fn default() -> Self {
        let project_directories = ProjectDirs::from("dev", "reynn", "vers").unwrap();
        let data_directory = project_directories.data_local_dir();
        let env_directory = data_directory.join("envs");

        Self {
            environment_directory: env_directory.clone().into(),
            file_path: data_directory.join(".vers.toml").into(),
        }
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        debug!("Saving Config file");
        if let Some(parent_dir) = self.file_path.parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir).unwrap();
            }
        }
        self.save().unwrap()
    }
}
