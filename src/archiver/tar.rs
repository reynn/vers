use super::{Archiver, ArchiverError};
use async_trait::async_trait;
use std::{fs::File, path::Path};
use tar::Archive;
use tracing::debug;

pub struct TarArchiver;

#[async_trait]
impl Archiver for TarArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> super::Result<()> {
        tracing::debug!(
            "Extracting {} using the 'Tar' Archiver",
            file_path.display()
        );
        let file = File::open(file_path).map_err(|open_err| ArchiverError::Io {
            file_path: file_path.to_path_buf(),
            source: open_err,
        })?;
        let mut archive = Archive::new(file);

        if let Ok(entries) = archive.entries() {
            for entry in entries {
                match entry {
                    Ok(mut file) => {
                        let out_file_name = file.path().unwrap();
                        debug!(
                            "Extracting '{}' from archive '{}'",
                            out_file_name.display(),
                            file_path.display()
                        );
                        // Write the file to the specified path
                        file.unpack_in(out_dir).map_err(|unpack_err| {
                            ArchiverError::ExtractorError {
                                extractor: "tar".to_string(),
                                message: unpack_err.to_string(),
                            }
                        })?;
                    }
                    Err(entry_err) => {
                        return Err(ArchiverError::ExtractorError {
                            extractor: "tar".to_string(),
                            message: format!(
                                "Unable to extract file from archive {}. {:?}",
                                file_path.display(),
                                entry_err,
                            ),
                        })
                    }
                }
            }
        } else {
            return Err(ArchiverError::ExtractorError {
                extractor: "tar".to_string(),
                message: format!(
                    "Unable to get archiver entries from {}",
                    file_path.display()
                ),
            });
        }

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> super::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        file_path
            .file_name()
            .map(|f| f.to_str().unwrap_or_default().ends_with(".tar"))
            .unwrap_or_default()
    }

    fn name(&self) -> &'static str {
        "tar"
    }
}
