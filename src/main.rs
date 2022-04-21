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
    log::{info, LevelFilter},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
    info!("{:#?}", opts);
    // let actions = cli::actions().run();
    match opts.action {
        cli::Actions::Add {
            name,
            alias,
            pattern,
            filter,
            pre_release,
        } => {
            info!("Adding the {} tool to the {} environment", name, &opts.env);
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli::add_new_tool(&name, &system, &mut env, pattern).await?;
        }
        cli::Actions::Remove { name, all } => {
            info!("Removing {} from the {} environment", name, &opts.env);
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli::remove_tool(&name, &env).await?;
        }
        cli::Actions::List {
            installed,
            known,
            current,
        } => {
            info!("Listing tools in the {} environment", &opts.env);
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli::list_tools(&env).await?;
        }
    }
    Ok(())
}
