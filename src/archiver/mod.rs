use {
    self::{tar::TarArchiver, zip::ZipArchiver},
    crate::Result,
    async_trait::async_trait,
    log::info,
    once_cell::sync::Lazy,
    regex::Regex,
    std::path::{Path, PathBuf},
};

mod tar;
mod zip;

static ARCHIVE_TYPE_TAR_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^.+\.tar\..*?"#).expect("Unable to compile regex for tar archiver"));
static ARCHIVE_TYPE_ZIP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"^.+\.zip"#).expect("Unable to compile regex for zip archiver"));

#[async_trait]
pub trait Archiver {
    async fn extract_to(&self, file_path: &'_ PathBuf, out_dir: &'_ PathBuf) -> crate::Result<()>;
    async fn extract(&self, file_path: &'_ PathBuf) -> crate::Result<()>;
}

pub fn determine_extractor(file_path: &'_ PathBuf) -> Option<Box<dyn Archiver>> {
    let file_path_str = file_path.to_str().unwrap_or_default();
    if ARCHIVE_TYPE_TAR_REGEX.is_match(file_path_str) {
        info!("Tar Extractor for {}", file_path_str);
        Some(Box::new(TarArchiver))
    } else if ARCHIVE_TYPE_ZIP_REGEX.is_match(file_path_str) {
        info!("Zip Extractor for {}", file_path_str);
        Some(Box::new(ZipArchiver))
    } else {
        info!("No extractor found for {}", file_path_str);
        None
    }
}

pub async fn handle_file_extraction(
    archiver: Box<dyn Archiver>,
    input_file: &'_ PathBuf,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    if let Some(out_dir) = output_dir {
        archiver.extract_to(&input_file, &out_dir).await
    } else {
        archiver.extract(&input_file).await
    }
}
