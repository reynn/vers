use {
    super::Archiver,
    async_trait::async_trait,
    log::*,
    std::{path::Path, path::PathBuf, process::Stdio},
    tokio::process::Command,
};

pub struct TarArchiver;

#[async_trait]
impl Archiver for TarArchiver {
    async fn extract_to(&self, file_path: &'_ PathBuf, out_dir: &'_ PathBuf) -> crate::Result<()> {
        let mut cmd = Command::new("tar");
        cmd.stdout(Stdio::null());
        cmd.args(&[
            "-x",
            "-f",
            file_path.to_str().unwrap_or_default(),
            "-C",
            out_dir.to_str().unwrap_or_default(),
        ]);

        match cmd.output().await {
            Ok(output) => {
                debug!("`tar` command {:?}. output: {:?}", &cmd, output);
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
