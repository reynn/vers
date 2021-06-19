use thiserror::Error;

pub type Result<T, E = CliError> = std::result::Result<T, E>;

#[derive(Debug, Clone, Error)]
pub enum CliError {}
