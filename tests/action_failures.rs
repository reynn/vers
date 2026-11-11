use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    process::Command,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use vers::cli::Actions;

#[async_std::test]
async fn failed_release_lookup_is_returned_by_add_and_update() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut requests = 0;
        while requests < 5 && Instant::now() < deadline {
            let (mut stream, _) = match listener.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
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
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                    break;
                }
            }
            let body =
                r#"{"message":"Not Found","documentation_url":"https://docs.github.com/rest"}"#;
            write!(
                stream,
                "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            ).unwrap();
            requests += 1;
        }
        requests
    });
    octocrab::initialise(
        octocrab::Octocrab::builder()
            .base_uri(format!("http://{address}"))
            .unwrap()
            .build()
            .unwrap(),
    );
    let dir = tempdir().unwrap();
    let add = Actions::Add {
        name: "owner/repo@v1.0.0".into(),
        alias: None,
        asset_pattern: None,
        file_filter: None,
        pre_release: false,
        show: false,
    };
    let error = add.execute(dir.path().into(), "global").await.unwrap_err();
    assert!(
        error.to_string().contains("Failed to install tools"),
        "{error}"
    );

    std::fs::write(
        dir.path().join("envs/global.json"),
        r#"{"name":"global","base_dir":"","tools":[{"name":"owner/repo","alias":"tool","current_version":"v0.1.0","installed_versions":["v0.1.0"]}]}"#,
    ).unwrap();
    for name in [Some("tool@v1.0.0".into()), None] {
        let error = Actions::Update { name }
            .execute(dir.path().into(), "global")
            .await
            .unwrap_err();
        assert!(!error.to_string().is_empty());
    }
    assert_eq!(server.join().unwrap(), 5);
}

#[test]
fn cli_failures_exit_nonzero_without_backtraces() {
    let dir = tempdir().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vers"))
            .arg("--data-dir")
            .arg(dir.path())
            .args(args)
            .env_remove("GITHUB_TOKEN")
            .output()
            .unwrap()
    };
    for args in [&["add", "invalid"][..], &["update", "missing"][..]] {
        let output = run(args);
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("Error:"), "{stderr}");
        assert!(!stderr.contains("stack backtrace"), "{stderr}");
    }

    std::fs::write(
        dir.path().join("envs/global.json"),
        r#"{"name":"global","base_dir":"","tools":[{"name":"invalid","alias":"tool","current_version":"v1.0.0"}]}"#,
    ).unwrap();
    for args in [&["sync"][..], &["update"][..]] {
        let output = run(args);
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("invalid"), "{stderr}");
        assert!(!stderr.contains("stack backtrace"), "{stderr}");
    }
}
