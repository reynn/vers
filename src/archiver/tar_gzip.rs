use super::{Archiver, Result};
use async_trait::async_trait;
use std::path::Path;

pub struct TarGzipArchiver;

#[async_trait()]
impl Archiver for TarGzipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> Result<()> {
        let gzip_archiver = super::GzipArchiver;
        let tar_archiver = super::TarArchiver;
        tracing::debug!(
            "Extracting {} using the 'TarGzip' Archiver",
            file_path.display()
        );

        gzip_archiver.extract_to(file_path, out_dir).await?;
        tar_archiver
            .extract_to(
                out_dir.join(file_path.file_stem().unwrap()).as_path(),
                out_dir,
            )
            .await?;
        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        file_path
            .file_name()
            .map(|f| f.to_str().unwrap_or_default().ends_with(".tar.gz"))
            .unwrap_or_default()
    }

    fn name(&self) -> &'static str {
        "tar-gzip"
    }
}
