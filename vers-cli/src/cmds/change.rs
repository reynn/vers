// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ChangeCmd {
    #[structopt(short, long)]
    pub name: String,
}
