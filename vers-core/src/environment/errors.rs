use thiserror::Error;

use crate::environment::{EnvironmentDirectoryError, EnvironmentNameError};

/// TODO: write docs
pub type Result<T, E = EnvironmentError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
/// TODO: write docs
pub enum EnvironmentError {
    #[error("I/O Error: {0}")]
    /// TODO: write docs
    IoError(#[from] std::io::Error),
    #[error("Discovered OS is not recognized: {0}")]
    /// TODO: write docs
    UnknownOs(String),
    #[error("Failed to determine appropriate architecture, provided {0}")]
    /// TODO: write docs
    UnknownArch(String),
    #[error("Environment named {0} not found")]
    /// TODO: write docs
    EnvironmentNotFoundByName(String),
    #[error("No environments with tool: {0}, not found")]
    /// TODO: write docs
    EnvironmentNotFoundByTool(String),
    #[error("There was a problem with the provided name of environment {0}")]
    /// TODO: write docs
    EnvironmentNameFailure(#[from] EnvironmentNameError),
    #[error("There was a problem with the provided directory of environment {0}")]
    EnvironmentDirectoryFailure(#[from] EnvironmentDirectoryError),
    #[error("Tool {0} not found in the {1} environment")]
    /// TODO: write docs
    ToolNotFoundInEnvironment(String, String),
    #[error("Failed to serialize TOML {0}")]
    /// TODO: write docs
    TomlSerializeError(#[from] toml::ser::Error),
    #[error("Failed to deserialize TOML {0}")]
    /// TODO: write docs
    TomlDeserializeError(#[from] toml::de::Error),
    #[error("General Environment error: {0}")]
    /// TODO: write docs
    General(String),
}

impl From<&str> for EnvironmentError {
    fn from(s: &str) -> Self {
        Self::General(s.to_owned())
    }
}
