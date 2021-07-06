/// TODO: write docs
pub mod manager;

use std::fmt::Display;

use crate::version::Version;
use serde::{Deserialize, Serialize};
use thiserror::Error;
// use crate::release::Release;

#[derive(Debug, Clone, Error)]
/// TODO: write docs
pub enum ToolError {
    #[error("{0}")]
    GeneralError(String),
}

pub struct ToolNameError;

prae::define! {
    pub ToolName: String
    validate |e| -> Option<ToolNameError> {
        if e.is_empty() {
            Some(ToolNameError)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// TODO: write docs
pub struct Tool {
    /// TODO: write docs
    pub name: String,
    /// TODO: write docs
    pub version: Version,
    // pub current_release: Option<&'t Release>,
    // pub installed_releases: Option<Vec<&'t Release>>,
}

impl Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Default for Tool {
    fn default() -> Self {
        Self {
            name: "cli/cli".into(),
            version: Version::Latest,
        }
    }
}
