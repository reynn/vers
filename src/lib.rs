// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]

pub mod archiver;
pub mod cli;
pub mod cli_actions;
pub mod dirs;
pub mod download;
pub mod environment;
pub mod github;
pub mod system;
pub mod tool;
pub mod version;
