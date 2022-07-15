use {
    directories_next::{BaseDirs, ProjectDirs, UserDirs},
    std::path::PathBuf,
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
