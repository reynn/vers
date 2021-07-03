use std::fmt::Display;

use crate::errors::*;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum VersionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionType {
    /// Semantic version is a description version system that tracks the severity of changes so end
    /// users can make informed decisions about upgrading. The specification is avaiable at [SemVer.org](https://semver.org/)
    SemVer {
        major: i8,
        minor: i8,
        patch: i8,
        pre_release: Option<String>,
        build: Option<String>,
    },
    Uknown(String),
}

impl Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionType::SemVer {
                major,
                minor,
                patch,
                pre_release,
                build,
            } => {
                let mut sem_ver = format!("{}.{}.{}", major, minor, patch);
                if let Some(pre_release) = pre_release {
                    sem_ver.push_str(&format!("-{}", pre_release));
                }
                if let Some(build) = build {
                    sem_ver.push_str(&format!("+{}", build));
                }
                write!(f, "({})", sem_ver)
            }
            VersionType::Uknown(_) => todo!(),
        }
    }
}

impl VersionType {
    pub fn parse(s: &'_ str) -> Result<VersionType> {
        Ok(VersionType::Uknown(s.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    Latest,
    Specific(VersionType),
    Lts,
    Stable,
    PreRelease,
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::Latest
    }
}

impl Version {
    pub fn parse(s: &str) -> Result<Version> {
        match s {
            "" => Ok(Version::Specific(VersionType::parse(s)?)),
            _ => Ok(Version::Latest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse() {
        let v = Version::parse("0.1.0-alpha+fd3f4b7a5a331a2384ed13fb3ead44e975438c3b")
            .expect("Semver parse failed");
        dbg!(v);
    }
}
