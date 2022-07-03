use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn extract_archive<P: Into<PathBuf>>(
    archive_path: P,
    out_directory: Option<P>,
) -> crate::Result<()> {
    let archive_path: PathBuf = archive_path.into();
    let out_directory: PathBuf = if let Some(out_dir) = out_directory {
        out_dir.into()
    } else {
        Path::new(".").into()
    };

    let cmd_result = match archive_path.to_str().unwrap_or_default() {
        x if x.ends_with(".tar.gz") => {
            log::debug!("extracting with `tar`");
            Command::new("tar")
                .args(&[
                    "x",
                    "f",
                    x,
                    "-C",
                    out_directory.to_str().unwrap_or_default(),
                ])
                .output()?
        }
        x if x.ends_with(".gz") => {
            log::debug!("extracting with `gunzip`");
            Command::new("gunzip").args(&["-c", x]).output()?
        }
        x if x.ends_with(".zip") => {
            log::debug!("extracting with `zip`");
            Command::new("unzip")
                .args(&[
                    "-q",
                    "-o",
                    x,
                    "-d",
                    out_directory.to_str().unwrap_or_default(),
                ])
                .output()?
        }
        _ => eyre::bail!("{:?} is not a known archive type.", archive_path),
    };

    log::debug!("Result of extract command: {:?}", cmd_result);
    Ok(())
}
