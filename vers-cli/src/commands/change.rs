use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ChangeCmd {
    #[structopt(short, long)]
    pub name: String,
    #[structopt(short, long)]
    pub version: String,
}

pub(crate) fn execute_change_cmd(
    args: &'_ ChangeCmd,
    env: &'_ Environment,
    _cfg: &'_ Config,
) -> Result<()> {
    let tool_name = args.clone().name;
    let version = Version::parse(&args.version).unwrap_or_default();
    Ok(vers_core::change_tool_version(
        &env,
        &tool_name,
        Some(&version),
    )?)
}
