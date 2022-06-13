use {
    cargo_util::{ProcessBuilder, ProcessError},
    std::path::{Path, PathBuf},
    vers::dirs,
};

mod cli;
mod commands;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let args = cli::cli().get_matches();

    env_logger::builder()
        .filter_level(match args.occurrences_of("verbose") {
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            3 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Warn,
        })
        .init();

    let config_dir: PathBuf = if let Some(config_dir) = args.value_of("config_dir") {
        config_dir.into()
    } else {
        vers::dirs::get_default_config_path()
    };

    log::info!("Loading config from {:?}", config_dir);

    Ok(())
}

fn execute_external_subcommand(cmd: &str, args: &[&str]) -> eyre::Result<()> {
    let path = find_external_subcommand(cmd);
    let command = match path {
        Some(command) => command,
        None => {
            eyre::bail!(format!("no such subcommand: `{}`", cmd))
        }
    };

    let err = match ProcessBuilder::new(&command).args(args).exec_replace() {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    if let Some(perr) = err.downcast_ref::<ProcessError>() {
        if let Some(code) = perr.code {
            eyre::bail!("Error code {}", code)
        }
    }

    eyre::bail!(err)
}

fn find_external_subcommand(cmd: &str) -> Option<PathBuf> {
    let command_exe = format!("cargo-{}{}", cmd, std::env::consts::EXE_SUFFIX);
    search_directories()
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file))
}

fn search_directories() -> Vec<PathBuf> {
    let config_dir = dirs::get_default_config_path();
    let mut dirs = vec![config_dir.join("bin")];
    if let Some(val) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&val));
    }
    dirs
}

#[cfg(unix)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::prelude::*;
    std::fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
#[cfg(windows)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}
