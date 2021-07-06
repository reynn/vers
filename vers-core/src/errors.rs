use crate::{environment::errors::EnvironmentError, machine::MachineError};
pub use thiserror::Error;

/// TODO: write docs
pub type Result<T, E = VersCoreError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
/// TODO: write docs
pub enum VersCoreError {
    #[error("Failed to find an existing environment with the name {0}")]
    /// TODO: write docs
    EnvFindError(String),
    #[error("General error in core: {0}")]
    /// TODO: write docs
    General(String),
    #[error("I/O Error: {0}")]
    /// TODO: write docs
    IoError(String),
    #[error("Failed to deserialize TOML: {0}")]
    /// TODO: write docs
    TomlDeError(#[from] toml::de::Error),
    #[error("Failed to serialize TOML: {0}")]
    /// TODO: write docs
    TomlSerError(#[from] toml::ser::Error),
    #[error("{0}")]
    /// TODO: write docs
    MachineError(#[from] MachineError),
    #[error("{0}")]
    /// TODO: write docs
    EnvironmentError(#[from] EnvironmentError),
}

impl From<&str> for VersCoreError {
    fn from(s: &str) -> Self {
        Self::General(s.to_owned())
    }
}
