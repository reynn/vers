use {super::Archiver, async_trait::async_trait, std::path::PathBuf};

pub struct TarArchiver;

#[async_trait]
impl Archiver for TarArchiver {
    async fn extract<P: Into<PathBuf> + Send>(
        file_path: P,
        out_dir: Option<P>,
    ) -> crate::Result<()> {
        unimplemented!()
    }
    async fn compress<P: Into<PathBuf> + Send>(
        file_dir: P,
        out_file: Option<P>,
    ) -> crate::Result<PathBuf> {
        unimplemented!()
    }
}
