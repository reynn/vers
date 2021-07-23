use directories_next::ProjectDirs;
use lazy_static::lazy_static;
use std::path::Path;

lazy_static! {
    static ref DIRS: ProjectDirs = ProjectDirs::from("dev", "reynn", "vers").unwrap();
}

pub fn get_default_data_dir() -> Option<&'static Path> {
    Some(DIRS.data_dir())
}

pub fn get_default_config_dir() -> Option<&'static Path> {
    Some(DIRS.config_dir())
}
