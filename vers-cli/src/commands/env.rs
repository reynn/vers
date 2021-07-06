use crate::prelude::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct EnvironmentCmd {
    #[structopt(subcommand)]
    pub sub_cmd: EnvironmentSubCmds,
}

#[derive(Debug, Clone, StructOpt)]
pub enum EnvironmentSubCmds {
    List {
        #[structopt(short, long)]
        all: bool,
    },
    Create {
        name: String,
    },
    Set {
        name: String,
    },
    Remove {
        name: String,
    },
}

pub fn execute_env_subcommand(subcommand: &'_ EnvironmentSubCmds, _cfg: &'_ Config) -> Result<()> {
    match subcommand {
        EnvironmentSubCmds::List { all } => log::info!("List: all({})", all),
        EnvironmentSubCmds::Create { name } => log::info!("Create: name({})", name),
        EnvironmentSubCmds::Remove { name } => log::info!("Remove: name({})", name),
        EnvironmentSubCmds::Set { name } => log::info!("Set: name({})", name),
    };
    Ok(())
}
