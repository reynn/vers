// Require docs for public APIs, deny unsafe code, etc.
// #![forbid(unsafe_code, unused_must_use, unstable_features)]
// #![deny(
//     clippy::clone_on_ref_ptr,
//     trivial_casts,
//     trivial_numeric_casts,
//     missing_docs,
//     unreachable_pub,
//     unused_import_braces,
//     unused_extern_crates,
//     unused_qualifications
// )]

/// TODO: write docs
pub mod archive;
/// TODO: write docs
pub mod asset;
/// TODO: write docs
pub mod config;
/// Environments help group usable tools together, this can be helpful for workflows where you need to keep
/// mutlple versions of the same tool, for instance multiple different versions of `kubectl` to connect to
/// legacy clusters and new clusters
pub mod environment;
/// TODO: write docs
pub mod errors;
/// TODO: write docs
pub mod machine;
/// TODO: write docs
pub mod manager;
/// TODO: write docs
pub mod prelude;
/// A tool is anything that can be installed into an environment, this ranges from a GitHub
/// Release, to a set of language tools for Go, Rust, NodeJS, etc.
pub mod tool;
/// A version is a set of libraries to translate or parse versions
pub mod version;

use crate::{environment::Environment, errors::Result, tool::Tool, version::Version};
use log::*;

pub struct InstallToolOpts<'ito> {
    pub tools: Vec<&'ito Tool<'ito>>,
    pub environment: &'ito Environment<'ito>,
}

/// The logic to install a [`Tool`] into an [`Environment`]
pub fn install_tool(opts: &InstallToolOpts) -> Result<()> {
    for tool in &opts.tools {
        info!(
            "Installing tool {} into environment: {}",
            tool, &opts.environment
        );
    }
    Ok(())
}

pub struct UpdateToolOpts<'uto> {
    pub environment: &'uto Environment<'uto>,
    pub tool: &'uto Tool<'uto>,
    pub version: Option<&'uto Version>,
}

/// Update a [`Tool`] currently installed in an [`Environment`]
pub fn update_tool(_opts: &UpdateToolOpts) -> Result<()> {
    Ok(())
}

pub struct ChangeToolOpts<'cto> {
    pub environment: &'cto Environment<'cto>,
    pub tool: &'cto Tool<'cto>,
    pub version: Option<&'cto Version>,
}

/// Update a [`Tool`] currently installed in an [`Environment`]
pub fn change_tool_version(opts: &ChangeToolOpts) -> Result<()> {
    let version = opts.version.unwrap_or(&Version::Latest);
    info!(
        "Changing to version {} of {} in the {} environment",
        version, opts.tool.name, opts.environment
    );
    Ok(())
}

#[derive(Debug, Clone)]
pub enum OutputType {
    Json,
    Yaml,
    Text,
}

#[derive(Debug, Clone)]
pub struct ListToolsOpts<'lo> {
    pub environment: &'lo Environment<'lo>,
    pub output_type: &'lo OutputType,
}

pub fn list_tools(opts: &ListToolsOpts) -> Result<()> {
    let tools = &opts.environment.tools().unwrap_or_default();
    if tools.is_empty() {
        println!("No tools are currently installed");
    } else {
        println!("--> There are currently {} tools installed", tools.len());
        match opts.output_type {
            OutputType::Json => todo!(),
            OutputType::Yaml => todo!(),
            OutputType::Text => tools.iter().for_each(|f| println!("{}", f.name)),
        }
    }
    Ok(())
}
