use directories_next::{BaseDirs, ProjectDirs, UserDirs};
use std::path::{Path, PathBuf};

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

pub fn get_environment_link_path(base_path: &'_ Path, env_name: &'_ str) -> PathBuf {
    base_path.join("envs").join(env_name)
}

pub fn get_environment_config_file_path(base_path: &'_ Path, env_name: &'_ str) -> PathBuf {
    base_path.join("envs").join(format!("{}.json", env_name))
}

pub fn get_tool_link_path(base_path: &'_ Path, tool_alias: &'_ str) -> PathBuf {
    base_path.join(tool_alias)
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
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tools")
        .join(name)
}

#[cfg(test)]
mod test {
    use {
        super::*,
        std::path::{Path, PathBuf},
        test_case::test_case,
    };

    #[test_case(
        Path::new("/home/test/.config/vers/envs/global"),
        "cli/cli",
        Path::new("/home/test/.config/vers/tools/cli/cli").to_path_buf() ; "cli/cli (linux)"
    )]
    #[test_case(
        Path::new("/Users/test/Library/Application Support/dev.reynn.vers/envs/global"),
        "neovim/neovim",
        Path::new("/Users/test/Library/Application Support/dev.reynn.vers/tools/neovim/neovim").to_path_buf() ; "neovim/neovim (osx)"
    )]
    pub fn test_tool_download_dir(input_path: &'_ Path, tool_name: &'_ str, expected: PathBuf) {
        assert_eq!(get_tool_download_dir(input_path, tool_name), expected)
    }

    #[test_case(
        Path::new("/home/test/.config/vers/envs/global"),
        "cli/cli",
        "v2.14.3",
        Path::new("/home/test/.config/vers/tools/cli/cli/v2.14.3").to_path_buf() ; "cli/cli@v2.14.3 (linux)"
    )]
    #[test_case(
        Path::new("/Users/test/Library/Application Support/dev.reynn.vers/envs/global"),
        "cli/cli",
        "v2.14.3",
        Path::new("/Users/test/Library/Application Support/dev.reynn.vers/tools/cli/cli/v2.14.3").to_path_buf() ; "cli/cli@v2.14.3 (osx)"
    )]
    pub fn test_get_tool_version_download_dir(
        input: &'_ Path,
        tool_name: &'_ str,
        version: &'_ str,
        expected: PathBuf,
    ) {
        assert_eq!(
            get_tool_version_download_dir(input, tool_name, version),
            expected
        )
    }

    #[test_case(
        Path::new("/home/test/.config/vers/envs/global"),
        "cli",
        Path::new("/home/test/.config/vers/envs/global/cli").to_path_buf() ;
        "global cli"
    )]
    #[test_case(
        Path::new("/home/test/.config/vers/envs/default"),
        "gh",
        Path::new("/home/test/.config/vers/envs/default/gh").to_path_buf() ;
        "default gh"
    )]
    pub fn test_get_tool_link_path(input: &'_ Path, tool_alias: &'_ str, expected: PathBuf) {
        assert_eq!(get_tool_link_path(input, tool_alias), expected)
    }
}
