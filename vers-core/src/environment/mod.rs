mod errors;

use log::*;
use std::{fmt::Display, fs::metadata, path::PathBuf};

pub use self::errors::*;
use crate::prelude::*;
use thiserror::*;

#[derive(Debug, Clone, Error)]
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
pub struct EnvironmentDirectoryError;

prae::define! {
    pub EnvironmentDirectory: PathBuf
    validate |e| -> Option<EnvironmentDirectoryError> {
        // if e. {} else
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
pub struct Environment<'e> {
    name: EnvironmentName,
    tools: Vec<Tool<'e>>,
    directory: PathBuf,
}

impl<'e> Drop for Environment<'e> {
    fn drop(&mut self) {
        self.save().unwrap();
    }
}

impl<'e> Display for Environment<'e> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name.get())
    }
}

impl<'e> Environment<'e> {
    /// Create a new [`Environment`] within the provided base_path
    ///
    /// ```rust
    /// // Environment will be created as <base_path>/<name>
    /// let env = Environment::new("temp", "/tmp/environments")?;
    /// ```
    fn new<P: Into<PathBuf>>(name: &'e str, base_path: P) -> Result<Environment<'e>> {
        let s = Self {
            name: EnvironmentName::new(name)?,
            tools: Vec::new(),
            directory: base_path.into(),
        };
        Ok(s)
    }

    /// Load or create an environment
    pub fn load_or_create<P: Into<PathBuf>>(
        name: Option<&'e str>,
        base_path: P,
    ) -> Result<Environment<'e>> {
        // let base_path = base_path.into();
        let name = name.unwrap_or("default");
        Ok(Environment::new(name, base_path)?)
        // Err(EnvironmentError::General("oops, no can do".into()))
    }

    /// save the environment configuration
    pub fn save(&self) -> Result<&'e Environment> {
        if !&self.directory.exists() {
            std::fs::create_dir_all(&self.directory)?;
        }
        Ok(self)
    }

    /// If unable to find an environment in the provided base path,
    /// returns a [`EnvironmentError::EnvironmentNotFoundByName`]
    pub fn find_env_by_name<P: Into<PathBuf>>(s: &'_ str, base_path: P) -> Result<Environment<'e>> {
        let base_path = base_path.into();
        info!("Finding environment named {} in {:?}", s, &base_path);
        if !&base_path.exists() {
            std::fs::create_dir_all(&base_path)?;
        }
        for entry in base_path.read_dir()? {
            let entry = entry?;
            let path = entry.path();

            let metadata = metadata(path)?;
            info!("{:?}", metadata);
        }
        Err(EnvironmentError::EnvironmentNotFoundByName(s.into()))
    }

    pub fn find_tool_by_name<S: Into<String>>(&self, tool_name: S) -> Result<Tool> {
        let tool_name: String = tool_name.into();
        info!("Searching for {} in {:?}", tool_name, &self.directory);

        Err(EnvironmentError::ToolNotFoundInEnvironment(
            tool_name,
            self.directory
                .as_path()
                .to_str()
                .unwrap_or("invalid-path")
                .into(),
        ))
    }

    pub fn tools(&self) -> Option<Vec<Tool<'_>>> {
        None
    }
}

impl<'e> Default for Environment<'e> {
    fn default() -> Self {
        Self::new("default", "").unwrap()
    }
}
