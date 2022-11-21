use async_std::fs::{create_dir_all, write};
use octocrab::models::repos::Asset;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("Failed to create directory '{dir}'. {source}")]
    CreateDirectoryError {
        dir: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("...")]
    FileWriteError {
        file_path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("...")]
    ResponseBytesError(#[from] reqwest::Error),
    #[error("...")]
    HttpGetError {
        url: reqwest::Url,
        source: reqwest::Error,
    },
}

/// Download a file from a provided URL
pub async fn download_asset<P: Into<PathBuf>>(
    asset: &'_ Asset,
    out_dir: P,
) -> Result<PathBuf, DownloadError> {
    let out_file_name: PathBuf = out_dir.into();
    let out_file_name = out_file_name.join(&asset.name);
    info!("Downloading file to {:?}", &out_file_name);
    if let Some(out_parent) = &out_file_name.parent() {
        match create_dir_all(out_parent).await {
            Ok(_) => {}
            Err(create_dirs_err) => {
                return Err(DownloadError::CreateDirectoryError {
                    dir: out_parent.to_path_buf(),
                    source: create_dirs_err,
                })
            }
        };
    };
    match reqwest::get(asset.browser_download_url.as_str()).await {
        Ok(download_result) => match download_result.bytes().await {
            Ok(download_bytes) => match write(&out_file_name, download_bytes).await {
                Ok(_) => Ok(out_file_name),
                Err(write_err) => Err(DownloadError::FileWriteError {
                    file_path: out_file_name,
                    source: write_err,
                }),
            },
            Err(bytes_err) => Err(DownloadError::ResponseBytesError(bytes_err)),
        },
        Err(down_err) => Err(DownloadError::HttpGetError {
            url: asset.browser_download_url.to_owned(),
            source: down_err,
        }),
    }
}
