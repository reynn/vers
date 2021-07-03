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
    cfg: &'_ Config,
) -> Result<()> {
    let tool = &env.find_tool_by_name(args.clone().name)?;
    let version = Version::parse(&args.version).unwrap_or_default();
    vers_core::change_tool_version(&vers_core::ChangeToolOpts {
        environment: &env,
        tool,
        version: Some(&version),
    })?;
    Ok(())
}
