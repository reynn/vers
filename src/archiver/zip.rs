use super::{Archiver, ArchiverError};
use async_trait::async_trait;
use std::path::Path;
use tracing::debug;
use zip::ZipArchive;

pub struct ZipArchiver;

#[async_trait]
impl Archiver for ZipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> super::Result<()> {
        tracing::debug!(
            "Extracting {} using the 'Zip' Archiver",
            file_path.display()
        );
        let mut archive = ZipArchive::new(std::fs::File::open(file_path).map_err(|e| {
            ArchiverError::IoError {
                file_path: file_path.to_path_buf(),
                source: e,
            }
        })?)
        .map_err(|zip_err| ArchiverError::ExtractorLoadError {
            extractor: "zip".to_string(),
            file_path: file_path.to_path_buf(),
            message: zip_err.to_string(),
        })?;

        for i in 0..archive.len() {
            let mut archive_file = archive.by_index(i).unwrap();
            let out_path = match archive_file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            let out_path = out_dir.join(out_path);
            debug!(
                "Extracting '{}' from archive '{}'",
                out_path.display(),
                file_path.display()
            );
            if archive_file.is_dir() {
                // create directory
                std::fs::create_dir_all(&out_path).map_err(|create_err| {
                    ArchiverError::DirectorCreateError {
                        file_path: out_path,
                        source: create_err,
                    }
                })?;
            } else {
                // write file
                let mut out_file = std::fs::File::create(&out_path).map_err(|file_create_err| {
                    ArchiverError::FileCreateError {
                        file_path: out_path.clone(),
                        source: file_create_err,
                    }
                })?;
                std::io::copy(&mut archive_file, &mut out_file).map_err(|copy_err| {
                    ArchiverError::IoError {
                        file_path: out_path.clone(),
                        source: copy_err,
                    }
                })?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    if let Some(mode) = archive_file.unix_mode() {
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))
                            .map_err(|permission_err| ArchiverError::IoError {
                                file_path: out_path,
                                source: permission_err,
                            })?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> super::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        file_path
            .file_name()
            .map(|f| f.to_str().unwrap_or_default().ends_with(".zip"))
            .unwrap_or_default()
        // match std::fs::File::open(file_path) {
        //     Ok(file) => ZipArchive::new(file).is_ok(),
        //     Err(_) => false,
        // }
    }

    fn name(&self) -> &'static str {
        "zip"
    }
}
