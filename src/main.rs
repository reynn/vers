// Turn off common dev assertions only for debug builds, release builds will still work as normal
#![warn(clippy::all)]

use tracing::{debug, info};
use tracing_subscriber::{filter::filter_fn, prelude::*};
use vers::{cli::Opts, dirs};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opts::default();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            opts.verbose.log_level_filter().to_string(),
        ))
        .with(
            tracing_subscriber::fmt::layer().with_filter(filter_fn(|metadata| {
                // Only log from this crate only
                metadata.target().starts_with(env!("CARGO_PKG_NAME"))
            })),
        )
        .init();

    let config_dir: std::path::PathBuf = match opts.data_dir {
        Some(dir) => dir,
        None => dirs::get_default_config_path(),
    };
    debug!("Config dir: {}", &config_dir.display());

    // initialize Octocrab with a configured GitHub API token if available
    if let Some(api_token) = opts.github_api_token {
        info!("Initializing the GitHub client with token from CLI args");
        octocrab::initialise(octocrab::Octocrab::builder().personal_token(api_token))?;
    } else if let Some(env_api_token) = std::env::var_os("GITHUB_TOKEN") {
        info!("Initializing the GitHub client with token from environment");
        octocrab::initialise(
            octocrab::Octocrab::builder().personal_token(env_api_token.to_str().unwrap().into()),
        )?;
    };

    // Run the main logic
    opts.action.execute(config_dir, opts.env).await?;

    Ok(())
}
