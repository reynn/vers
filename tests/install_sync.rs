#![cfg(unix)]

use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    os::unix::fs::{symlink, MetadataExt, PermissionsExt},
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Duration,
};
use tempfile::tempdir;
use vers::cli::Actions;

const SCRIPT: &[u8] = b"#!/bin/sh\nprintf 'fixture-tool\\n'\n";

struct Fixture {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Fixture {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let mut routes = HashMap::new();

        let tarball = tar_gzip("nested/tool", SCRIPT);
        let missing = tar_gzip("tool/not-the-binary", b"readme");
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gzip.write_all(SCRIPT).unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("nested/tool", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(SCRIPT).unwrap();
        let zipped = zip.finish().unwrap().into_inner();
        for (repo, name, content) in [
            ("raw", "tool-platform", SCRIPT.to_vec()),
            ("tar", "tool.tar.gz", tarball.clone()),
            ("tgz", "tool.tgz", tarball),
            ("gzip", "tool.gz", gzip.finish().unwrap()),
            ("zip", "tool.zip", zipped),
            ("missing", "tool.tar.gz", missing),
            ("corrupt", "tool.tar.gz", b"not gzip data".to_vec()),
            ("corruptzip", "tool.zip", b"not zip data".to_vec()),
            (
                "unsupported",
                "tool.tar.xz",
                b"\xfd7zXZ\0unsupported".to_vec(),
            ),
            ("text", "tool.txt", b"not an executable".to_vec()),
        ] {
            let download_path = format!("/download/{repo}/{name}");
            let asset = json!({
                "url": format!("{base_url}/assets/1"),
                "browser_download_url": format!("{base_url}{download_path}"),
                "id": 1, "node_id": "asset", "name": name, "state": "uploaded",
                "content_type": "application/octet-stream", "size": content.len(),
                "download_count": 0,
                "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
            });
            let mut author = json!({
                "login": "owner", "id": 1, "node_id": "user", "gravatar_id": "",
                "type": "User", "site_admin": false
            });
            for key in [
                "avatar_url",
                "url",
                "html_url",
                "followers_url",
                "following_url",
                "gists_url",
                "starred_url",
                "subscriptions_url",
                "organizations_url",
                "repos_url",
                "events_url",
                "received_events_url",
            ] {
                author[key] = json!(base_url);
            }
            let release = json!({
                "url": base_url, "html_url": base_url, "assets_url": base_url,
                "upload_url": base_url, "id": 1, "node_id": "release",
                "tag_name": "v1.0.0", "target_commitish": "main",
                "draft": false, "prerelease": false, "author": author, "assets": [asset]
            });
            for endpoint in ["tags/v1.0.0", "latest"] {
                routes.insert(
                    format!("/repos/owner/{repo}/releases/{endpoint}"),
                    serde_json::to_vec(&release).unwrap(),
                );
            }
            routes.insert(download_path, content);
        }

        let seen = Arc::clone(&requests);
        let stopping = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let path = line.split_whitespace().nth(1).unwrap().to_string();
                seen.lock().unwrap().push(path.clone());
                loop {
                    line.clear();
                    if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                        break;
                    }
                }
                let (status, body) = match routes.get(&path) {
                    Some(body) => ("200 OK", body.as_slice()),
                    None => ("404 Not Found", &b"{\"message\":\"Not Found\"}"[..]),
                };
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                ).unwrap();
                stream.write_all(body).unwrap();
            }
        });
        Self {
            base_url,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let result = thread.join();
            if !std::thread::panicking() {
                result.unwrap();
            }
        }
    }
}

fn tar_gzip(name: &str, body: &[u8]) -> Vec<u8> {
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut tar = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    tar.append_data(&mut header, name, body).unwrap();
    tar.into_inner().unwrap().finish().unwrap()
}

fn add(repo: &str) -> Actions {
    Actions::Add {
        name: format!("owner/{repo}@latest"),
        alias: Some("tool".into()),
        asset_pattern: Some(".*".into()),
        file_filter: Some("tool".into()),
        pre_release: false,
        show: false,
    }
}

fn config(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("envs/global.json")).unwrap()).unwrap()
}

fn assert_executable(link: &Path) {
    let output = Command::new(link).output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"fixture-tool\n");
}

// Keep HTTP-client global initialization in one test; integration test files
// run as separate processes, so the other fixture suite remains isolated.
#[async_std::test]
async fn installs_assets_and_repairs_missing_files_and_links() {
    let fixture = Fixture::start();
    octocrab::initialise(
        octocrab::Octocrab::builder()
            .base_uri(fixture.base_url.as_str())
            .unwrap()
            .build()
            .unwrap(),
    );
    let dir = tempdir().unwrap();
    let raw = dir.path().join("raw");
    add("raw").execute(raw.clone(), "global").await.unwrap();
    let link = raw.join("envs/global/tool");
    let binary = raw.join("tools/owner/raw/v1.0.0/tool-platform");
    assert_executable(&link);
    assert_eq!(config(&raw)["tools"][0]["binary_path"], "tool-platform");
    assert_eq!(config(&raw)["tools"][0]["current_version"], "v1.0.0");
    assert!(!raw.join("tools/owner/raw/latest").exists());

    let count = fixture.count();
    let inode = std::fs::symlink_metadata(&link).unwrap().ino();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_eq!(std::fs::symlink_metadata(&link).unwrap().ino(), inode);
    std::fs::remove_file(&link).unwrap();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_executable(&link);

    let other = dir.path().join("unrelated");
    std::fs::write(&other, "do not alter").unwrap();
    std::fs::remove_file(&link).unwrap();
    symlink(&other, &link).unwrap();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_eq!(std::fs::read_to_string(&other).unwrap(), "do not alter");
    assert_executable(&link);

    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o644)).unwrap();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_executable(&link);
    assert_eq!(
        fixture.count(),
        count,
        "cached repairs should not use the network"
    );

    std::fs::remove_file(&link).unwrap();
    std::fs::write(&link, "user file").unwrap();
    assert!(Actions::Sync.execute(raw.clone(), "global").await.is_err());
    assert_eq!(std::fs::read_to_string(&link).unwrap(), "user file");
    std::fs::remove_file(&link).unwrap();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();

    std::fs::remove_file(&binary).unwrap();
    assert!(!link.exists());
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_executable(&link);
    assert_eq!(
        fixture.count(),
        count + 2,
        "fetch the exact tag and missing asset"
    );
    assert_eq!(
        config(&raw)["tools"][0]["installed_versions"],
        json!(["v1.0.0"])
    );

    std::fs::remove_dir_all(raw.join("tools")).unwrap();
    Actions::Sync.execute(raw.clone(), "global").await.unwrap();
    assert_executable(&link);

    for repo in ["tar", "tgz", "gzip", "zip"] {
        let data = dir.path().join(repo);
        add(repo).execute(data.clone(), "global").await.unwrap();
        let link = data.join("envs/global/tool");
        assert_executable(&link);
        assert!(!data
            .join(format!("tools/owner/{repo}/v1.0.0/tool.tar"))
            .exists());
        // Older configurations without recorded executable paths still repair.
        let mut legacy = config(&data);
        legacy["tools"][0]
            .as_object_mut()
            .unwrap()
            .remove("binary_path");
        std::fs::write(
            data.join("envs/global.json"),
            serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        std::fs::remove_file(&link).unwrap();
        let count = fixture.count();
        Actions::Sync.execute(data.clone(), "global").await.unwrap();
        assert_executable(&link);
        assert_eq!(fixture.count(), count);
        std::fs::remove_dir_all(data.join("tools")).unwrap();
        Actions::Sync.execute(data.clone(), "global").await.unwrap();
        assert_executable(&link);
    }

    for (repo, message) in [
        ("missing", "Unable to find binary"),
        ("corrupt", "Failed to extract"),
        ("corruptzip", "Failed to extract"),
        ("unsupported", "not a supported archive or executable"),
        ("text", "not a supported archive or executable"),
    ] {
        let data = dir.path().join(repo);
        let error = add(repo).execute(data.clone(), "global").await.unwrap_err();
        assert!(error.to_string().contains(message), "{repo}: {error}");
        assert_eq!(config(&data)["tools"], json!([]));
        assert!(!data.join("envs/global/tool").exists());
    }

    // A failed tool must not prevent sync from repairing another tool.
    let mut mixed = config(&raw);
    mixed["tools"].as_array_mut().unwrap().insert(
        0,
        json!({
            "name": "invalid", "alias": "bad", "current_version": "v1.0.0"
        }),
    );
    std::fs::write(
        raw.join("envs/global.json"),
        serde_json::to_vec(&mixed).unwrap(),
    )
    .unwrap();
    std::fs::remove_file(&link).unwrap();
    assert!(Actions::Sync.execute(raw.clone(), "global").await.is_err());
    assert_executable(&link);
}
