// use crate::errors::*;
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct EnvironmentCmd {
    #[structopt(short, long, default_value = "default")]
    pub name: String,
}
