// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct UninstallCmd {
    #[structopt(short, long)]
    pub name: String,
}
