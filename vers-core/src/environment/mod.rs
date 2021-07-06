pub mod errors;

use crate::tool::Tool;

use self::errors::*;
use log::*;
// use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::metadata,
    path::{Path, PathBuf},
};
use thiserror::*;

#[derive(Debug, Clone, Error)]
/// TODO: write docs
pub struct EnvironmentNameError;

impl Display for EnvironmentNameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

prae::define! {
    pub EnvironmentName: String
    validate |e| -> Option<EnvironmentNameError> {
        let e = e.trim();
        if e.is_empty() {
            Some(EnvironmentNameError)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
/// TODO: write docs
pub struct EnvironmentDirectoryError;

prae::define! {
    pub EnvironmentDirectory: PathBuf
    validate |e| -> Option<EnvironmentDirectoryError> {
        if !e.exists() {
            if let Err(create_err) = std::fs::create_dir_all(e) {
                error!("Failed to create directory: {:?}. Error: {:?}", e, create_err);
                Some(EnvironmentDirectoryError)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// TODO: write docs
pub struct Environment {
    /// The name of the environment.
    name: EnvironmentName,
    /// A set of tools that live as long as the environment.
    tools: Vec<Tool>,
    /// The directory where the environment is located.
    directory: PathBuf,
}

impl Drop for Environment {
    fn drop(&mut self) {
        self.save().unwrap();
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.get())
    }
}

impl Environment {
    /// Create a new [`Environment`] within the provided base_path
    ///
    /// ```rust
    /// // Environment will be created as <base_path>/<name>
    /// # use vers_core::environment::Environment;
    /// let env = Environment::new("temp", "/tmp/environments").unwrap();
    /// assert_eq!(env.directory(), "/tmp/environments/temp");
    /// ```
    pub fn new<P: Into<PathBuf>>(name: &'_ str, base_path: P) -> Result<Environment> {
        let s = Self {
            name: EnvironmentName::new(name)?,
            tools: Vec::new(),
            directory: base_path.into(),
        };
        Ok(s)
    }

    /// Provide the directory where the environment is located.
    pub fn directory(&self) -> String {
        self.directory
            .join(self.name.get())
            .to_str()
            .unwrap_or_default()
            .into()
    }

    /// Load or create an environment
    pub fn load_or_create<P: Into<PathBuf>>(
        name: Option<&'_ str>,
        base_path: P,
    ) -> Result<Environment> {
        // let base_path = base_path.into();
        let name = name.unwrap_or("default");
        Environment::new(name, base_path)
    }

    /// save the environment configuration
    pub fn save(&self) -> Result<&'_ Environment> {
        let env_dir = self.directory();
        debug!("env_dir: {:?}", &env_dir);
        let env_dir = Path::new(&env_dir);
        if !env_dir.exists() {
            std::fs::create_dir_all(&env_dir)?;
        }
        let mut env_cfg: HashMap<&str, &str> = std::collections::HashMap::new();
        env_cfg.insert("name", self.name.get());
        let env_cfg = toml::to_string_pretty(&env_cfg)?;
        std::fs::write(&env_dir.join(".vers.env.toml"), env_cfg)?;
        Ok(self)
    }

    /// If unable to find an environment in the provided base path,
    /// returns a [`EnvironmentError::EnvironmentNotFoundByName`]
    pub fn find_env_by_name<P: Into<PathBuf>>(s: &'_ str, base_path: P) -> Result<Environment> {
        let base_path = base_path.into();
        info!("Finding environment named {} in {:?}", s, &base_path);
        if !&base_path.exists() {
            return Err(EnvironmentError::EnvironmentNotFoundByName(s.into()));
        }
        for entry in base_path.read_dir()? {
            let entry = entry?;
            let path = entry.path();
            debug!("Environment directory entry: {:?}", &path);

            let metadata = metadata(path)?;
            debug!("{:?}", metadata);
        }
        Err(EnvironmentError::EnvironmentNotFoundByName(s.into()))
    }

    /// Find a tool by name in the environment.
    pub fn find_tool_by_name(&self, name: &'_ str) -> Option<Tool> {
        for tool in self.tools() {
            if tool.name == name.to_string() {
                return Some(tool);
            }
        }
        None
    }

    // /// TODO: write docs
    // pub fn find_tool_by_name<S: Into<String>>(&self, tool_name: S) -> Result<Tool> {
    //     let tool_name: String = tool_name.into();

    //     info!("Searching for {} in {:?}", tool_name, &self.directory);

    //     Err(EnvironmentError::ToolNotFoundInEnvironment(
    //         tool_name,
    //         self.directory
    //             .as_path()
    //             .to_str()
    //             .unwrap_or("invalid-path")
    //             .into(),
    //     ))
    // }

    /// Returns a list of all tools installed in the environment.
    pub fn tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new("default", "").unwrap()
    }
}
