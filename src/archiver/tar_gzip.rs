use super::{Archiver, ArchiverError, Result};
use async_trait::async_trait;
use std::path::Path;

pub struct TarGzipArchiver;

#[async_trait()]
impl Archiver for TarGzipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> Result<()> {
        tracing::debug!(
            "Extracting {} using the 'TarGzip' Archiver",
            file_path.display()
        );

        let file = std::fs::File::open(file_path).map_err(|source| ArchiverError::Io {
            file_path: file_path.to_path_buf(),
            source,
        })?;
        let decoder = flate2::read::MultiGzDecoder::new(file);
        tar::Archive::new(decoder)
            .unpack(out_dir)
            .map_err(|source| ArchiverError::Io {
                file_path: file_path.to_path_buf(),
                source,
            })
    }

    async fn extract(&self, file_path: &'_ Path) -> Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        file_path
            .file_name()
            .map(|f| {
                let name = f.to_str().unwrap_or_default();
                name.ends_with(".tar.gz") || name.ends_with(".tgz")
            })
            .unwrap_or_default()
    }

    fn name(&self) -> &'static str {
        "tar-gzip"
    }
}
