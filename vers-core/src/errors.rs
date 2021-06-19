use super::{environment::EnvironmentError, machine::MachineError};
pub use thiserror::Error;

pub type Result<T, E = VersCoreError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum VersCoreError {
    #[error("Failed to find an existing environment with the name {0}")]
    EnvFindError(String),
    #[error("General error in core: {0}")]
    General(String),
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to deserialize TOML: {0}")]
    TomlDeError(#[from] toml::de::Error),
    #[error("Failed to serialize TOML: {0}")]
    TomlSerError(#[from] toml::ser::Error),
    #[error("{0}")]
    MachineError(#[from] MachineError),
    #[error("{0}")]
    EnvironmentError(#[from] EnvironmentError),
}

impl From<&str> for VersCoreError {
    fn from(s: &str) -> Self {
        Self::General(s.to_owned())
    }
}
