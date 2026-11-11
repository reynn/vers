#![cfg(unix)]

use serde_json::json;
use std::{
    fs::OpenOptions,
    os::unix::fs::symlink,
    process::{Child, Command, Output, Stdio},
    thread::sleep,
    time::{Duration, Instant},
};
use tempfile::tempdir;

fn finish(mut child: Child) -> Output {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("CLI did not finish within ten seconds");
        }
        sleep(Duration::from_millis(10));
    }
}

#[test]
fn store_lock_blocks_mutations_but_not_read_only_commands() {
    let dir = tempdir().unwrap();
    let env_dir = dir.path().join("envs");
    let binary = dir.path().join("tools/owner/repo/v1/tool");
    let link = env_dir.join("global/tool");
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::fs::write(&binary, "fixture").unwrap();
    symlink(&binary, &link).unwrap();
    let config_path = env_dir.join("global.json");
    let config = serde_json::to_vec(&json!({
        "name": "global", "base_dir": "",
        "tools": [{
            "name": "owner/repo", "alias": "tool", "current_version": "v1",
            "installed_versions": ["v1"], "binary_path": "tool"
        }]
    }))
    .unwrap();
    std::fs::write(&config_path, &config).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(env_dir.join(".store-lock"))
        .unwrap();
    lock.lock().unwrap();

    let spawn = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_vers"))
            .arg("--data-dir")
            .arg(dir.path())
            .args(args)
            .env_remove("GITHUB_TOKEN")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let list = finish(spawn(&["list", "--output", "json"]));
    assert!(list.status.success(), "{:?}", list);
    assert_eq!(std::fs::read(&config_path).unwrap(), config);

    let mut remove = spawn(&["remove", "owner/repo"]);
    sleep(Duration::from_millis(150));
    assert!(remove.try_wait().unwrap().is_none());
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(&config_path).unwrap(), config);

    drop(lock);
    let remove = finish(remove);
    assert!(remove.status.success(), "{:?}", remove);
    assert!(!link.exists());
    assert!(
        binary.exists(),
        "ordinary removal must retain cached assets"
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config_path).unwrap()).unwrap();
    assert_eq!(saved["tools"], json!([]));
}

#[test]
fn hidden_environment_name_does_not_collide_with_the_store_lock() {
    let dir = tempdir().unwrap();
    let child = Command::new(env!("CARGO_BIN_EXE_vers"))
        .arg("--data-dir")
        .arg(dir.path())
        .args(["--env", ".store", "sync"])
        .env_remove("GITHUB_TOKEN")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = finish(child);
    assert!(output.status.success(), "{output:?}");
    assert!(dir.path().join("envs/.store.json").is_file());
    assert!(dir.path().join("envs/.store-lock").is_file());
}
