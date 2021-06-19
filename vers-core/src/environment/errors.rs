use thiserror::Error;

pub type Result<T, E = EnvironmentError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Discovered OS is not recognized: {0}")]
    UnknownOs(String),
    #[error("Failed to determine appropriate architecture, provided {0}")]
    UnknownArch(String),
    #[error("Environment named {0} not found")]
    EnvironmentNotFoundByName(String),
    #[error("No environments with tool: {0}, not found")]
    EnvironmentNotFoundByTool(String),
    #[error("General Environment error: {0}")]
    General(String),
}

impl From<&str> for EnvironmentError {
    fn from(s: &str) -> Self {
        Self::General(s.to_owned())
    }
}
