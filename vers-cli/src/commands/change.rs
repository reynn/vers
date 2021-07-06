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
    let tool = &env.find_tool_by_name(&tool_name).expect(
        format!(
            "Tool not found in environment. Try installing first with `vers install {}`",
            tool_name
        )
        .as_str(),
    );
    let version = Version::parse(&args.version).unwrap_or_default();
    vers_core::change_tool_version(&vers_core::ChangeToolOpts {
        environment: &env,
        tool,
        version: Some(&version),
    })?;
    Ok(())
}
