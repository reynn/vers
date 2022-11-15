use {
    super::Archiver,
    async_trait::async_trait,
    flate2::read::GzDecoder,
    std::{fs::File, path::Path},
    tracing::debug,
};

pub struct GzipArchiver;

#[async_trait]
impl Archiver for GzipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> crate::Result<()> {
        let out_file_path = out_dir.join(file_path.file_stem().unwrap());
        let mut gz = flate2::read::MultiGzDecoder::new(File::open(file_path)?);

        match gz.get_ref().metadata() {
            Ok(metadata) => debug!("metadata: {:?}", metadata),
            Err(_) => anyhow::bail!("Failed to get Gzip metadata"),
        }

        let mut out_file = File::create(&out_file_path)?;

        std::io::copy(&mut gz, &mut out_file)?;

        super::TarArchiver
            .extract_to(&out_file_path, out_dir)
            .await?;
        std::fs::remove_file(&out_file_path)?;

        Ok(())
    }

    async fn extract(&self, file_path: &'_ Path) -> crate::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }

    fn can_handle(&self, file_path: &'_ Path) -> bool {
        match std::fs::File::open(file_path) {
            Ok(file) => GzDecoder::new(file).header().is_some(),
            Err(_) => false,
        }
    }
    fn name(&self) -> &'static str {
        "gzip"
    }
}
