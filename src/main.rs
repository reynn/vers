// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]
// #![cfg_attr(
//     debug_assertions,
//     allow(dead_code, unused_macros, unused_imports, unused_variables)
// )]

mod archiver;
mod cli;
mod cli_actions;
mod dirs;
mod download;
mod environment;
mod github;
mod manager;
mod system;
mod tool;
mod version;

use {
    crate::{cli::Actions, cli_actions::UpdateType, environment::Environment, system::System},
    log::*,
};

pub type Result<T> = eyre::Result<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::opts().run();

    env_logger::builder()
        .filter_level(match opts.verbose {
            3 => LevelFilter::Trace,
            2 => LevelFilter::Debug,
            1 => LevelFilter::Info,
            _ => LevelFilter::Warn,
        })
        .init();

    let config_dir = dirs::get_default_config_path();

    if let Some(api_token) = opts.github_api_token {
        info!("Initializing the GitHub client with token from CLI args");
        octocrab::initialise(octocrab::Octocrab::builder().personal_token(api_token))?;
    } else if let Some(env_api_token) = std::env::var_os("GITHUB_TOKEN") {
        info!("Initializing the GitHub client with token from environment");
        octocrab::initialise(
            octocrab::Octocrab::builder().personal_token(env_api_token.to_str().unwrap().into()),
        )?;
    };

    match opts.action {
        Actions::Add {
            name,
            alias,
            pattern,
            filter,
            pre_release,
            show,
        } => {
            debug!("CLI: Name `{name}`, alias `{:?}`, pattern `{:?}`, filter `{:?}`, pre_release `{pre_release}`, show `{show}`", alias, pattern, filter);
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::add_new_tool(
                &mut env,
                &name,
                &system,
                pattern,
                filter,
                alias,
                show,
                pre_release,
            )
            .await?;
        }
        Actions::Remove { name, all } => {
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::remove_tool(&mut env, &name, all).await?;
        }
        Actions::List { installed } => {
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::list_tools(&env, installed).await?;
        }
        Actions::Update { name } => {
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::update_tools(
                &mut env,
                &system,
                if let Some(name) = name {
                    UpdateType::Specific(name)
                } else {
                    UpdateType::All
                },
            )
            .await?;
        }
        Actions::Env { name, shell } => {
            let name = match name {
                Some(name) => name,
                None => opts.env,
            };
            let env = Environment::load(&config_dir, &name).await?;
            match &shell[..] {
                "fish" => println!("set -p PATH \"{}\"", env.base_dir),
                "bash" | "sh" | "zsh" => println!("export PATH=\"{}:$PATH\"", env.base_dir),
                _ => panic!("{} is not a known shell", shell),
            }
        }
        Actions::Sync => {
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            println!("Syncing versions with {} configuration.", env.name);
            cli_actions::sync_tools(&mut env, &system).await?;
        }
    };
    Ok(())
}
