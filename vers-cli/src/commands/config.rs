use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ConfigCmd {}

pub(crate) fn execute_config_cmd(_args: &ConfigCmd, cfg: &'_ Config) -> Result<()> {
    debug!("Executing config command, current config:\n\t{:?}", cfg);
    Ok(())
}
