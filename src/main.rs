// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]
#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_macros, unused_imports, unused_variables)
)]

mod archiver;
mod cli;
mod dirs;
mod download;
mod environment;
mod github;
mod shell;
mod system;
mod tool;
mod version;

use {
    crate::{cli::Actions, environment::Environment, system::System},
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
    match opts.action {
        Actions::Add {
            name,
            alias,
            pattern,
            file_pattern,
            filter,
            pre_release,
            show,
        } => {
            info!("Adding the {} tool to the {} environment", name, &opts.env);
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli::add_new_tool(&name, &system, &mut env, pattern, file_pattern, alias, show).await?;
        }
        Actions::Remove { name, all } => {
            info!("Removing {} from the {} environment", name, &opts.env);
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli::remove_tool(&name, &env).await?;
        }
        Actions::List {
            installed,
            known,
            current,
        } => {
            info!("Listing tools in the {} environment", &opts.env);
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli::list_tools(&env).await?;
        }
        Actions::Update { all, name } => {
            let env = Environment::load(&config_dir, &opts.env).await?;
            if let Some(name) = name {
                info!("Updating {} in {} to latest version", name, env.name)
            } else {
                info!("Updating all tools in {} to the latest version", env.name)
            };
        }
        Actions::Env { name, shell } => {
            let name = if let Some(name) = name {
                name
            } else {
                opts.env
            };
            let env = Environment::load(&config_dir, &name).await?;
            match &shell[..] {
                "fish" => println!("set -p PATH \"{}\"", env.base_dir),
                "bash" | "sh" | "zsh" => println!("export PATH=\"{}:$PATH\"", env.base_dir),
                _ => panic!("{} is not a known shell", shell),
            }
        }
    }
    Ok(())
}
