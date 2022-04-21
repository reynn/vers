use {
    log::info,
    once_cell::sync::Lazy,
    regex::Regex,
    serde::{Deserialize, Serialize},
    std::fmt::Display,
};

static SEMVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)(?P<pre_release>-[a-zA-Z\d][-a-zA-Z.\d]*)?(?P<metadata>\+[a-zA-Z\d][-a-zA-Z.\d]*)?$"#).expect("Unable to compile regex for Semantic Versioning")
});

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    /// The latest version of a tool as determined by the tool managers
    Latest,
    /// SemVer should follow the patterns outlined at [SemVer.org](https://semver.org/)
    SemVer {
        major: usize,
        minor: usize,
        patch: usize,
        metadata: Option<String>,
        pre_release: Option<String>,
    },
    /// LTS is not available for all tool managers, this will be a version that isn't the latest but is supported longer than a typical release.
    Lts,
    /// Stable often times is synonymous with Latest however this will ensure that the version is considered stable before installing.
    Stable,
    /// Similar to Latest except will use pre-releases if there are any available for the tool
    PreRelease,
}

impl Version {
    pub fn as_tag(&self) -> String {
        match self {
            Version::SemVer {
                major,
                minor,
                patch,
                metadata,
                pre_release,
            } => {
                let mut v = format!("v{major}.{minor}.{patch}");
                if let Some(metadata) = metadata {
                    v += format!("-{}", metadata).as_str();
                }
                if let Some(pre_release) = pre_release {
                    v += format!("+{}", pre_release).as_str();
                }
                v
            }
            Version::Latest => "latest".to_string(),
            Version::Lts => "lts".to_string(),
            Version::Stable => "stable".to_string(),
            Version::PreRelease => "pre-release".to_string(),
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_tag())
    }
}

pub fn parse_version(provided_version: &'_ str) -> super::Result<Version> {
    let provided_version = provided_version.strip_prefix('v').unwrap();
    info!("Parsing version: {:?}", provided_version);
    if let Some(captures) = SEMVER_REGEX.captures(provided_version) {
        Ok(Version::SemVer {
            major: captures.name("major").unwrap().as_str().parse().unwrap(),
            minor: captures.name("minor").unwrap().as_str().parse().unwrap(),
            patch: captures.name("patch").unwrap().as_str().parse().unwrap(),
            metadata: if let Some(metadata) = captures.name("metadata") {
                Some(metadata.as_str()[1..].to_string())
            } else {
                None
            },
            pre_release: if let Some(pre_release) = captures.name("pre_release") {
                Some(pre_release.as_str()[1..].to_string())
            } else {
                None
            },
        })
    } else {
        Ok(Version::Latest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse_with_metadata_and_prerelease() {
        assert_eq!(
            parse_version("0.1.0-alpha+fd3f4b7a5a331a2384ed13fb3ead44e975438c3b").unwrap(),
            Version::SemVer {
                major: 0,
                minor: 1,
                patch: 0,
                pre_release: Some("alpha".into()),
                metadata: Some("fd3f4b7a5a331a2384ed13fb3ead44e975438c3b".into()),
            }
        );
    }
    #[test]
    fn test_semver_parse_with_metadata() {
        assert_eq!(
            parse_version("1.3.5+2022-04-20").unwrap(),
            Version::SemVer {
                major: 1,
                minor: 3,
                patch: 5,
                metadata: Some("2022-04-20".to_string()),
                pre_release: None,
            }
        );
    }
    #[test]
    fn test_semver_parse_with_prerelease() {
        assert_eq!(
            parse_version("3.3.3-alpha").unwrap(),
            Version::SemVer {
                major: 3,
                minor: 3,
                patch: 3,
                metadata: None,
                pre_release: Some("alpha".to_string())
            }
        );
    }
}
