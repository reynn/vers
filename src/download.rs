use {
    async_std::fs::{create_dir_all, write},
    octocrab::models::repos::Asset,
    std::path::PathBuf,
    tracing::info,
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
            Err(create_dirs_err) => anyhow::bail!(create_dirs_err),
        };
    };
    match reqwest::get(asset.browser_download_url.as_str()).await {
        Ok(download_result) => match download_result.bytes().await {
            Ok(download_bytes) => match write(&out_file_name, download_bytes).await {
                Ok(_) => Ok(out_file_name),
                Err(write_err) => anyhow::bail!(write_err),
            },
            Err(bytes_err) => anyhow::bail!(bytes_err),
        },
        Err(down_err) => anyhow::bail!(down_err),
    }
}
