use {
    super::Archiver,
    async_trait::async_trait,
    log::*,
    std::{path::Path, process::Stdio},
    tokio::process::Command,
};

pub struct ZipArchiver;

#[async_trait]
impl Archiver for ZipArchiver {
    async fn extract_to(&self, file_path: &'_ Path, out_dir: &'_ Path) -> eyre::Result<()> {
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
                debug!("`zip` command {:?}. output: {:?}", &cmd, output);
                Ok(())
            }
            Err(io_err) => eyre::bail!(io_err),
        }
    }

    async fn extract(&self, file_path: &'_ Path) -> eyre::Result<()> {
        self.extract_to(file_path, Path::new(".")).await
    }
}
