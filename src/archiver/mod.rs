use {
    self::{gzip::GzipArchiver, tar::TarArchiver, zip::ZipArchiver},
    crate::Result,
    async_trait::async_trait,
    std::path::{Path, PathBuf},
};

mod gzip;
mod tar;
mod zip;

#[async_trait]
pub trait Archiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> crate::Result<()>;
    async fn extract(&self, file_path: &'_ Path) -> crate::Result<()>;
    fn can_handle(&self, file_path: &'_ Path) -> bool;
    fn name(&self) -> &'static str;
}

pub fn determine_possible_extractors(file_path: &'_ Path) -> Vec<Box<dyn Archiver>> {
    let possible_extractors: Vec<Box<dyn Archiver>> = vec![
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
