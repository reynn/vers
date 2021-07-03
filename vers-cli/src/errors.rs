use thiserror::Error;
use vers_core::prelude::*;

pub type Result<T, E = CliError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("There was an error in the core library: {0}")]
    VersCoreError(#[from] VersCoreError),
    #[error("There was an error in the internal environment: {0}")]
    VersEnvironmentError(#[from] EnvironmentError),
}
