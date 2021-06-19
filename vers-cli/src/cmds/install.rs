// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct InstallCmd {
    /// The name of the tool you wish to install
    // #[structopt(short, long)]
    pub name: Option<String>,
    /// Provide a version or alias for which version of the tool to install
    #[structopt(short, long)]
    pub version: Option<String>,
}
