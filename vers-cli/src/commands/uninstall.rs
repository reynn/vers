// use crate::errors::*;
use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct UninstallCmd {
    #[structopt(short, long)]
    pub name: String,
}

pub(crate) fn execute_uninstall_cmd(
    _args: &'_ UninstallCmd,
    _env: &'_ Environment,
    _cfg: &'_ Config,
) -> Result<()> {
    Ok(())
}
