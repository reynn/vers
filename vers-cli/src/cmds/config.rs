// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ConfigCmd {
    #[structopt(short, long)]
    pub name: String,
}
