use clap::{Arg, ArgMatches, Command};
use vers::environment::Environment;

pub fn cli() -> Command<'static> {
    Command::new("go")
        .about("Install Golang versions")
        .args(vec![Arg::new("version")])
}

pub fn exec(env: &mut Environment, matches: &ArgMatches) -> eyre::Result<()> {
    Ok(())
}
