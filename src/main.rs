// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]

mod archiver;
mod cli;
mod cli_actions;
mod dirs;
mod download;
mod environment;
mod github;
mod system;
mod tool;
mod version;

use {
    crate::{
        cli::Actions,
        cli_actions::{Patterns, UpdateType},
        environment::Environment,
        system::System,
    },
    log::*,
};

pub type Result<T> = eyre::Result<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = cli::Opts::new();

    env_logger::builder()
        .filter_level(opts.verbose.log_level_filter())
        .init();

    let config_dir: std::path::PathBuf = if let Some(dir) = opts.data_dir {
        dir
    } else {
        dirs::get_default_config_path()
    };

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
            asset_pattern,
            file_filter,
            pre_release,
            show,
        } => {
            debug!("CLI: Name `{name}`, alias `{:?}`, pattern `{:?}`, filter `{:?}`, pre_release `{pre_release}`, show `{show}`", alias, asset_pattern, file_filter);
            let system = System::new();
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::add_new_tool(
                &mut env,
                &name,
                &system,
                Patterns {
                    asset: asset_pattern,
                    file: file_filter,
                },
                alias,
                show,
                pre_release,
            )
            .await?;
        }
        Actions::Remove {
            name,
            all,
            link_only: _link_only,
        } => {
            let mut env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::remove_tool(&mut env, &name, all).await?;
        }
        Actions::List { installed, output } => {
            let env = Environment::load(&config_dir, &opts.env).await?;
            cli_actions::list_tools(&env, installed, output).await?;
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
        Actions::Completions { shell } => {
            cli_actions::generate_completions(shell);
        }
        Actions::Env {
            name,
            shell,
            bare_path,
        } => {
            let name = match name {
                Some(name) => name,
                None => opts.env,
            };
            let env = Environment::load(&config_dir, &name).await?;
            if bare_path {
                println!("{}", env.base_dir)
            } else {
                match &shell[..] {
                    "fish" => println!("set -p PATH \"{}\"", env.base_dir),
                    "bash" | "sh" | "zsh" => println!("export PATH=\"{}:$PATH\"", env.base_dir),
                    _ => panic!("{} is not a known shell", shell),
                }
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
