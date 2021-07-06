use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ConfigCmd {
    #[structopt(short, long)]
    pub name: String,
}

pub(crate) fn execute_config_cmd(_args: &ConfigCmd, _cfg: &'_ Config) -> Result<()> {
    Ok(())
}
