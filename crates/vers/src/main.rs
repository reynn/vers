// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]
#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_macros, unused_imports, unused_variables)
)]

mod cli;
mod cli_actions;
mod config;
mod dirs;
mod download;
mod environment;
mod executor;
mod extractor;
mod github;
mod system;
mod tool;
mod version;

use {
    crate::{
        cli::Actions,
        cli_actions::{AddOpts, UpdateType},
        environment::Environment,
        executor::*,
        system::System,
    },
    clap::Command,
    clap_complete::Generator,
    log::*,
};

pub type Result<T> = eyre::Result<T>;

// #[tokio::main]
fn main() -> Result<()> {
    let opts = cli::new();
    env_logger::builder()
        .filter_level(opts.verbose.log_level_filter())
        .init();

    println!("------------------------------");
    ["github", "golang"].iter().for_each(|plugin| {
        execute_wasm_test(format!("target/wasm32-wasi/debug/vers_{}.wasm", plugin)).unwrap();
        println!("------------------------------");
    });

    // std::process::exit(1);
    // let config_dir = dirs::get_default_config_path();
    //
    // match opts.action {
    //     Actions::Add {
    //         name,
    //         alias,
    //         user_pattern,
    //         file_pattern,
    //         plugin,
    //         pre_release,
    //         show,
    //     } => {
    //         debug!("CLI: Name `{name}`, alias `{:?}`, pattern `{:?}`, filter `{:?}`, pre_release `{pre_release}`, show `{show}`, plugin: `{:?}`", alias, user_pattern, file_pattern, plugin);
    //         let system = System::new();
    //         let mut env = Environment::load(&config_dir, &opts.env).await?;
    //         cli_actions::add_new_tool(
    //             &mut env,
    //             &name,
    //             &system,
    //             &AddOpts {
    //                 user_pattern,
    //                 file_pattern,
    //                 alias,
    //                 show,
    //                 pre_release,
    //             },
    //         )
    //         .await?;
    //     },
    //     Actions::Remove { name, all } => {
    //         let mut env = Environment::load(&config_dir, &opts.env).await?;
    //         cli_actions::remove_tool(&mut env, &name, all).await?;
    //     },
    //     Actions::List { installed } => {
    //         let env = Environment::load(&config_dir, &opts.env).await?;
    //         cli_actions::list_tools(&env, installed).await?;
    //     },
    //     Actions::Update { name } => {
    //         let system = System::new();
    //         let mut env = Environment::load(&config_dir, &opts.env).await?;
    //         cli_actions::update_tools(
    //             &mut env,
    //             &system,
    //             if let Some(name) = name {
    //                 UpdateType::Specific(name)
    //             } else {
    //                 UpdateType::All
    //             },
    //         )
    //         .await?;
    //     },
    //     Actions::Env { name, shell } => {
    //         let name = match name {
    //             Some(name) => name,
    //             None => opts.env,
    //         };
    //         let env = Environment::load(&config_dir, &name).await?;
    //         match &shell[..] {
    //             "fish" => println!("set -p PATH \"{}\"", env.base_dir),
    //             "bash" | "sh" | "zsh" => println!("export PATH=\"{}:$PATH\"", env.base_dir),
    //             _ => panic!("{} is not a known shell", shell),
    //         }
    //     },
    //     Actions::Sync => {
    //         let system = System::new();
    //         let mut env = Environment::load(&config_dir, &opts.env).await?;
    //         println!("Syncing versions with {} configuration.", env.name);
    //         cli_actions::sync_tools(&mut env, &system).await?;
    //     },
    //     Actions::Completions { shell } => {
    //         let mut cmd = cli::Opts::cmd();
    //         print_completions(shell, &mut cmd)
    //     },
    // };
    Ok(())
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    clap_complete::generate(gen, cmd, cmd.get_name().to_string(), &mut std::io::stdout())
}
