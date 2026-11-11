#![cfg(unix)]

//! Offline filesystem and CLI regressions for safe tool removal.
//!
//! These tests drive the real `vers` binary so they stay decoupled from
//! internal Rust signatures (including the future `--purge` field). Fixtures
//! are hand-written environment JSON plus fake executable trees under a
//! temporary data directory — no network access.

use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use tempfile::TempDir;

const SCRIPT: &[u8] = b"#!/bin/sh\nprintf 'fixture\\n'\n";
type TreeEntry = (PathBuf, Option<Vec<u8>>, Option<PathBuf>);

struct Store {
    dir: TempDir,
}

impl Store {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn envs_dir(&self) -> PathBuf {
        self.path().join("envs")
    }

    fn env_file(&self, env: &str) -> PathBuf {
        self.envs_dir().join(format!("{env}.json"))
    }

    fn env_bin_dir(&self, env: &str) -> PathBuf {
        self.envs_dir().join(env)
    }

    fn link_path(&self, env: &str, alias: &str) -> PathBuf {
        self.env_bin_dir(env).join(alias)
    }

    fn tools_root(&self) -> PathBuf {
        self.path().join("tools")
    }

    fn version_dir(&self, tool: &str, version: &str) -> PathBuf {
        self.tools_root().join(tool).join(version)
    }

    fn binary_path(&self, tool: &str, version: &str, relative: &str) -> PathBuf {
        self.version_dir(tool, version).join(relative)
    }

    fn write_env(&self, env: &str, tools: Value) {
        fs::create_dir_all(self.envs_dir()).unwrap();
        let doc = json!({
            "name": env,
            "base_dir": "",
            "tools": tools,
        });
        fs::write(self.env_file(env), serde_json::to_vec_pretty(&doc).unwrap()).unwrap();
    }

    fn write_raw_env(&self, env: &str, contents: &str) {
        fs::create_dir_all(self.envs_dir()).unwrap();
        fs::write(self.env_file(env), contents).unwrap();
    }

    fn read_env(&self, env: &str) -> Value {
        serde_json::from_slice(&fs::read(self.env_file(env)).unwrap()).unwrap()
    }

    fn install_version(&self, tool: &str, version: &str, relative: &str) -> PathBuf {
        let binary = self.binary_path(tool, version, relative);
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, SCRIPT).unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
        binary
    }

    fn link_alias(&self, env: &str, alias: &str, target: &Path) {
        let link = self.link_path(env, alias);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        if link.exists() || link.symlink_metadata().is_ok() {
            fs::remove_file(&link).unwrap();
        }
        let absolute = fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
        symlink(absolute, &link).unwrap();
    }

    fn tool_entry(
        name: &str,
        alias: &str,
        current: &str,
        installed: &[&str],
        binary_path: Option<&str>,
    ) -> Value {
        let mut entry = json!({
            "name": name,
            "alias": alias,
            "asset_pattern": "",
            "file_pattern": "",
            "current_version": current,
            "installed_versions": installed,
        });
        if let Some(path) = binary_path {
            entry["binary_path"] = json!(path);
        }
        entry
    }

    fn run(&self, env: &str, args: &[&str]) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vers"))
            .arg("--data-dir")
            .arg(self.path())
            .arg("--env")
            .arg(env)
            .args(args)
            .env_remove("GITHUB_TOKEN")
            .env_remove("RUST_BACKTRACE")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn vers");

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match child.try_wait().expect("poll vers") {
                Some(_) => break,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let output = child.wait_with_output().expect("collect timed-out vers");
                    panic!(
                        "vers timed out after 10s\nargs: {args:?}\nstdout: {}\nstderr: {}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        child.wait_with_output().expect("collect vers output")
    }

    fn snapshot_tree(&self) -> Vec<TreeEntry> {
        let mut entries = Vec::new();
        if !self.path().exists() {
            return entries;
        }
        fn is_lock_housekeeping(rel: &Path) -> bool {
            // Only envs/.store-lock and envs/<env>.lock — not tools or alias paths.
            let mut components = rel.components();
            match (components.next(), components.next(), components.next()) {
                (
                    Some(std::path::Component::Normal(envs)),
                    Some(std::path::Component::Normal(name)),
                    None,
                ) if envs == "envs" => {
                    let name = name.to_string_lossy();
                    name == ".store-lock" || name.ends_with(".lock")
                }
                _ => false,
            }
        }
        fn walk(root: &Path, dir: &Path, entries: &mut Vec<TreeEntry>) {
            let mut children: Vec<_> = fs::read_dir(dir)
                .map(|rd| rd.filter_map(Result::ok).collect())
                .unwrap_or_default();
            children.sort_by_key(|e| e.file_name());
            for child in children {
                let path = child.path();
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                if is_lock_housekeeping(&rel) {
                    continue;
                }
                let meta = fs::symlink_metadata(&path).ok();
                let file_type = meta.as_ref().map(|m| m.file_type());
                if file_type.map(|t| t.is_symlink()).unwrap_or(false) {
                    entries.push((rel, None, fs::read_link(&path).ok()));
                } else if file_type.map(|t| t.is_file()).unwrap_or(false) {
                    entries.push((rel, fs::read(&path).ok(), None));
                } else if file_type.map(|t| t.is_dir()).unwrap_or(false) {
                    entries.push((rel, None, None));
                    walk(root, &path, entries);
                } else {
                    entries.push((rel, None, None));
                }
            }
        }
        entries.push((PathBuf::from(""), None, None));
        walk(self.path(), self.path(), &mut entries);
        entries
    }
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstatus: {}\nstdout: {}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_exists(path: &Path) -> bool {
    path.exists() || path.symlink_metadata().is_ok()
}

fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Default removal of the active version keeps shared downloads and drops the
/// alias link and environment record when it was the only version.
#[test]
fn default_remove_preserves_downloads_and_clears_alias_link() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo"]);
    assert_success(&output);

    assert!(!path_exists(&store.link_path("global", "tool")));
    assert!(binary.is_file(), "shared download must remain");
    assert_eq!(store.read_env("global")["tools"], json!([]));
}

/// `--link-only` matches default non-purging removal: update/remove records and
/// links the same way, but never delete downloads. Records must not be left in
/// place, or a later sync could resurrect the tool.
#[test]
fn link_only_matches_default_metadata_removal_and_preserves_downloads() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    // Selected inactive version with --link-only still forgets that version.
    let output = store.run("global", &["remove", "owner/repo@v1.0.0", "--link-only"]);
    assert_success(&output);
    assert!(v1.is_file());
    assert!(v2.is_file());
    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v2.0.0");
    assert_eq!(tools["installed_versions"], json!(["v2.0.0"]));
    assert!(is_symlink(&store.link_path("global", "tool")));

    // Restore both versions to exercise active-version fallback under --link-only.
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );
    store.link_alias("global", "tool", &v2);

    let output = store.run("global", &["remove", "owner/repo", "--link-only"]);
    assert_success(&output);
    assert!(v1.is_file());
    assert!(v2.is_file());
    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v1.0.0");
    assert_eq!(tools["installed_versions"], json!(["v1.0.0"]));
    assert_eq!(
        fs::canonicalize(store.link_path("global", "tool")).unwrap(),
        fs::canonicalize(&v1).unwrap()
    );

    // Removing the final version forgets the tool record entirely.
    let output = store.run("global", &["remove", "owner/repo", "--link-only"]);
    assert_success(&output);
    assert!(v1.is_file());
    assert!(v2.is_file());
    assert_eq!(store.read_env("global")["tools"], json!([]));
    assert!(!path_exists(&store.link_path("global", "tool")));
}

/// Alias resolution must delete dangling symlinks and tolerate already-missing
/// links while still updating environment metadata.
#[test]
fn removes_alias_link_including_dangling_and_missing() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    // Make the link dangling.
    fs::remove_file(&binary).unwrap();
    assert!(is_symlink(&store.link_path("global", "tool")));
    assert!(!store.link_path("global", "tool").exists());

    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo"]);
    assert_success(&output);
    assert!(!path_exists(&store.link_path("global", "tool")));
    assert_eq!(store.read_env("global")["tools"], json!([]));

    // Missing link is idempotent for filesystem state.
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/other",
            "other",
            "v2.0.0",
            &["v2.0.0"],
            Some("tool"),
        )]),
    );
    assert!(!path_exists(&store.link_path("global", "other")));
    let output = store.run("global", &["remove", "owner/other"]);
    assert_success(&output);
    assert_eq!(store.read_env("global")["tools"], json!([]));
}

/// `--all` forgets every version for the tool in this environment but keeps
/// the shared download tree unless `--purge` is also supplied.
#[test]
fn all_forgets_tool_but_preserves_cache() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo", "--all"]);
    assert_success(&output);

    assert!(!path_exists(&store.link_path("global", "tool")));
    assert!(v1.is_file());
    assert!(v2.is_file());
    assert!(store.version_dir("owner/repo", "v1.0.0").is_dir());
    assert!(store.version_dir("owner/repo", "v2.0.0").is_dir());
    assert_eq!(store.read_env("global")["tools"], json!([]));
}

/// Selective `--purge` deletes only the requested version directory when no
/// other environment references it.
#[test]
fn purge_selected_version_deletes_unreferenced_assets() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo@v1.0.0", "--purge"]);
    assert_success(&output);

    assert!(!store.version_dir("owner/repo", "v1.0.0").exists());
    assert!(v2.is_file());
    assert!(path_exists(&store.link_path("global", "tool")));
    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v2.0.0");
    assert_eq!(tools["installed_versions"], json!(["v2.0.0"]));
    // Guard against accidental whole-repo cache wipe.
    assert!(store.tools_root().join("owner/repo").is_dir());
    let _ = v1;
}

/// `--all --purge` removes the repository cache for this tool when nothing else
/// references it.
#[test]
fn purge_all_deletes_repository_cache_when_unreferenced() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_success(&output);

    assert!(!path_exists(&store.link_path("global", "tool")));
    assert!(!store.tools_root().join("owner/repo").exists());
    assert_eq!(store.read_env("global")["tools"], json!([]));
    let _ = (v1, v2);
}

/// Active or installed references in another environment must block purge and
/// leave the caller environment unchanged.
#[test]
fn other_environment_active_and_installed_refs_block_purge() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v1);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    // Other env actively uses v1.
    store.link_alias("work", "tool", &v1);
    store.write_env(
        "work",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--purge"]);
    assert_failure(&output);
    assert_eq!(
        store.snapshot_tree(),
        before,
        "blocked purge must be a no-op"
    );
    assert!(v1.is_file());

    // Inactive installed reference (v2 only listed in work) still blocks.
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v2.0.0"],
            Some("tool"),
        )]),
    );
    store.link_alias("global", "tool", &v2);
    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo@v2.0.0", "--purge"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert!(v2.is_file());
    assert!(path_exists(&store.link_path("global", "tool")));
}

/// Malformed sibling environment JSON fails closed for purge but must not
/// prevent default (non-purge) removal in the active environment.
#[test]
fn malformed_env_json_blocks_purge_but_not_default_remove() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    store.write_raw_env("broken", "{not-json");

    let before = store.snapshot_tree();
    let purge = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_failure(&purge);
    assert_eq!(store.snapshot_tree(), before);

    let default = store.run("global", &["remove", "owner/repo"]);
    assert_success(&default);
    assert!(!path_exists(&store.link_path("global", "tool")));
    assert!(binary.is_file());
    assert_eq!(store.read_env("global")["tools"], json!([]));
}

/// Removing the active version falls back to the last remaining recorded
/// version when its cached executable is present.
#[test]
fn removing_active_version_falls_back_to_last_remaining_cached() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo"]);
    assert_success(&output);

    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v1.0.0");
    assert_eq!(tools["installed_versions"], json!(["v1.0.0"]));
    assert!(v1.is_file());
    assert!(v2.is_file(), "default remove preserves downloads");

    let link = store.link_path("global", "tool");
    assert!(is_symlink(&link));
    let target = fs::canonicalize(&link).unwrap();
    assert_eq!(target, fs::canonicalize(&v1).unwrap());
}

/// Fallback must reuse a recorded raw platform filename (`binary_path`) even
/// when it differs from the command alias, as long as the same relative path
/// exists inside the remaining version directory.
#[test]
fn fallback_reuses_raw_binary_path_distinct_from_alias() {
    let store = Store::new();
    let raw_name = "tool-aarch64-apple-darwin";
    let v1 = store.install_version("owner/repo", "v1.0.0", raw_name);
    let v2 = store.install_version("owner/repo", "v2.0.0", raw_name);
    store.link_alias("global", "mytool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "mytool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some(raw_name),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo"]);
    assert_success(&output);

    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v1.0.0");
    assert_eq!(tools["alias"], "mytool");
    assert_eq!(tools["binary_path"], raw_name);
    assert_eq!(tools["installed_versions"], json!(["v1.0.0"]));
    assert!(v1.is_file());
    assert!(v2.is_file());

    let link = store.link_path("global", "mytool");
    assert!(is_symlink(&link));
    assert_eq!(
        fs::canonicalize(&link).unwrap(),
        fs::canonicalize(&v1).unwrap()
    );
}

/// Explicit inactive version removal leaves the active link and current_version
/// alone while pruning installed_versions metadata.
#[test]
fn removing_explicit_inactive_version_keeps_active_link() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let output = store.run("global", &["remove", "owner/repo@v1.0.0"]);
    assert_success(&output);

    let tools = &store.read_env("global")["tools"][0];
    assert_eq!(tools["current_version"], "v2.0.0");
    assert_eq!(tools["installed_versions"], json!(["v2.0.0"]));
    assert!(is_symlink(&store.link_path("global", "tool")));
    assert_eq!(
        fs::canonicalize(store.link_path("global", "tool")).unwrap(),
        fs::canonicalize(&v2).unwrap()
    );
    assert!(v1.is_file());
}

/// When the only remaining fallback version has no cached executable, refuse
/// the change so links, files, and records stay intact. `--all` can still detach.
#[test]
fn missing_fallback_cache_errors_without_changes() {
    let store = Store::new();
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    // v1 is recorded but has no on-disk cache.
    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);

    let detach = store.run("global", &["remove", "owner/repo", "--all"]);
    assert_success(&detach);
    assert!(!path_exists(&store.link_path("global", "tool")));
    assert!(v2.is_file());
    assert_eq!(store.read_env("global")["tools"], json!([]));
}

/// Regular files and directories sitting at the alias path must never be deleted.
#[test]
fn regular_file_and_directory_aliases_are_protected() {
    let store = Store::new();
    store.install_version("owner/repo", "v1.0.0", "tool");
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    let file_link = store.link_path("global", "tool");
    fs::create_dir_all(file_link.parent().unwrap()).unwrap();
    fs::write(&file_link, "user file").unwrap();

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert_eq!(fs::read_to_string(&file_link).unwrap(), "user file");

    // Directory collision.
    fs::remove_file(&file_link).unwrap();
    fs::create_dir_all(file_link.join("nested")).unwrap();
    fs::write(file_link.join("nested/keep"), "stay").unwrap();
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--link-only"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert_eq!(
        fs::read_to_string(file_link.join("nested/keep")).unwrap(),
        "stay"
    );
}

/// Missing cache paths are harmless for both default removal and purge.
#[test]
fn missing_cache_paths_are_idempotent() {
    let store = Store::new();
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    assert!(!store.version_dir("owner/repo", "v1.0.0").exists());

    let output = store.run("global", &["remove", "owner/repo"]);
    assert_success(&output);
    assert_eq!(store.read_env("global")["tools"], json!([]));

    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    let output = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_success(&output);
    assert_eq!(store.read_env("global")["tools"], json!([]));
    assert!(!store.tools_root().join("owner/repo").exists());
}

/// Conflicting CLI combinations are rejected before any mutation.
#[test]
fn cli_rejects_purge_with_link_only_and_all_with_explicit_version() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    let before = store.snapshot_tree();

    let conflict = store.run(
        "global",
        &["remove", "owner/repo", "--purge", "--link-only"],
    );
    assert_failure(&conflict);
    assert_eq!(store.snapshot_tree(), before);

    let versioned_all = store.run("global", &["remove", "owner/repo@v1.0.0", "--all"]);
    assert_failure(&versioned_all);
    assert_eq!(store.snapshot_tree(), before);
}

/// Invalid path components in the tool selector are rejected without touching
/// the store. Symlinked cache ancestors must not allow purge to escape.
#[test]
fn invalid_path_components_and_symlinked_purge_ancestors_are_rejected() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    let before = store.snapshot_tree();
    for args in [
        ["remove", "owner/../repo"].as_slice(),
        ["remove", "owner/repo@v1/../v2"].as_slice(),
        ["remove", "../owner/repo"].as_slice(),
    ] {
        let output = store.run("global", args);
        assert_failure(&output);
        assert_eq!(store.snapshot_tree(), before, "args={args:?}");
    }

    // Replace the tool cache path with a symlink pointing outside the store.
    let outside = store.path().join("outside-cache");
    fs::create_dir_all(outside.join("v1.0.0")).unwrap();
    fs::write(outside.join("v1.0.0/tool"), SCRIPT).unwrap();
    let repo_cache = store.tools_root().join("owner/repo");
    fs::remove_dir_all(&repo_cache).unwrap();
    fs::create_dir_all(store.tools_root().join("owner")).unwrap();
    symlink(&outside, &repo_cache).unwrap();
    // Keep the env link aimed at the escaped cache so a naive delete would
    // remove user data outside the managed store.
    store.link_alias("global", "tool", &outside.join("v1.0.0/tool"));

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert!(outside.join("v1.0.0/tool").is_file());
}

/// Unknown tools fail without creating environment side effects.
#[test]
fn missing_tool_returns_error() {
    let store = Store::new();
    store.write_env("global", json!([]));
    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/missing"]);
    assert_failure(&output);
    assert!(stderr_of(&output).contains("Error:"));
    assert_eq!(store.snapshot_tree(), before);
}

/// `.hidden` is a valid environment name; its shared cache references must
/// block purge even though the config file starts with a dot.
#[test]
fn hidden_environment_shared_refs_block_purge() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );

    store.link_alias(".hidden", "tool", &binary);
    store.write_env(
        ".hidden",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )]),
    );
    assert!(store.env_file(".hidden").is_file());

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert!(binary.is_file());
    assert_eq!(
        store.read_env(".hidden")["tools"][0]["current_version"],
        "v1.0.0"
    );
}

/// A blocked purge that would have needed a fallback executable must not chmod
/// that fallback during preflight resolution.
#[test]
fn blocked_purge_does_not_chmod_fallback_binary() {
    let store = Store::new();
    let v1 = store.install_version("owner/repo", "v1.0.0", "tool");
    let v2 = store.install_version("owner/repo", "v2.0.0", "tool");
    // Leave the fallback candidate non-executable.
    fs::set_permissions(&v1, fs::Permissions::from_mode(0o644)).unwrap();
    let mode_before = fs::metadata(&v1).unwrap().permissions().mode() & 0o777;

    store.link_alias("global", "tool", &v2);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );
    // Sibling env keeps v2 alive so purge is rejected.
    store.write_env(
        "work",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v2.0.0"],
            Some("tool"),
        )]),
    );

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--purge"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    let mode_after = fs::metadata(&v1).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode_after, mode_before, "preflight must not chmod fallback");
    assert_eq!(mode_after, 0o644);
}

/// A JSON document whose embedded `name` disagrees with its filename must not
/// let save overwrite another environment or skip purge reference checks.
#[test]
fn mismatched_json_name_cannot_overwrite_other_env_or_bypass_purge() {
    let store = Store::new();
    let binary = store.install_version("owner/repo", "v1.0.0", "tool");
    store.link_alias("global", "tool", &binary);

    // File is global.json but claims to be the "work" environment.
    let mismatched = json!({
        "name": "work",
        "base_dir": "",
        "tools": [Store::tool_entry(
            "owner/repo",
            "tool",
            "v1.0.0",
            &["v1.0.0"],
            Some("tool"),
        )],
    });
    fs::create_dir_all(store.envs_dir()).unwrap();
    fs::write(
        store.env_file("global"),
        serde_json::to_vec_pretty(&mismatched).unwrap(),
    )
    .unwrap();

    // Real work env also references the cache and must keep its own contents.
    let work_tools = json!([Store::tool_entry(
        "owner/repo",
        "tool",
        "v1.0.0",
        &["v1.0.0"],
        Some("tool"),
    )]);
    store.write_env("work", work_tools.clone());
    let work_before = fs::read(store.env_file("work")).unwrap();

    let before = store.snapshot_tree();
    let output = store.run("global", &["remove", "owner/repo", "--all", "--purge"]);
    assert_failure(&output);
    assert_eq!(store.snapshot_tree(), before);
    assert_eq!(fs::read(store.env_file("work")).unwrap(), work_before);
    assert!(binary.is_file());

    // Even a successful non-purge remove against --env global must write
    // global.json, never clobber work.json via the mismatched embedded name.
    let output = store.run("global", &["remove", "owner/repo"]);
    // Implementation may reject mismatched names outright or complete against
    // the file-derived environment identity; either way work.json is sacred.
    if output.status.success() {
        assert_eq!(fs::read(store.env_file("work")).unwrap(), work_before);
        assert_ne!(
            store.read_env("global")["tools"],
            work_tools,
            "global.json must reflect the operated environment"
        );
    } else {
        assert_eq!(fs::read(store.env_file("work")).unwrap(), work_before);
        let global = fs::read(store.env_file("global")).unwrap();
        assert!(!global.is_empty());
    }
}

#[test]
fn purge_refuses_incomplete_or_invalid_reference_configurations() {
    for sibling in [
        json!({}),
        json!({"tools": [{"name": "owner/repo"}]}),
        json!({"tools": [{
            "name": "owner/repo",
            "current_version": "../v1.0.0",
            "installed_versions": []
        }]}),
    ] {
        let store = Store::new();
        let binary = store.install_version("owner/repo", "v1.0.0", "tool");
        store.link_alias("global", "tool", &binary);
        store.write_env(
            "global",
            json!([Store::tool_entry(
                "owner/repo",
                "tool",
                "v1.0.0",
                &["v1.0.0"],
                Some("tool"),
            )]),
        );
        store.write_raw_env("broken", &sibling.to_string());
        for args in [
            &["remove", "owner/repo", "--purge"][..],
            &["remove", "owner/repo", "--all", "--purge"][..],
        ] {
            let before = store.snapshot_tree();
            let output = store.run("global", args);
            assert_failure(&output);
            assert_eq!(store.snapshot_tree(), before, "{sibling}");
        }
    }
}

#[test]
fn purging_active_version_switches_to_cached_fallback_without_deleting_other_assets() {
    let store = Store::new();
    let old = store.install_version("owner/repo", "v1.0.0", "tool-platform");
    let active = store.install_version("owner/repo", "v2.0.0", "tool-platform");
    let unrelated = store.install_version("owner/other", "v2.0.0", "other");
    store.link_alias("global", "tool", &active);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool-platform"),
        )]),
    );

    assert_success(&store.run("global", &["remove", "owner/repo", "--purge"]));
    assert!(!active.exists());
    assert!(old.is_file());
    assert!(unrelated.is_file());
    assert_eq!(
        fs::canonicalize(store.link_path("global", "tool")).unwrap(),
        fs::canonicalize(old).unwrap()
    );
    let config = store.read_env("global");
    assert_eq!(config["tools"][0]["current_version"], "v1.0.0");
    assert_eq!(config["tools"][0]["installed_versions"], json!(["v1.0.0"]));
}

#[test]
fn symlinked_fallback_version_directory_is_rejected_without_changes() {
    let store = Store::new();
    let active = store.install_version("owner/repo", "v2.0.0", "tool");
    let external = tempfile::tempdir().unwrap();
    let external_binary = external.path().join("tool");
    fs::write(&external_binary, SCRIPT).unwrap();
    fs::set_permissions(&external_binary, fs::Permissions::from_mode(0o644)).unwrap();
    symlink(external.path(), store.version_dir("owner/repo", "v1.0.0")).unwrap();
    store.link_alias("global", "tool", &active);
    store.write_env(
        "global",
        json!([Store::tool_entry(
            "owner/repo",
            "tool",
            "v2.0.0",
            &["v1.0.0", "v2.0.0"],
            Some("tool"),
        )]),
    );
    let before = store.snapshot_tree();
    assert_failure(&store.run("global", &["remove", "owner/repo"]));
    assert_eq!(store.snapshot_tree(), before);
    assert_eq!(
        fs::metadata(external_binary).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

// Store-lock concurrency coverage lives in lead-owned tests/store_lock.rs.
