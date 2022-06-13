use clap::{ArgMatches, Command};
use vers::environment::Environment;

pub fn cli() -> Command<'static> {
    Command::new("completion")
}

pub fn exec(env: &mut Environment, matches: &ArgMatches) -> eyre::Result<()> {
    Ok(())
}
