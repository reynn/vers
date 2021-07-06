use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct InstallCmd {
    /// The name of the tool you wish to install
    pub name: String,
    /// Provide a version or alias for which version of the tool to install
    #[structopt(short, long)]
    pub version: Option<String>,
}

pub fn execute_install_cmd(
    cmd: &'_ InstallCmd,
    env: &'_ Environment,
    _cfg: &'_ Config,
) -> Result<()> {
    let tool_name = &cmd.name;
    let version = &cmd.version.to_owned().unwrap_or_default();
    let tool = &Tool {
        name: tool_name.into(),
        version: Version::parse(&version)?,
    };
    vers_core::install_tool(vec![tool], env)?;
    Ok(())
}
