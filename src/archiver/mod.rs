use self::{gzip::GzipArchiver, tar::TarArchiver, tar_gzip::TarGzipArchiver, zip::ZipArchiver};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod gzip;
mod tar;
mod tar_gzip;
mod zip;

#[derive(Debug, Error)]
pub enum ArchiverError {
    #[error("IO Error accessing {file_path}. {source:?}")]
    IoError {
        file_path: PathBuf,
        source: std::io::Error,
    },
    #[error("Extractor({extractor}) was unable to load '{file_path}'. {message}")]
    ExtractorLoadError {
        extractor: String,
        file_path: PathBuf,
        message: String,
    },
    #[error("Extractor({extractor}) encountered an error {message}")]
    ExtractorError { extractor: String, message: String },
    #[error("Unable to create directory {file_path}. {source:?}")]
    DirectorCreateError {
        file_path: PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to create file {file_path}. {source:?}")]
    FileCreateError {
        file_path: PathBuf,
        source: std::io::Error,
    },
}

type Result<T, E = ArchiverError> = std::result::Result<T, E>;

#[async_trait]
pub trait Archiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> Result<()>;
    async fn extract(&self, file_path: &'_ Path) -> Result<()>;
    fn can_handle(&self, file_path: &'_ Path) -> bool;
    fn name(&self) -> &'static str;
}

pub fn determine_possible_extractors(file_path: &'_ Path) -> Vec<Box<dyn Archiver>> {
    let possible_extractors: Vec<Box<dyn Archiver>> = vec![
        Box::new(TarGzipArchiver),
        Box::new(TarArchiver),
        Box::new(ZipArchiver),
        Box::new(GzipArchiver),
    ];

    possible_extractors
        .into_iter()
        .filter(|e| e.can_handle(file_path))
        .collect()
}

pub async fn handle_file_extraction(
    archiver: Box<dyn Archiver>,
    input_file: &'_ Path,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    if let Some(out_dir) = output_dir {
        archiver.extract_to(input_file, &out_dir).await
    } else {
        archiver.extract(input_file).await
    }
}
