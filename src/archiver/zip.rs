use {super::Archiver, async_trait::async_trait, std::path::Path, tracing::debug, zip::ZipArchive};

pub struct ZipArchiver;

#[async_trait]
impl Archiver for ZipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> crate::Result<()> {
        let mut archive = ZipArchive::new(std::fs::File::open(file_path)?)?;

        for i in 0..archive.len() {
            let mut archive_file = archive.by_index(i)?;
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
                std::fs::create_dir_all(out_path)?;
            } else {
                // write file
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut archive_file, &mut out_file)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    if let Some(mode) = archive_file.unix_mode() {
                        std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode))?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> crate::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        match std::fs::File::open(file_path) {
            Ok(file) => ZipArchive::new(file).is_ok(),
            Err(_) => false,
        }
    }

    fn name(&self) -> &'static str {
        "zip"
    }
}
