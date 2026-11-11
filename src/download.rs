use async_std::io::WriteExt;
use indicatif::{ProgressBar, ProgressStyle};
use octocrab::models::repos::Asset;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;
use tracing::{debug, info, warn};

/// How long to wait while establishing the connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Upper bound for the complete request, including streaming the body.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// Total number of download attempts, including the initial one.
const MAX_ATTEMPTS: u32 = 3;
/// Base delay between retries; the delay doubles after each failed attempt.
const RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("Failed to create directory '{dir}'. {source}")]
    CreateDirectoryFailed {
        dir: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to write file '{file_path}'. {source}")]
    FileWrite {
        file_path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to move the downloaded file into place at '{file_path}'. {source}")]
    FileRename {
        file_path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to configure the HTTP client. {source}")]
    ClientBuild { source: reqwest::Error },
    #[error("Failed to get '{url}': {source}")]
    HttpGet {
        url: reqwest::Url,
        source: reqwest::Error,
    },
    #[error("HTTP request to '{url}' failed with status {status}")]
    HttpStatus {
        url: reqwest::Url,
        status: reqwest::StatusCode,
    },
    #[error("Failed while reading the response body from '{url}': {source}")]
    ResponseChunk {
        url: reqwest::Url,
        source: reqwest::Error,
    },
}

impl DownloadError {
    /// Whether retrying the download may reasonably succeed.
    fn is_retryable(&self) -> bool {
        match self {
            // Connection, timeout, and body errors are transient. Builder and
            // redirect errors will fail the same way every time.
            DownloadError::HttpGet { source, .. } => !source.is_builder() && !source.is_redirect(),
            DownloadError::HttpStatus { status, .. } => {
                status.is_server_error()
                    || *status == reqwest::StatusCode::REQUEST_TIMEOUT
                    || *status == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            // A stream that fails part-way through may succeed on a fresh request.
            DownloadError::ResponseChunk { .. } => true,
            // Local filesystem and client configuration failures are not transient.
            DownloadError::CreateDirectoryFailed { .. }
            | DownloadError::FileWrite { .. }
            | DownloadError::FileRename { .. }
            | DownloadError::ClientBuild { .. } => false,
        }
    }
}

/// Download a file from a provided URL.
pub async fn download_asset<P: Into<PathBuf>>(
    asset: &'_ Asset,
    out_dir: P,
) -> Result<PathBuf, DownloadError> {
    download_url(
        asset.browser_download_url.clone(),
        &asset.name,
        out_dir.into(),
    )
    .await
}

/// Stream `url` to `out_dir/file_name`, retrying transient failures.
async fn download_url(
    url: reqwest::Url,
    file_name: &'_ str,
    out_dir: PathBuf,
) -> Result<PathBuf, DownloadError> {
    let out_file = out_dir.join(file_name);
    info!("Downloading '{}' to {:?}", url, out_file);

    if let Some(out_parent) = out_file.parent() {
        async_std::fs::create_dir_all(out_parent)
            .await
            .map_err(|source| DownloadError::CreateDirectoryFailed {
                dir: out_parent.to_path_buf(),
                source,
            })?;
    }

    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|source| DownloadError::ClientBuild { source })?;

    // Write to a sibling temporary file and rename it into place only after the
    // whole body has been received, so a failed attempt never leaves a corrupt
    // target file behind.
    let partial_file = partial_path(&out_file);
    let mut attempt = 0;

    loop {
        attempt += 1;
        if attempt > 1 {
            let delay = RETRY_BASE_DELAY * 2u32.pow(attempt - 2);
            warn!(
                "Retrying download of '{}' in {:?} (attempt {}/{})",
                url, delay, attempt, MAX_ATTEMPTS
            );
            async_std::task::sleep(delay).await;
        }

        match download_to_file(&client, &url, &partial_file, file_name).await {
            Ok(()) => {
                if let Err(source) = async_std::fs::rename(&partial_file, &out_file).await {
                    let _ = async_std::fs::remove_file(&partial_file).await;
                    return Err(DownloadError::FileRename {
                        file_path: out_file,
                        source,
                    });
                }
                info!("Downloaded '{}' to {:?}", url, out_file);
                return Ok(out_file);
            }
            Err(err) => {
                let _ = async_std::fs::remove_file(&partial_file).await;
                if !err.is_retryable() || attempt >= MAX_ATTEMPTS {
                    return Err(err);
                }
                debug!("Download attempt {} for '{}' failed: {}", attempt, url, err);
            }
        }
    }
}

/// Perform a single download attempt, streaming the response body to disk.
async fn download_to_file(
    client: &'_ reqwest::Client,
    url: &'_ reqwest::Url,
    file_path: &'_ Path,
    label: &'_ str,
) -> Result<(), DownloadError> {
    let mut response =
        client
            .get(url.clone())
            .send()
            .await
            .map_err(|source| DownloadError::HttpGet {
                url: url.clone(),
                source,
            })?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadError::HttpStatus {
            url: url.clone(),
            status,
        });
    }

    let mut file = async_std::fs::File::create(file_path)
        .await
        .map_err(|source| DownloadError::FileWrite {
            file_path: file_path.to_path_buf(),
            source,
        })?;

    let progress = download_progress(label, response.content_length());
    let result = stream_response(&mut response, &mut file, &progress, url, file_path).await;
    if result.is_ok() {
        // Leave a completed bar visible for successful downloads.
        progress.finish();
    } else {
        progress.finish_and_clear();
    }
    result
}

/// Stream the response body into `file`, updating `progress` as data arrives.
async fn stream_response(
    response: &'_ mut reqwest::Response,
    file: &'_ mut async_std::fs::File,
    progress: &'_ ProgressBar,
    url: &'_ reqwest::Url,
    file_path: &'_ Path,
) -> Result<(), DownloadError> {
    while let Some(chunk) =
        response
            .chunk()
            .await
            .map_err(|source| DownloadError::ResponseChunk {
                url: url.clone(),
                source,
            })?
    {
        file.write_all(&chunk)
            .await
            .map_err(|source| DownloadError::FileWrite {
                file_path: file_path.to_path_buf(),
                source,
            })?;
        progress.inc(chunk.len() as u64);
    }

    file.flush()
        .await
        .map_err(|source| DownloadError::FileWrite {
            file_path: file_path.to_path_buf(),
            source,
        })?;
    file.sync_all()
        .await
        .map_err(|source| DownloadError::FileWrite {
            file_path: file_path.to_path_buf(),
            source,
        })?;

    Ok(())
}

fn download_progress(label: &'_ str, total_bytes: Option<u64>) -> ProgressBar {
    let progress = match total_bytes {
        Some(total_bytes) => {
            let progress = ProgressBar::new(total_bytes);
            progress.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{spinner:.green} [{bar:40.cyan/blue}] {percent:>3}% \
                         {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta}) {msg}",
                    )
                    .expect("download progress template should be valid")
                    .progress_chars("█▉▊▋▌▍▎▏  "),
            );
            progress
        }
        None => {
            let progress = ProgressBar::new_spinner();
            progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {bytes} ({bytes_per_sec}) {msg}")
                    .expect("download spinner template should be valid"),
            );
            progress.enable_steady_tick(Duration::from_millis(100));
            progress
        }
    };

    progress.set_message(label.to_string());
    progress
}

/// Path used for the in-progress download, kept alongside the final file so the
/// final rename stays on the same filesystem.
fn partial_path(out_file: &'_ Path) -> PathBuf {
    let mut partial_name = std::ffi::OsString::from(".");
    if let Some(file_name) = out_file.file_name() {
        partial_name.push(file_name);
    }
    partial_name.push(format!(".{}.part", std::process::id()));
    out_file.with_file_name(partial_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Write},
        net::{SocketAddr, TcpListener, TcpStream},
    };
    use tempfile::tempdir;

    fn read_request(stream: &mut TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                break;
            }
        }
    }

    fn write_response(stream: &mut TcpStream, status: &str, body: &[u8]) {
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
    }

    fn spawn_server(
        responses: Vec<(&'static str, Vec<u8>)>,
    ) -> (SocketAddr, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                read_request(&mut stream);
                write_response(&mut stream, status, &body);
            }
        });
        (addr, handle)
    }

    #[test]
    fn partial_path_stays_next_to_the_target() {
        let out_file = Path::new("/tmp/tools/ripgrep.tar.gz");
        let partial = partial_path(out_file);

        assert_eq!(partial.parent(), out_file.parent());
        let name = partial.file_name().unwrap().to_string_lossy();
        assert!(name.contains("ripgrep.tar.gz"), "{name}");
        assert!(name.ends_with(".part"), "{name}");
    }

    #[async_std::test]
    async fn downloads_a_response_body_to_disk() {
        let body = b"streamed body contents".to_vec();
        let (addr, server) = spawn_server(vec![("200 OK", body.clone())]);

        let dir = tempdir().unwrap();
        let url: reqwest::Url = format!("http://{addr}/tool.tar.gz").parse().unwrap();
        let path = download_url(url, "tool.tar.gz", dir.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), body);
        server.join().unwrap();
    }

    #[async_std::test]
    async fn retries_transient_server_errors() {
        let body = b"retried body".to_vec();
        let (addr, server) = spawn_server(vec![
            ("500 Internal Server Error", Vec::new()),
            ("200 OK", body.clone()),
        ]);

        let dir = tempdir().unwrap();
        let url: reqwest::Url = format!("http://{addr}/tool.tar.gz").parse().unwrap();
        let path = download_url(url, "tool.tar.gz", dir.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), body);
        server.join().unwrap();
    }

    #[async_std::test]
    async fn does_not_retry_client_errors() {
        let body = b"not found".to_vec();
        let (addr, server) = spawn_server(vec![("404 Not Found", body)]);

        let dir = tempdir().unwrap();
        let url: reqwest::Url = format!("http://{addr}/missing.tar.gz").parse().unwrap();
        let err = download_url(url, "missing.tar.gz", dir.path().to_path_buf())
            .await
            .unwrap_err();

        assert!(matches!(err, DownloadError::HttpStatus { .. }));
        assert!(!dir.path().join("missing.tar.gz").exists());
        server.join().unwrap();
    }
}
