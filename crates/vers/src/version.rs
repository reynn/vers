use {log::info, std::fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    /// The latest version of a tool as determined by the tool managers
    Latest,
    /// SemVer should follow the patterns outlined at [SemVer.org](https://semver.org/)
    SemVer(semver::Version),
    /// LTS is not available for all tool managers, this will be a version that isn't the latest but is supported longer than a typical release.
    Lts,
    /// Stable often times is synonymous with Latest however this will ensure that the version is considered stable before installing.
    Stable,
    /// Similar to Latest except will use pre-releases if there are any available for the tool
    PreRelease,
    ///
    Simple(String),
}

impl Default for Version {
    fn default() -> Self {
        Self::Latest
    }
}

impl Version {
    pub fn as_tag(&self) -> String {
        match self {
            Version::SemVer(v) => v.to_string(),
            Version::Simple(s) => s.to_string(),
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

pub fn parse_version(provided_version: &'_ str) -> Version {
    match provided_version.to_lowercase().as_str() {
        "latest" => Version::Latest,
        "stable" => Version::Stable,
        "lts" => Version::Lts,
        "prerelease" | "pre-release" => Version::PreRelease,
        _ => {
            let provided_version = provided_version.trim_start_matches('v');
            info!("Parsing version: {}", provided_version);
            if let Ok(parsed_semver) = semver::Version::parse(provided_version) {
                Version::SemVer(parsed_semver)
            } else {
                Version::Simple(provided_version.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    #[test_case("laTeST", Version::Latest ; "Latest: mixed cases")]
    #[test_case("LATEST", Version::Latest ; "Latest: all uppercase")]
    #[test_case("latest", Version::Latest ; "Latest: all lowercase")]
    #[test_case("LtS", Version::Lts ; "LTS: mixed cases")]
    #[test_case("LTS", Version::Lts ; "LTS: all uppercase")]
    #[test_case("lts", Version::Lts ; "LTS: all lowercase")]
    #[test_case("PrEREleASe", Version::PreRelease ; "PreRelease: mixed case")]
    #[test_case("PRERELEASE", Version::PreRelease ; "PreRelease: all uppercase")]
    #[test_case("PreRelease", Version::PreRelease ; "PreRelease: all lowercase")]
    #[test_case("stABLe", Version::Stable ; "Stable: mixed case")]
    #[test_case("STABLE", Version::Stable ; "Stable: all uppercase")]
    #[test_case("stable", Version::Stable ; "Stable: all lowercase")]
    #[test_case("0.0.0",
                Version::SemVer(semver::Version{ major: 0, minor: 0, patch: 0, pre: semver::Prerelease::EMPTY, build: semver::BuildMetadata::EMPTY });
                "SemVer: 0.0.0 without prelease or metadata")]
    #[test_case("1.0.0",
                Version::SemVer(semver::Version{ major: 1, minor: 0, patch: 0, pre: semver::Prerelease::EMPTY, build: semver::BuildMetadata::EMPTY });
                "SemVer: 1.0.0 without prelease or metadata")]
    #[test_case("0.1.0-alpha+fd3f4b7a5a331a2384ed13fb3ead44e975438c3b",
                Version::SemVer(semver::Version{ major: 0, minor: 1, patch: 0, pre: semver::Prerelease::new("alpha").unwrap(), build: semver::BuildMetadata::new("fd3f4b7a5a331a2384ed13fb3ead44e975438c3b").unwrap() });
                "SemVer: with prelease and metadata")]
    #[test_case("2.10.0-alpha",
                Version::SemVer(semver::Version{ major: 2, minor: 10, patch: 0, pre: semver::Prerelease::new("alpha").unwrap(), build: semver::BuildMetadata::EMPTY });
                "SemVer: with prelease")]
    #[test_case("3.2.1+fd3f4b7a5a331a2384ed13fb3ead44e975438c3b",
                Version::SemVer(semver::Version{ major: 3, minor: 2, patch: 1, pre: semver::Prerelease::EMPTY, build: semver::BuildMetadata::new("fd3f4b7a5a331a2384ed13fb3ead44e975438c3b").unwrap() });
                "SemVer: with metadata")]
    fn parse_version_testing(input: &'_ str, expected: Version) {
        assert_eq!(parse_version(input), expected)
    }

    #[test_case(Version::Latest, "latest" ; "Latest tag")]
    #[test_case(Version::Stable, "stable" ; "Stable tag")]
    #[test_case(Version::Lts, "lts" ; "LTS tag")]
    #[test_case(Version::PreRelease, "pre-release" ; "PreRelease tag")]
    #[test_case(Version::SemVer(semver::Version{
        major: 3, minor: 0, patch: 7, pre: semver::Prerelease::new("rc2").unwrap(), build: semver::BuildMetadata::EMPTY
    }), "3.0.7-rc2" ; "SemVer: pre-release")]
    #[test_case(Version::SemVer(semver::Version{
        major: 1, minor: 0, patch: 0, pre: semver::Prerelease::EMPTY, build: semver::BuildMetadata::EMPTY
    }), "1.0.0" ; "SemVer: simple")]
    #[test_case(Version::SemVer(semver::Version{
        major: 0, minor: 1, patch: 0, pre: semver::Prerelease::new("beta").unwrap(), build: semver::BuildMetadata::new("0d4ef68a70ae20c3178a0b6321dcf6538895346c").unwrap()
    }), "0.1.0-beta+0d4ef68a70ae20c3178a0b6321dcf6538895346c" ; "SemVer: pre-release and metadata")]
    #[test_case(Version::SemVer(semver::Version{
        major: 3, minor: 10, patch: 0, pre: semver::Prerelease::EMPTY, build: semver::BuildMetadata::new("0d4ef68a70ae20c3178a0b6321dcf6538895346c").unwrap()
    }), "3.10.0+0d4ef68a70ae20c3178a0b6321dcf6538895346c" ; "SemVer: metadata")]
    fn as_tag_tests(input: Version, expected: &'_ str) {
        assert_eq!(input.as_tag(), expected)
    }
}
