use crate::{
    archiver, dirs, download,
    error::format_error_chain,
    tool::Tool,
    validation::{
        validate_alias, validate_environment_name, validate_tool_name, validate_version,
        ValidationError,
    },
    version::Version,
};
use async_std::fs::read_to_string;
use octocrab::models::repos::Asset;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;
use tracing::{debug, info};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Error)]
pub enum EnvironmentFileError {
    #[error("Failed to create the directory '{path}'. {source}")]
    FailedToCreateDirectory {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Unable to read file '{file_path}'. {source}")]
    FileReadError {
        file_path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to deserialize file: {file_path}, using {format}. {msg}")]
    DeserializationError {
        file_path: std::path::PathBuf,
        format: String,
        msg: String,
        // source: Box<dyn serde::de::Error>,
    },
    #[error("Failed to acquire the environment lock file '{path}'. {source}")]
    LockFileError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to serialize the '{name}' environment. {source}")]
    SerializationError {
        name: String,
        source: serde_json::Error,
    },
    #[error("Unable to determine the environment directory from '{base_dir}'")]
    InvalidBaseDir { base_dir: String },
    #[error("Failed to write the environment file '{path}'. {source}")]
    FileWriteError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Failed to replace the environment file '{path}'. {source}")]
    FilePersistError {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

fn binary_io(path: &Path, source: std::io::Error) -> EnvironmentError {
    EnvironmentError::BinaryIo {
        path: path.to_path_buf(),
        source,
    }
}

/// Canonicalize `path` and confirm it is a regular file inside `version_dir`.
///
/// This is side-effect free: it does not change permissions. Callers that need
/// an executable bit set should use [`prepare_binary`] after preflight checks.
fn resolve_binary_path(
    path: &Path,
    version_dir: &Path,
) -> std::result::Result<PathBuf, EnvironmentError> {
    let binary = std::fs::canonicalize(path).map_err(|source| binary_io(path, source))?;
    let root =
        std::fs::canonicalize(version_dir).map_err(|source| binary_io(version_dir, source))?;
    if !binary.starts_with(&root) || !binary.is_file() {
        return Err(EnvironmentError::InvalidBinaryPath { path: binary });
    }
    Ok(binary)
}

fn ensure_executable(binary: &Path) -> std::result::Result<(), EnvironmentError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(binary)
            .map_err(|source| binary_io(binary, source))?
            .permissions();
        if permissions.mode() & 0o100 == 0 {
            permissions.set_mode(permissions.mode() | 0o100);
            std::fs::set_permissions(binary, permissions)
                .map_err(|source| binary_io(binary, source))?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = binary;
    }
    Ok(())
}

fn prepare_binary(
    path: &Path,
    version_dir: &Path,
) -> std::result::Result<PathBuf, EnvironmentError> {
    let binary = resolve_binary_path(path, version_dir)?;
    ensure_executable(&binary)?;
    Ok(binary)
}

fn is_raw_executable(path: &Path) -> std::result::Result<bool, EnvironmentError> {
    let mut file = File::open(path).map_err(|source| binary_io(path, source))?;
    let mut magic = [0; 4];
    let count = file
        .read(&mut magic)
        .map_err(|source| binary_io(path, source))?;
    Ok((count >= 2 && &magic[..2] == b"#!")
        || (count == 4
            && matches!(
                &magic,
                b"\x7fELF"
                    | b"\xfe\xed\xfa\xce"
                    | b"\xce\xfa\xed\xfe"
                    | b"\xfe\xed\xfa\xcf"
                    | b"\xcf\xfa\xed\xfe"
                    | b"\xca\xfe\xba\xbe"
                    | b"\xbe\xba\xfe\xca"
                    | b"\xca\xfe\xba\xbf"
                    | b"\xbf\xba\xfe\xca"
            )))
}

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("Failed to access binary '{path}'. {source}")]
    BinaryIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Binary path '{path}' must identify a file inside the tool's version directory")]
    InvalidBinaryPath { path: PathBuf },
    #[error(
        "Asset '{path}' is not a supported archive or executable (ELF, Mach-O, or shebang script)"
    )]
    UnsupportedAsset { path: PathBuf },
    #[error("Failed to extract '{path}': {failures}")]
    ExtractionFailed { path: PathBuf, failures: String },
    #[error("Failed to link '{target}' as '{link}'. {source}")]
    Symlink {
        target: PathBuf,
        link: PathBuf,
        source: std::io::Error,
    },
    #[error("Unable to find binary '{expected_file_name}' within folder {search_base_path}")]
    UnableToFindBinaryError {
        expected_file_name: String,
        search_base_path: std::path::PathBuf,
    },
    #[error("Failed to download file '{asset_name}' from '{asset_uri}'. {source}")]
    AssetDownloadError {
        asset_uri: reqwest::Url,
        asset_name: String,
        source: Box<download::DownloadError>,
    },
    #[error(transparent)]
    Validation(#[from] ValidationError),
}

type Result<T, E = EnvironmentFileError> = std::result::Result<T, E>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub name: String,
    pub base_dir: String,
    // #[serde(skip)]
    // base_dir_path: PathBuf,
    pub tools: Vec<Tool>,
    /// Filename stem used to load this environment (`{config_key}.json`).
    ///
    /// On load, [`name`] is rewritten to match this key so saves always write
    /// back to the file that was opened. Purge safety checks also skip the
    /// current file by this key.
    #[serde(skip)]
    config_key: String,
    /// Exclusive lock held for the lifetime of an environment loaded for update.
    #[serde(skip)]
    lock_file: Option<File>,
    /// Shared store lock serializing mutations across all environments.
    ///
    /// Acquired before [`lock_file`] so purge reference scans cannot race with
    /// installs or config writes in other environments.
    #[serde(skip)]
    store_lock: Option<File>,
}

impl Environment {
    pub async fn load<P: Into<PathBuf>>(config_dir: P, name: &'_ str) -> Result<Self> {
        validate_environment_name(name)?;
        // Read-only loads must not create the environment directory, lock files,
        // or the shared store lock; `read` treats a missing directory like a
        // missing environment file.
        let env_dir = resolve_config_dir(config_dir.into())?.join("envs");
        Self::read(env_dir, name, None, None).await
    }

    /// Load the environment while holding the shared store lock and an exclusive
    /// lock on its lock file.
    ///
    /// Use this for commands that may mutate and later [`save`](Self::save) the
    /// environment. The shared store lock is acquired first so cross-environment
    /// operations (for example purge reference checks) cannot race with other
    /// writers. Both locks are held until the returned `Environment` is dropped.
    pub async fn load_for_update<P: Into<PathBuf>>(config_dir: P, name: &'_ str) -> Result<Self> {
        validate_environment_name(name)?;
        let config_dir = resolve_config_dir(config_dir.into())?;
        let env_dir = Self::ensure_env_dir(config_dir.clone())?;
        let env_dir = std::fs::canonicalize(&env_dir).map_err(|source| {
            EnvironmentFileError::FileReadError {
                file_path: config_dir.join("envs"),
                source,
            }
        })?;
        // Store-wide lock first, then the per-environment lock.
        let store_lock = acquire_store_lock(&env_dir)?;
        let lock_file = acquire_lock(&env_dir, name)?;
        Self::read(env_dir, name, Some(lock_file), Some(store_lock)).await
    }

    fn ensure_env_dir(config_dir: PathBuf) -> Result<PathBuf> {
        let env_dir = config_dir.join("envs");
        if !env_dir.exists() {
            std::fs::create_dir_all(&env_dir).map_err(|source| {
                EnvironmentFileError::FailedToCreateDirectory {
                    path: env_dir.clone(),
                    source,
                }
            })?;
        }
        Ok(env_dir)
    }

    async fn read(
        env_dir: PathBuf,
        name: &'_ str,
        lock_file: Option<File>,
        store_lock: Option<File>,
    ) -> Result<Self> {
        let env_path = env_dir.join(format!("{}.json", name));
        match read_to_string(&env_path).await {
            Ok(file_contents) => match from_str::<Self>(&file_contents) {
                Ok(mut res) => {
                    // The requested filename is authoritative. A mismatched
                    // JSON `name` must not redirect saves onto another env file.
                    res.name = name.to_string();
                    res.base_dir = env_dir
                        .join(name)
                        .to_str()
                        .expect("Unable to convert path to a string")
                        .to_string();
                    res.config_key = name.to_string();
                    res.lock_file = lock_file;
                    res.store_lock = store_lock;
                    Ok(res)
                }
                Err(serde_err) => Err(EnvironmentFileError::DeserializationError {
                    file_path: env_path,
                    format: "json".into(),
                    msg: serde_err.to_string(),
                }),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    debug!("Environment file does not exist");
                    let base_dir: String = env_dir.join(name).to_str().unwrap_or_default().into();
                    Ok(Environment {
                        name: name.to_string(),
                        base_dir,
                        tools: Vec::new(),
                        config_key: name.to_string(),
                        lock_file,
                        store_lock,
                    })
                }
                _ => Err(EnvironmentFileError::FileReadError {
                    file_path: env_path,
                    source: read_err,
                }),
            },
        }
    }

    /// Atomically persist the environment to disk.
    ///
    /// The JSON is written to a temporary file next to the environment file,
    /// flushed to disk, and then renamed over the real file. A lock file guards
    /// against concurrent writers; mutating commands should load with
    /// [`load_for_update`](Self::load_for_update) so the lock also covers the
    /// read/modify/write cycle.
    pub fn save(&self) -> Result<()> {
        validate_environment_name(&self.name)?;

        let env_dir = Path::new(&self.base_dir).parent().ok_or_else(|| {
            EnvironmentFileError::InvalidBaseDir {
                base_dir: self.base_dir.clone(),
            }
        })?;

        std::fs::create_dir_all(env_dir).map_err(|source| {
            EnvironmentFileError::FailedToCreateDirectory {
                path: env_dir.to_path_buf(),
                source,
            }
        })?;

        // Mutating commands hold both locks already, via `load_for_update`.
        // Any other caller still gets a consistent, non-torn write. The shared
        // store lock is always taken before the per-environment lock.
        let _transient_store_lock = if self.store_lock.is_none() {
            Some(acquire_store_lock(env_dir)?)
        } else {
            None
        };
        let _transient_lock = if self.lock_file.is_none() {
            Some(acquire_lock(env_dir, &self.name)?)
        } else {
            None
        };

        let contents =
            to_string_pretty(self).map_err(|source| EnvironmentFileError::SerializationError {
                name: self.name.clone(),
                source,
            })?;

        let out_file = env_dir.join(format!("{}.json", self.name));
        let temp_file = env_dir.join(format!(".{}.json.tmp.{}", self.name, std::process::id()));

        if let Err(source) = write_and_sync(&temp_file, contents.as_bytes()) {
            let _ = std::fs::remove_file(&temp_file);
            return Err(EnvironmentFileError::FileWriteError {
                path: temp_file,
                source,
            });
        }

        if let Err(source) = std::fs::rename(&temp_file, &out_file) {
            let _ = std::fs::remove_file(&temp_file);
            return Err(EnvironmentFileError::FilePersistError {
                path: out_file,
                source,
            });
        }

        // Make the rename itself durable. Not every platform permits syncing a
        // directory, so failure here is not fatal.
        if let Ok(dir) = File::open(env_dir) {
            let _ = dir.sync_all();
        }

        debug!("Saved environment '{}' to {:?}", self.name, out_file);
        Ok(())
    }

    pub async fn add_tool(
        &mut self,
        name: &'_ str,
        alias: &'_ str,
        version: Version,
        asset: Asset,
        asset_pattern: &'_ str,
        file_pattern: &'_ str,
    ) -> std::result::Result<(), EnvironmentError> {
        // Every one of these values is joined into a filesystem path below.
        validate_tool_name(name)?;
        validate_alias(alias)?;
        let version_tag = version.as_tag();
        validate_version(&version_tag)?;

        let env_base_path = Path::new(&self.base_dir);
        let tool_version_dir =
            dirs::get_tool_version_download_dir(env_base_path, name, &version_tag);
        // Asset names also become filesystem components.
        validate_alias(&asset.name)?;
        let asset_path = download::download_asset(&asset, &tool_version_dir)
            .await
            .map_err(|source| EnvironmentError::AssetDownloadError {
                asset_uri: asset.browser_download_url.clone(),
                asset_name: asset.name.clone(),
                source: Box::new(source),
            })?;
        let extractors = archiver::determine_possible_extractors(&asset_path);
        let binary = if extractors.is_empty() {
            // A raw asset need not have the same name as the command alias.
            // Check its header rather than chmod'ing arbitrary unsupported files.
            if !is_raw_executable(&asset_path)? {
                return Err(EnvironmentError::UnsupportedAsset { path: asset_path });
            }
            asset_path
        } else {
            let mut extracted = false;
            let mut failures = Vec::new();
            for extractor in extractors {
                let name = extractor.name();
                match archiver::handle_file_extraction(
                    extractor,
                    &asset_path,
                    Some(tool_version_dir.clone()),
                )
                .await
                {
                    Ok(()) => {
                        extracted = true;
                        break;
                    }
                    Err(error) => failures.push(format!("{name}: {}", format_error_chain(&error))),
                }
            }
            if !extracted {
                return Err(EnvironmentError::ExtractionFailed {
                    path: asset_path,
                    failures: failures.join("; "),
                });
            }
            let binary_name = if file_pattern.is_empty() {
                alias
            } else {
                file_pattern
            };
            find_binary(&tool_version_dir, binary_name)
                .map(DirEntry::into_path)
                .ok_or_else(|| EnvironmentError::UnableToFindBinaryError {
                    expected_file_name: binary_name.to_string(),
                    search_base_path: tool_version_dir.clone(),
                })?
        };

        let binary = prepare_binary(&binary, &tool_version_dir)?;
        let symlink_dest = dirs::get_tool_link_path(env_base_path, alias);
        create_symlink(&binary, &symlink_dest)?;
        let root = std::fs::canonicalize(&tool_version_dir)
            .map_err(|source| binary_io(&tool_version_dir, source))?;
        let relative_binary = binary
            .strip_prefix(&root)
            .map_err(|_| EnvironmentError::InvalidBinaryPath {
                path: binary.clone(),
            })?
            .to_path_buf();

        if let Some(installed) = self.tools.iter_mut().find(|tool| tool.name == name) {
            installed.set_current_version(&version);
            if !installed.installed_versions.contains(&version_tag) {
                installed.installed_versions.push(version_tag);
            }
            installed.alias = alias.to_string();
            installed.asset_pattern = asset_pattern.to_string();
            installed.file_pattern = file_pattern.to_string();
            installed.binary_path = Some(relative_binary);
        } else {
            let mut tool = Tool::new(name, alias, &version, asset_pattern, file_pattern);
            tool.binary_path = Some(relative_binary);
            self.tools.push(tool);
        }
        Ok(())
    }
}

/// Open (creating if necessary) and exclusively lock the shared store lock file.
///
/// Located at `envs/.store-lock` (not `*.lock`) so it cannot collide with a
/// per-environment lock for a valid environment name such as `.store`, whose
/// lock would otherwise be `envs/.store.lock` and deadlock on the same inode.
fn acquire_store_lock(env_dir: &'_ Path) -> Result<File> {
    let lock_path = env_dir.join(".store-lock");
    lock_file_at(&lock_path)
}

/// Open (creating if necessary) and exclusively lock the environment's lock file.
///
/// The returned `File` owns the lock; it is released when the file is dropped.
fn acquire_lock(env_dir: &'_ Path, name: &'_ str) -> Result<File> {
    let lock_path = env_dir.join(format!("{}.lock", name));
    lock_file_at(&lock_path)
}

fn lock_file_at(lock_path: &Path) -> Result<File> {
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|source| EnvironmentFileError::LockFileError {
            path: lock_path.to_path_buf(),
            source,
        })?;

    lock_file
        .lock()
        .map_err(|source| EnvironmentFileError::LockFileError {
            path: lock_path.to_path_buf(),
            source,
        })?;

    Ok(lock_file)
}

fn write_and_sync(path: &'_ Path, contents: &'_ [u8]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}

fn resolve_config_dir(config_dir: PathBuf) -> Result<PathBuf> {
    let resolved = if config_dir.exists() {
        std::fs::canonicalize(&config_dir)
    } else {
        std::path::absolute(&config_dir)
    };
    resolved.map_err(|source| EnvironmentFileError::FileReadError {
        file_path: config_dir,
        source,
    })
}

fn create_symlink(src: &'_ Path, dest: &'_ Path) -> std::result::Result<(), EnvironmentError> {
    let link_error = |source| EnvironmentError::Symlink {
        target: src.to_path_buf(),
        link: dest.to_path_buf(),
        source,
    };
    // Relative link targets are interpreted from the link's parent, not from
    // the process working directory. Always store the canonical source path.
    let source = std::fs::canonicalize(src).map_err(link_error)?;
    match std::fs::symlink_metadata(dest) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing = std::fs::read_link(dest).map_err(link_error)?;
            if existing == source {
                return Ok(());
            }
            std::fs::remove_file(dest).map_err(link_error)?;
        }
        Ok(_) => {
            return Err(link_error(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "destination exists and is not a symlink; move it before installing",
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(link_error(error)),
    }

    let parent = dest
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(link_error)?;
    info!("Creating symlink from {:?} to {:?}", source, dest);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&source, dest).map_err(link_error)
    }
    #[cfg(not(unix))]
    {
        Err(link_error(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "symlink installation is only supported on Unix",
        )))
    }
}

impl Environment {
    /// Filename stem for the config file this environment was loaded from.
    pub fn config_key(&self) -> &str {
        if self.config_key.is_empty() {
            &self.name
        } else {
            &self.config_key
        }
    }

    /// Resolve a cached executable for `tool` at `version` without side effects.
    ///
    /// Prefers a recorded relative `binary_path` when it still points at a file
    /// inside the requested version directory (including when switching versions
    /// for raw assets whose on-disk name differs from the alias). Falls back to
    /// searching by `file_pattern` / alias. Does not chmod or create links;
    /// callers that install must use [`Self::activate_cached_binary`].
    pub fn resolve_cached_binary(
        &self,
        tool: &Tool,
        version: &str,
    ) -> std::result::Result<PathBuf, EnvironmentError> {
        validate_tool_name(&tool.name)?;
        validate_alias(&tool.alias)?;
        validate_version(version)?;
        let env_path = Path::new(&self.base_dir);
        let version_dir = dirs::get_tool_version_download_dir(env_path, &tool.name, version);

        if let Some(path) = &tool.binary_path {
            if path.as_os_str().is_empty()
                || !path
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(EnvironmentError::InvalidBinaryPath { path: path.clone() });
            }
            let candidate = version_dir.join(path);
            if candidate.is_file() {
                return resolve_binary_path(&candidate, &version_dir);
            }
        }

        let name = if tool.file_pattern.is_empty() {
            tool.alias.as_str()
        } else {
            tool.file_pattern.as_str()
        };
        let binary = find_binary(&version_dir, name)
            .map(DirEntry::into_path)
            .ok_or_else(|| EnvironmentError::UnableToFindBinaryError {
                expected_file_name: name.to_string(),
                search_base_path: version_dir.clone(),
            })?;
        resolve_binary_path(&binary, &version_dir)
    }

    /// Link `tool`'s alias to a previously resolved cached binary and return the
    /// path of that binary relative to the version download directory.
    pub fn activate_cached_binary(
        &self,
        tool: &Tool,
        version: &str,
        binary: &Path,
    ) -> std::result::Result<PathBuf, EnvironmentError> {
        validate_tool_name(&tool.name)?;
        validate_alias(&tool.alias)?;
        validate_version(version)?;
        let env_path = Path::new(&self.base_dir);
        let version_dir = dirs::get_tool_version_download_dir(env_path, &tool.name, version);
        let binary = prepare_binary(binary, &version_dir)?;
        create_symlink(&binary, &dirs::get_tool_link_path(env_path, &tool.alias))?;
        let root = std::fs::canonicalize(&version_dir)
            .map_err(|source| binary_io(&version_dir, source))?;
        binary
            .strip_prefix(&root)
            .map(|path| path.to_path_buf())
            .map_err(|_| EnvironmentError::InvalidBinaryPath {
                path: binary.clone(),
            })
    }

    /// Return false only when the cached binary is missing and must be fetched.
    /// Existing environment files without binary_path use their filename filter.
    pub fn repair_tool_link(&self, tool: &Tool) -> std::result::Result<bool, EnvironmentError> {
        let binary = match self.resolve_cached_binary(tool, &tool.current_version) {
            Ok(path) => path,
            Err(EnvironmentError::UnableToFindBinaryError { .. }) => return Ok(false),
            Err(EnvironmentError::BinaryIo { source, .. })
                if source.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        if !binary.is_file() {
            return Ok(false);
        }
        self.activate_cached_binary(tool, &tool.current_version, &binary)?;
        Ok(true)
    }
}

fn find_binary(folder: &'_ Path, bin_name: &'_ str) -> Option<DirEntry> {
    WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| entry.file_name() == bin_name && entry.path().is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn raw_asset_detection_recognizes_binaries_and_scripts_not_archives_or_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("asset");
        for bytes in [
            &b"\x7fELFexecutable"[..],
            &b"\xcf\xfa\xed\xfemach-o"[..],
            &b"\xca\xfe\xba\xbefat-binary"[..],
            &b"#!/bin/sh\nexit 0\n"[..],
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(is_raw_executable(&path).unwrap(), "{bytes:?}");
        }
        for bytes in [
            &b""[..],
            &b"#"[..],
            &b"readme"[..],
            &b"PK\x03\x04zip"[..],
            &b"\x1f\x8bgzip"[..],
            &b"\xfd7zXZ\0"[..],
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(!is_raw_executable(&path).unwrap(), "{bytes:?}");
        }
    }

    #[cfg(unix)]
    #[async_std::test]
    async fn cached_binary_paths_cannot_escape_the_version_directory() {
        let dir = tempdir().unwrap();
        let env = Environment::load_for_update(dir.path(), "global")
            .await
            .unwrap();
        let mut tool = Tool::new("owner/repo", "tool", &Version::Simple("v1".into()), "", "");
        let root = dirs::get_tool_version_download_dir(Path::new(&env.base_dir), &tool.name, "v1");
        std::fs::create_dir_all(&root).unwrap();
        let outside = dir.path().join("outside");
        std::fs::write(&outside, "user data").unwrap();
        for path in [PathBuf::from("../outside"), outside.clone()] {
            tool.binary_path = Some(path);
            assert!(matches!(
                env.repair_tool_link(&tool),
                Err(EnvironmentError::InvalidBinaryPath { .. })
            ));
        }
        std::os::unix::fs::symlink(&outside, root.join("binary")).unwrap();
        tool.binary_path = Some(PathBuf::from("binary"));
        assert!(matches!(
            env.repair_tool_link(&tool),
            Err(EnvironmentError::InvalidBinaryPath { .. })
        ));
        assert!(!Path::new(&env.base_dir).join("tool").exists());
        assert_eq!(std::fs::read_to_string(outside).unwrap(), "user data");
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_idempotent_and_replace_stale_or_dangling_targets() {
        use std::os::unix::fs::MetadataExt;

        let dir = tempdir().unwrap();
        let source = dir.path().join("binary");
        let next = dir.path().join("next");
        let link = dir.path().join("env").join("tool");
        std::fs::write(&source, "first").unwrap();
        std::fs::write(&next, "second").unwrap();

        create_symlink(&source, &link).unwrap();
        let inode = std::fs::symlink_metadata(&link).unwrap().ino();
        create_symlink(&source, &link).unwrap();
        assert_eq!(std::fs::symlink_metadata(&link).unwrap().ino(), inode);

        create_symlink(&next, &link).unwrap();
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "second");
        std::fs::remove_file(&next).unwrap();
        assert!(!link.exists());
        create_symlink(&source, &link).unwrap();
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "first");
    }

    #[cfg(unix)]
    #[test]
    fn symlink_errors_preserve_existing_files_and_links() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("binary");
        let dest = dir.path().join("tool");
        std::fs::write(&source, "binary").unwrap();
        std::fs::write(&dest, "user file").unwrap();

        assert!(matches!(
            create_symlink(&source, &dest),
            Err(EnvironmentError::Symlink { source, .. })
                if source.kind() == std::io::ErrorKind::AlreadyExists
        ));
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "user file");

        let link = dir.path().join("link");
        create_symlink(&source, &link).unwrap();
        assert!(create_symlink(&dir.path().join("missing"), &link).is_err());
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "binary");
        assert!(create_symlink(&source, &dest.join("child")).is_err());
    }

    #[cfg(unix)]
    #[async_std::test]
    async fn relative_data_directories_produce_working_absolute_links() {
        // Keep the fixture under cwd so paths are genuinely relative without
        // changing the process-wide working directory during parallel tests.
        let cwd = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir_in(&cwd).unwrap();
        let relative = dir.path().strip_prefix(&cwd).unwrap().join("data");
        let env = Environment::load_for_update(&relative, "global")
            .await
            .unwrap();
        assert!(Path::new(&env.base_dir).is_absolute());

        let source = relative.join("tools/owner/repo/v1/binary");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "binary").unwrap();
        let link = Path::new(&env.base_dir).join("tool");
        create_symlink(&source, &link).unwrap();

        assert!(std::fs::read_link(&link).unwrap().is_absolute());
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "binary");
        env.save().unwrap();
        let loaded = Environment::load(&relative, "global").await.unwrap();
        assert_eq!(loaded.base_dir, env.base_dir);
    }

    #[async_std::test]
    async fn load_does_not_write_environment_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");

        let env = Environment::load(&config_dir, "global").await.unwrap();

        assert!(env.tools.is_empty());
        assert!(!config_dir.join("envs").join("global.json").exists());
        assert!(!config_dir.join("envs").exists());
    }

    #[async_std::test]
    async fn save_writes_and_round_trips_environment() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let mut env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();
        env.tools
            .push(Tool::new("cli/cli", "gh", &Version::Latest, "", "gh"));

        env.save().unwrap();

        let reloaded = Environment::load(&config_dir, "global").await.unwrap();
        assert_eq!(reloaded.tools.len(), 1);
        assert_eq!(reloaded.tools[0].name, "cli/cli");
        assert_eq!(reloaded.tools[0].alias, "gh");
    }

    #[async_std::test]
    async fn save_replaces_the_existing_file_without_leaving_temporaries() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");
        let mut env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();

        env.tools
            .push(Tool::new("cli/cli", "gh", &Version::Latest, "", "gh"));
        env.save().unwrap();

        env.tools.push(Tool::new(
            "neovim/neovim",
            "nvim",
            &Version::Latest,
            "",
            "nvim",
        ));
        env.save().unwrap();

        let env_dir = config_dir.join("envs");
        let temporary_files: Vec<String> = std::fs::read_dir(&env_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp."))
            .collect();

        assert!(temporary_files.is_empty(), "{temporary_files:?}");
        let reloaded = Environment::load(&config_dir, "global").await.unwrap();
        assert_eq!(reloaded.tools.len(), 2);
    }

    #[async_std::test]
    async fn load_for_update_creates_a_lock_file() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config");

        let _env = Environment::load_for_update(&config_dir, "global")
            .await
            .unwrap();

        assert!(config_dir.join("envs").join("global.lock").exists());
    }
}
