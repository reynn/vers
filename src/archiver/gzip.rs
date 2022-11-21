use super::{Archiver, ArchiverError};
use async_trait::async_trait;
use std::{fs::File, path::Path};

pub struct GzipArchiver;

#[async_trait]
impl Archiver for GzipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> super::Result<()> {
        tracing::debug!(
            "Extracting {} using the 'Gzip' Archiver",
            file_path.display()
        );
        let out_file_path = out_dir.join(file_path.file_stem().unwrap());
        let mut gz =
            flate2::read::MultiGzDecoder::new(File::open(file_path).map_err(|open_err| {
                ArchiverError::IoError {
                    file_path: file_path.to_path_buf(),
                    source: open_err,
                }
            })?);

        let mut out_file =
            File::create(&out_file_path).map_err(|create_err| ArchiverError::FileCreateError {
                file_path: out_file_path.to_path_buf(),
                source: create_err,
            })?;

        std::io::copy(&mut gz, &mut out_file).map_err(|copy_err| ArchiverError::IoError {
            file_path: out_file_path.clone(),
            source: copy_err,
        })?;

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> super::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        file_path
            .file_name()
            .map(|f| f.to_str().unwrap_or_default().ends_with(".tar.gz"))
            .unwrap_or_default()
    }

    fn name(&self) -> &'static str {
        "gzip"
    }
}
