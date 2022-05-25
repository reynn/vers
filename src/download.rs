use {
    crate::manager::Asset,
    log::*,
    std::path::PathBuf,
    tokio::fs::{create_dir_all, write},
};

/// Download a file from a provided URL
pub async fn download_asset<P: Into<PathBuf>>(
    asset: &'_ Asset,
    out_dir: P,
) -> crate::Result<PathBuf> {
    let out_file_name: PathBuf = out_dir.into();
    let out_file_name = out_file_name.join(&asset.name);
    info!("Downloading file to {:?}", &out_file_name);
    if let Some(out_parent) = &out_file_name.parent() {
        match create_dir_all(out_parent).await {
            Ok(_) => {}
            Err(create_dirs_err) => eyre::bail!(create_dirs_err),
        };
    };
    match reqwest::get(asset.download_url.as_str()).await {
        Ok(cd) => match cd.bytes().await {
            Ok(bytes) => match write(&out_file_name, bytes).await {
                Ok(_) => Ok(out_file_name),
                Err(write_err) => eyre::bail!(write_err),
            },
            Err(bytes_err) => eyre::bail!(bytes_err),
        },
        Err(down_err) => eyre::bail!(down_err),
    }
}
