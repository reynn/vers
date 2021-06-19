use crate::errors::*;

#[derive(Debug, Clone)]
pub enum VersionType {
    SemVer {
        major: i8,
        minor: i8,
        patch: i8,
        pre_release: Option<String>,
        build: Option<String>,
    },
    Uknown(String),
}

impl VersionType {
    pub fn parse(s: &'_ str) -> Result<VersionType> {
        Ok(VersionType::Uknown(s.to_owned()))
    }
}

#[derive(Debug, Clone)]
pub enum Version {
    Latest,
    Specific(VersionType),
    Stable,
    PreRelease,
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
