use thiserror::Error;

pub type Result<T, E = MachineError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Error)]
pub enum MachineError {
    #[error("Discovered OS is not recognized: {0}")]
    UnknownOs(String),
    #[error("Failed to determine appropriate architecture, provided {0}")]
    UnknownArch(String),
}
