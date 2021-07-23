use crate::errors::*;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
/// TODO: write docs
pub enum VersionError {}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
/// TODO: write docs
pub enum VersionType {
    /// Semantic version is a description version system that tracks the severity of changes so end
    /// users can make informed decisions about upgrading. The specification is avaiable at [SemVer.org](https://semver.org/)
    SemVer {
        /// TODO: write docs
        major: i8,
        /// TODO: write docs
        minor: i8,
        /// TODO: write docs
        patch: i8,
        /// TODO: write docs
        pre_release: Option<String>,
        /// TODO: write docs
        build: Option<String>,
    },
    /// TODO: write docs
    Unknown(String),
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
            VersionType::Unknown(_) => todo!(),
        }
    }
}

impl VersionType {
    /// Parse input into a VersionType
    pub fn parse<S: Into<String>>(s: S) -> Result<VersionType> {
        Ok(VersionType::Unknown(s.into()))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
/// TODO: write docs
pub enum Version {
    /// TODO: write docs
    Latest,
    /// TODO: write docs
    Specific(VersionType),
    /// TODO: write docs
    Lts,
    /// TODO: write docs
    Stable,
    /// TODO: write docs
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
    /// TODO: write docs
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
        dbg!(&v);
        let t = Version::Specific(VersionType::SemVer {
            major: 0,
            minor: 1,
            patch: 0,
            pre_release: Some("alpha".into()),
            build: Some("fd3f4b7a5a331a2384ed13fb3ead44e975438c3b".into()),
        });
        assert_eq!(&v, &t);
    }
}
