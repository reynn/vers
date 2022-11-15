use {
    super::Archiver,
    async_trait::async_trait,
    std::{fs::File, path::Path},
    tar::Archive,
    tracing::debug,
};

pub struct TarArchiver;

#[async_trait]
impl Archiver for TarArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> crate::Result<()> {
        let file = File::open(file_path)?;
        let mut archive = Archive::new(file);

        for entry in archive.entries()? {
            let mut file = entry?;

            let out_file_name = file.path()?;
            debug!(
                "Extracting '{}' from archive '{}'",
                out_file_name.display(),
                file_path.display()
            );
            // Write the file to the specified path
            file.unpack_in(out_dir)?;
        }

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> crate::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        match std::fs::File::open(file_path) {
            Ok(file) => tar::Archive::new(file).entries().is_ok(),
            Err(_) => false,
        }
    }
    fn name(&self) -> &'static str {
        "tar"
    }
}
