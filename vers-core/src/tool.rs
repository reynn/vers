use std::fmt::Display;

use crate::version::Version;
use thiserror::Error;
// use crate::release::Release;

#[derive(Debug, Clone, Error)]
pub enum ToolError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool<'t> {
    pub name: &'t str,
    pub version: Version,
    // pub current_release: Option<&'t Release>,
    // pub installed_releases: Option<Vec<&'t Release>>,
}

impl<'t> Display for Tool<'t> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'t> Default for Tool<'t> {
    fn default() -> Self {
        Self {
            name: "cli/cli",
            version: Version::Latest,
        }
    }
}
