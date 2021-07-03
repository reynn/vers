// use crate::errors::*;
use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct UninstallCmd {
    #[structopt(short, long)]
    pub name: String,
}

pub(crate) fn execute_uninstall_cmd(
    args: &'_ UninstallCmd,
    env: &'_ Environment,
    cfg: &'_ Config,
) -> Result<()> {
    Ok(())
}
