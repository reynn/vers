use regex::Regex;
use std::{
    env::consts::{ARCH, OS},
    fmt::Display,
};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct System {
    pub architecture: PlatformArchitecture,
    pub os: OperatingSystem,
}

impl Default for System {
    fn default() -> Self {
        Self {
            architecture: match ARCH {
                "x86" => PlatformArchitecture::I686,
                "x86_64" => PlatformArchitecture::Amd64,
                "arm" => PlatformArchitecture::Arm32,
                "aarch64" => PlatformArchitecture::Arm64,
                _ => panic!("Running on a unknown system architecture"),
            },
            os: match OS {
                "linux" => OperatingSystem::Linux,
                "macos" => OperatingSystem::Mac,
                "windows" => OperatingSystem::Windows,
                _ => panic!("Running on a unknown operating system"),
            },
        }
    }
}

impl System {
    pub fn is_match(&self, s: &'_ str) -> bool {
        let os_regex = self.os.get_match_regex();
        let arch_regex = self.architecture.get_match_regex();

        debug!(
            "System OS Regex[{}], Arch Regex[{}], matching {}",
            os_regex.to_string(),
            arch_regex.to_string(),
            s
        );
        os_regex.is_match(s) && arch_regex.is_match(s)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum OperatingSystem {
    Linux,
    Mac,
    Windows,
}

impl Display for OperatingSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            OperatingSystem::Linux => write!(f, "linux"),
            OperatingSystem::Mac => write!(f, "Mac OS"),
            OperatingSystem::Windows => write!(f, "Windows"),
        }
    }
}

impl OperatingSystem {
    fn get_match_regex(&self) -> Regex {
        match self {
            Self::Linux => {
                Regex::new(r#"(?i).*linux.*"#).expect("unable to create regex for Linux")
            }
            Self::Mac => {
                Regex::new(r#"(?i).*mac|macos|darwin"#).expect("unable to create regex for Mac")
            }
            Self::Windows => {
                Regex::new(r#"(?i).*windows.*"#).expect("unable to create regex for Windows")
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PlatformArchitecture {
    I686,
    Amd64,
    Arm32,
    Arm64,
}

impl Display for PlatformArchitecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PlatformArchitecture::I686 => write!(f, "i686"),
            PlatformArchitecture::Amd64 => write!(f, "x86_64"),
            PlatformArchitecture::Arm32 => write!(f, "armv7"),
            PlatformArchitecture::Arm64 => write!(f, "arm64"),
        }
    }
}

impl PlatformArchitecture {
    fn get_match_regex(&self) -> Regex {
        match self {
            Self::I686 => {
                Regex::new(r#"(?i).*i386|i686.*"#).expect("Unable to create regex for i686")
            }
            Self::Amd64 => {
                Regex::new(r#"(?i).*amd64|x86_64.*"#).expect("Unable to create regex for amd64")
            }
            Self::Arm32 => {
                Regex::new(r#"(?i).*arm32|armv6.*"#).expect("Unable to create regex for arm32")
            }
            Self::Arm64 => {
                Regex::new(r#"(?i).*arm64|aarch64.*"#).expect("Unable to create regex for arm64")
            }
        }
    }
}
