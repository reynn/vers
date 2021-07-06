//! A Prelude pattern to simplify library usage

/// TODO: write docs
pub mod v1 {
    pub use crate::{
        config::Config,
        environment::{errors::EnvironmentError, Environment},
        errors::VersCoreError,
        machine::{Details, DetailsError},
        tool::{Tool, ToolError},
        version::{Version, VersionError},
        OutputType,
    };
    pub use log::*;
}

pub use v1::*;
