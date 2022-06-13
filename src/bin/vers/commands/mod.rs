use clap::{ArgMatches, Command};
use vers::environment::Environment;

pub mod completion;
pub mod env;
pub mod github;
pub mod go;
pub mod list;
pub mod remove;
pub mod sync;
pub mod update;

pub fn builtins() -> Vec<Command<'static>> {
    vec![
        completion::cli(),
        env::cli(),
        github::cli(),
        go::cli(),
        list::cli(),
        remove::cli(),
        sync::cli(),
        update::cli(),
    ]
}

pub fn builtin_exec(cmd: &'_ str) -> Option<fn(&mut Environment, &ArgMatches) -> eyre::Result<()>> {
    let m = match cmd {
        "completion" => completion::exec,
        "github" => github::exec,
        "go" => go::exec,
        "list" => list::exec,
        "remove" => remove::exec,
        "sync" => sync::exec,
        "update" => update::exec,
        _ => return None,
    };
    Some(m)
}
