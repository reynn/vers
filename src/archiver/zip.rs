use {
    super::Archiver,
    async_trait::async_trait,
    std::{path::Path, path::PathBuf, process::Stdio},
    tokio::process::Command,
};

pub struct ZipArchiver;

#[async_trait]
impl Archiver for ZipArchiver {
    async fn extract_to(&self, file_path: &'_ PathBuf, out_dir: &'_ PathBuf) -> crate::Result<()> {
        let mut cmd = Command::new("unzip");
        cmd.stdout(Stdio::null());
        cmd.args(&[
            "-oq",
            file_path.to_str().unwrap_or_default(),
            "-d",
            out_dir.to_str().unwrap_or_default(),
        ]);

        match cmd.output().await {
            Ok(output) => {
                log::debug!("`zip` command {:?}. output: {:?}", &cmd, output);
                Ok(())
            }
            Err(io_err) => Err(io_err.into()),
        }
    }

    async fn extract(&self, file_path: &'_ PathBuf) -> crate::Result<()> {
        self.extract_to(file_path, &Path::new(".").to_path_buf())
            .await
    }
}
