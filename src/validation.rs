use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error(
        "Invalid tool name '{name}'. Expected 'owner/repo', where each part is a single path \
         component containing only letters, digits, '-', '_', '.', or '+'."
    )]
    InvalidToolName { name: String },
    #[error(
        "Invalid alias '{alias}'. Aliases must be a single path component containing only \
         letters, digits, '-', '_', '.', or '+'."
    )]
    InvalidAlias { alias: String },
    #[error(
        "Invalid version '{version}'. Versions must be a single path component containing only \
         letters, digits, '-', '_', '.', or '+'."
    )]
    InvalidVersion { version: String },
    #[error(
        "Invalid environment name '{name}'. Environment names must be a single path component \
         containing only letters, digits, '-', '_', '.', or '+'."
    )]
    InvalidEnvironmentName { name: String },
}

type Result<T> = std::result::Result<T, ValidationError>;

/// Split and validate a tool name in the form `owner/repo`.
pub fn parse_tool_name(name: &'_ str) -> Result<(&'_ str, &'_ str)> {
    let mut parts = name.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(owner), Some(repo), None) if is_safe_component(owner) && is_safe_component(repo) => {
            Ok((owner, repo))
        }
        _ => Err(ValidationError::InvalidToolName {
            name: name.to_string(),
        }),
    }
}

pub fn validate_tool_name(name: &'_ str) -> Result<()> {
    parse_tool_name(name).map(|_| ())
}

pub fn validate_alias(alias: &'_ str) -> Result<()> {
    if is_safe_component(alias) {
        Ok(())
    } else {
        Err(ValidationError::InvalidAlias {
            alias: alias.to_string(),
        })
    }
}

pub fn validate_version(version: &'_ str) -> Result<()> {
    if is_safe_component(version) {
        Ok(())
    } else {
        Err(ValidationError::InvalidVersion {
            version: version.to_string(),
        })
    }
}

pub fn validate_environment_name(name: &'_ str) -> Result<()> {
    if is_safe_component(name) {
        Ok(())
    } else {
        Err(ValidationError::InvalidEnvironmentName {
            name: name.to_string(),
        })
    }
}

/// Whether `value` can be used as a single path component without escaping its
/// parent directory.
fn is_safe_component(value: &'_ str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("cli/cli" ; "simple")]
    #[test_case("BurntSushi/ripgrep" ; "case and hyphen")]
    #[test_case("owner/repo.name" ; "repo with dot")]
    #[test_case("owner/repo_name" ; "repo with underscore")]
    #[test_case("owner/repo+extra" ; "repo with plus")]
    fn accepts_valid_tool_names(name: &'_ str) {
        assert!(validate_tool_name(name).is_ok(), "{name}");
    }

    #[test_case("cli" ; "missing repo")]
    #[test_case("cli/cli/extra" ; "too many components")]
    #[test_case("../etc/passwd" ; "parent traversal")]
    #[test_case("cli/.." ; "parent component")]
    #[test_case("/cli" ; "absolute path")]
    #[test_case("cli/" ; "trailing slash")]
    #[test_case("cli//repo" ; "empty component")]
    #[test_case("owner/repo@1.0.0" ; "version included")]
    #[test_case("" ; "empty")]
    fn rejects_invalid_tool_names(name: &'_ str) {
        assert!(
            matches!(
                validate_tool_name(name),
                Err(ValidationError::InvalidToolName { .. })
            ),
            "{name}"
        );
    }

    #[test_case("gh" ; "simple")]
    #[test_case("gh-v" ; "hyphen")]
    #[test_case("python3.11" ; "dot")]
    #[test_case("a_b" ; "underscore")]
    #[test_case("a+b" ; "plus")]
    #[test_case(".hidden" ; "leading dot is still a file name")]
    fn accepts_valid_aliases(alias: &'_ str) {
        assert!(validate_alias(alias).is_ok(), "{alias}");
    }

    #[test_case("" ; "empty")]
    #[test_case("." ; "current directory")]
    #[test_case(".." ; "parent directory")]
    #[test_case("a/b" ; "slash")]
    #[test_case("a\\b" ; "backslash")]
    #[test_case("a b" ; "space")]
    #[test_case("a@b" ; "at sign")]
    fn rejects_invalid_aliases(alias: &'_ str) {
        assert!(
            matches!(
                validate_alias(alias),
                Err(ValidationError::InvalidAlias { .. })
            ),
            "{alias}"
        );
    }

    #[test_case("2.103.3" ; "semver")]
    #[test_case("v2.103.3" ; "v-prefixed tag")]
    #[test_case("jq-1.7.1" ; "release name")]
    #[test_case("1.0.0-rc.1" ; "prerelease")]
    #[test_case("1.0.0+build.5" ; "build metadata")]
    #[test_case("latest" ; "latest")]
    fn accepts_valid_versions(version: &'_ str) {
        assert!(validate_version(version).is_ok(), "{version}");
    }

    #[test_case("" ; "empty")]
    #[test_case("." ; "current directory")]
    #[test_case(".." ; "parent directory")]
    #[test_case("../../etc" ; "parent traversal")]
    #[test_case("v1/2" ; "slash")]
    #[test_case("v1 2" ; "space")]
    fn rejects_invalid_versions(version: &'_ str) {
        assert!(
            matches!(
                validate_version(version),
                Err(ValidationError::InvalidVersion { .. })
            ),
            "{version}"
        );
    }

    #[test_case("global" ; "simple")]
    #[test_case("work-env" ; "hyphen")]
    fn accepts_valid_environment_names(name: &'_ str) {
        assert!(validate_environment_name(name).is_ok(), "{name}");
    }

    #[test_case("" ; "empty")]
    #[test_case("." ; "current directory")]
    #[test_case(".." ; "parent directory")]
    #[test_case("../escape" ; "traversal")]
    #[test_case("a/b" ; "slash")]
    fn rejects_invalid_environment_names(name: &'_ str) {
        assert!(
            matches!(
                validate_environment_name(name),
                Err(ValidationError::InvalidEnvironmentName { .. })
            ),
            "{name}"
        );
    }
}
