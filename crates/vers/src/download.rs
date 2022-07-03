use {
    log::*,
    octocrab::models::repos::Asset,
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

    if let Some(out_parent) = &out_file_name.parent() {
        if let Err(create_dirs_err) = create_dir_all(out_parent).await {
            eyre::bail!(create_dirs_err)
        }
    };

    info!("Downloading file to {:?}", &out_file_name);
    let downloaded = reqwest::get(asset.browser_download_url.as_str()).await?;
    let bytes = downloaded.bytes().await?;
    write(&out_file_name, bytes).await?;

    Ok(out_file_name)
}
