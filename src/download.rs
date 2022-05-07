use {
    log::*,
    octocrab::models::repos::Asset,
    reqwest::{IntoUrl, Url},
    std::path::PathBuf,
    tokio::fs::{copy, create_dir_all, write, File},
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
            Err(create_dirs_err) => return Err(create_dirs_err.into()),
        };
    };
    let out_file = match File::create(&out_file_name).await {
        Ok(file) => file,
        Err(file_create_err) => return Err(file_create_err.into()),
    };
    match reqwest::get(asset.browser_download_url.as_str()).await {
        Ok(cd) => match cd.bytes().await {
            Ok(bytes) => match write(&out_file_name, bytes).await {
                Ok(_) => Ok(out_file_name),
                Err(write_err) => Err(write_err.into()),
            },
            Err(bytes_err) => Err(bytes_err.into()),
        },
        Err(down_err) => Err(down_err.into()),
    }
}
