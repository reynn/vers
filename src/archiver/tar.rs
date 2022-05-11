use {
    super::Archiver,
    async_trait::async_trait,
    log::*,
    std::{path::Path, process::Stdio},
    tokio::process::Command,
};

pub struct TarArchiver;

#[async_trait]
impl Archiver for TarArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> crate::Result<()> {
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
            Err(io_err) => eyre::bail!(io_err),
        }
    }

    async fn extract(&self, file_path: &'_ Path) -> crate::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }
}
