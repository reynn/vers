use thiserror::Error;

/// TODO: write docs
pub type Result<T, E = MachineError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Error)]
/// TODO: write docs
pub enum MachineError {
    #[error("Discovered OS is not recognized: {0}")]
    /// TODO: write docs
    UnknownOs(String),
    #[error("Failed to determine appropriate architecture, provided {0}")]
    /// TODO: write docs
    UnknownArch(String),
}
