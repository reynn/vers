use {async_trait::async_trait, std::path::PathBuf};

mod tar;
mod zip;

#[async_trait]
pub trait Archiver {
    async fn extract<P: Into<PathBuf> + Send>(
        file_path: P,
        out_dir: Option<P>,
    ) -> crate::Result<()>;
    async fn compress<P: Into<PathBuf> + Send>(
        file_dir: P,
        out_file: Option<P>,
    ) -> crate::Result<PathBuf>;
}
