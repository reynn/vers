use crate::cli::Cli;
use crate::prelude::*;
use structopt::{
    clap::{arg_enum, Shell},
    StructOpt,
};
// use vers_core::prelude::*;

#[derive(Debug, Clone, StructOpt)]
pub struct CompletionsCmd {
    #[structopt(short, long, case_insensitive = true)]
    pub shell: CompletionsShell,
}

arg_enum! {
    #[derive(Debug, Clone)]
    pub enum CompletionsShell {
        Fish,
        Bash,
        Zsh,
        Powershell,
    }
}

impl From<CompletionsShell> for Shell {
    fn from(val: CompletionsShell) -> Self {
        match val {
            CompletionsShell::Fish => Shell::Fish,
            CompletionsShell::Bash => Shell::Bash,
            CompletionsShell::Zsh => Shell::Zsh,
            CompletionsShell::Powershell => Shell::PowerShell,
        }
    }
}

pub(crate) fn execute_completion_cmd(args: &'_ CompletionsCmd, cfg: &'_ Config) -> Result<()> {
    Cli::clap().gen_completions_to(
        env!("CARGO_PKG_NAME"),
        args.clone().shell.into(),
        &mut std::io::stdout(),
    );
    Ok(())
}
