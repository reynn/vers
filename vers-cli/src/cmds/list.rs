// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct ListCmd {
    #[structopt(short, long, aliases = &["env"])]
    pub environment: Option<String>,
    #[structopt(short, long)]
    pub short: Option<bool>,
}
