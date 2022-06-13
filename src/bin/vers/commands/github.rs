use clap::{Arg, ArgMatches, Command};
use vers::environment::Environment;

pub fn cli() -> Command<'static> {
    Command::new("github")
        .about("Install tools from GitHub releases")
        .args(vec![
            Arg::new("api_token").long("api-token").short('A').help(
                "Use a GitHub Personal Access Token to overcome some limitations with API calls",
            ),
            Arg::new("alias")
                .long("alias")
                .short('a')
                .help("Use an alias for calling the binary from the environment"),
            Arg::new("filter")
                .long("filter")
                .short('f')
                .help("Filter used to find the executable to link into the environment"),
            Arg::new("pattern")
                .long("pattern")
                .short('p')
                .help("Allow install of pre-release versions of the tool"),
            Arg::new("pre-release")
                .long("pre-release")
                .short('P')
                .help(""),
            Arg::new("REPO"),
        ])
}

pub fn exec(env: &mut Environment, matches: &ArgMatches) -> eyre::Result<()> {
    Ok(())
}
