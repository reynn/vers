#![warn(clippy::all)]

use tracing::{debug, info};
use tracing_subscriber::{filter::filter_fn, prelude::*};
use vers::{cli::Cli, dirs};

#[async_std::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let opts = Cli::default();

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
    if let Some(api_token) = opts.github_token {
        info!("Initializing the GitHub client with token from CLI args");
        octocrab::initialise(
            octocrab::Octocrab::builder()
                .personal_token(api_token)
                .build()?,
        );
    } else if let Ok(env_api_token) = std::env::var("GITHUB_TOKEN") {
        info!("Initializing the GitHub client with token from environment");
        octocrab::initialise(
            octocrab::Octocrab::builder()
                .personal_token(env_api_token)
                .build()?,
        );
    };

    // Run the main logic
    opts.action.execute(config_dir, &opts.env).await?;

    Ok(())
}
