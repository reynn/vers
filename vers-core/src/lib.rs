//! Vers Core - A library for managing versions of developer tools.
//! This includes things such as CLI tools, Lanaguage versions and potentially many others.

// Require docs for public APIs, deny unsafe code, etc.
#![forbid(unsafe_code, unused_must_use, unstable_features)]
#![deny(
    clippy::clone_on_ref_ptr,
    trivial_casts,
    trivial_numeric_casts,
    // missing_docs,
    unreachable_pub,
    unused_import_braces,
    unused_extern_crates,
    unused_qualifications
)]

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
pub mod prelude;
/// A tool is anything that can be installed into an environment, this ranges from a GitHub
/// Release, to a set of language tools for Go, Rust, NodeJS, etc.
pub mod tool;
/// A version is a set of libraries to translate or parse versions
pub mod version;

use crate::prelude::*;

/// The logic to install a [`Tool`] into an [`Environment`]
pub fn install_tool(
    tools: Vec<&'_ Tool>,
    environment: &'_ Environment,
) -> crate::errors::Result<()> {
    for tool in tools {
        info!("Installing tool {} into environment: {}", tool, environment);
    }
    Ok(())
}

/// TODO: write docs
pub struct UpdateToolOpts<'uto> {
    /// TODO: write docs
    pub environment: &'uto Environment,
    /// TODO: write docs
    pub tool: &'uto Tool,
    /// TODO: write docs
    pub version: Option<&'uto Version>,
}

/// Update a [`Tool`] currently installed in an [`Environment`]
pub fn update_tool(_opts: &UpdateToolOpts) -> crate::errors::Result<()> {
    Ok(())
}

/// Update a [`Tool`] currently installed in an [`Environment`]
pub fn change_tool_version(
    environment: &'_ Environment,
    tool: &'_ str,
    version: Option<&'_ Version>,
) -> crate::errors::Result<()> {
    let version = version.unwrap_or(&Version::Latest);
    let tool_name = tool;
    info!(
        "Changing to version {} of {} in the {} environment",
        version, tool_name, environment
    );
    Ok(())
}

#[derive(Debug, Clone)]
/// TODO: write docs
pub enum OutputType {
    /// TODO: write docs
    Json,
    /// TODO: write docs
    Yaml,
    /// TODO: write docs
    Text,
}

/// TODO: write docs
pub fn list_tools(
    environment: &'_ Environment,
    output_type: &'_ OutputType,
) -> crate::errors::Result<()> {
    let tools = environment.tools();
    if tools.is_empty() {
        println!("No tools are currently installed");
    } else {
        println!("--> There are currently {} tools installed", tools.len());
        match output_type {
            OutputType::Json => todo!(),
            OutputType::Yaml => todo!(),
            OutputType::Text => tools.iter().for_each(|f| println!("{}", f.name)),
        }
    }
    Ok(())
}
