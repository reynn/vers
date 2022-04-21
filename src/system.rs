use {
    regex::Regex,
    std::env::consts::{ARCH, OS},
};

#[derive(Debug)]
pub struct System {
    pub architecture: PlatformArchitecture,
    pub os: OperatingSystem,
}

impl System {
    pub fn new() -> Self {
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

    pub fn is_match(&self, s: &'_ str) -> bool {
        let os_regex = self.os.get_match_regex();
        let arch_regex = self.architecture.get_match_regex();

        os_regex.is_match(s) && arch_regex.is_match(s)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum OperatingSystem {
    Linux,
    Mac,
    Windows,
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

#[derive(Debug, PartialEq, Eq)]
pub enum PlatformArchitecture {
    I686,
    Amd64,
    Arm32,
    Arm64,
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
