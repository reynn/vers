use {
    directories_next::{BaseDirs, ProjectDirs, UserDirs},
    std::path::{Path, PathBuf},
};

pub fn get_default_config_path() -> PathBuf {
    if let Some(project_dirs) = ProjectDirs::from("dev", "reynn", "vers") {
        project_dirs.config_dir().to_path_buf()
    } else if let Some(base_dirs) = BaseDirs::new() {
        base_dirs.config_dir().join("vers")
    } else if let Some(user_dirs) = UserDirs::new() {
        user_dirs.home_dir().join(".config").join("vers")
    } else {
        panic!("Unable to determine a base directory for user configs")
    }
}

pub fn get_tool_link_path(base_path: &'_ Path, env_name: &'_ str, name: &'_ str) -> PathBuf {
    base_path.join("envs").join(env_name).join(name)
}

pub fn get_tool_version_download_dir(
    base_path: &'_ Path,
    name: &'_ str,
    version: &'_ str,
) -> PathBuf {
    get_tool_download_dir(base_path, name).join(version)
}

pub fn get_tool_download_dir(base_path: &'_ Path, name: &'_ str) -> PathBuf {
    base_path
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
        .join("tools")
        .join(name)
}
