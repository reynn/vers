use crate::cli::Cli;
use clap::CommandFactory;
use clap_complete::Shell;
use std::io::stdout;

pub fn generate_completions(shell: &'_ Shell) {
    let mut cmd = Cli::command();
    let cmd_name = cmd.get_name().to_string();
    clap_complete::generate(*shell, &mut cmd, cmd_name, &mut stdout())
}
