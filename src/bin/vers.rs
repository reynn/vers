use {
    tracing::{debug, info},
    tracing_subscriber::{filter::filter_fn, prelude::*},
    vers::{cli::Opts, dirs},
};

#[async_std::main]
async fn main() -> vers::Result<()> {
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

    let config_dir: std::path::PathBuf = if let Some(dir) = opts.data_dir {
        dir
    } else {
        dirs::get_default_config_path()
    };
    debug!("Config dir: {}", &config_dir.display());

    if let Some(api_token) = opts.github_api_token {
        info!("Initializing the GitHub client with token from CLI args");
        octocrab::initialise(octocrab::Octocrab::builder().personal_token(api_token))?;
    } else if let Some(env_api_token) = std::env::var_os("GITHUB_TOKEN") {
        info!("Initializing the GitHub client with token from environment");
        octocrab::initialise(
            octocrab::Octocrab::builder().personal_token(env_api_token.to_str().unwrap().into()),
        )?;
    };

    opts.action.execute(config_dir, opts.env).await?;

    Ok(())
}
