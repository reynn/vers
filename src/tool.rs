use {
    crate::{environment::Environment, version::Version},
    serde::{Deserialize, Serialize},
    std::path::Path,
    tokio::fs::read_to_string,
};

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct Tool {
    pub name: String,
    pub alias: String,
    pub asset_pattern: String,
    pub file_pattern: String,
    pub current_version: String,
    pub installed_versions: Vec<String>,
    pub known_versions: Vec<String>,
}

impl Tool {
    pub fn new(
        name: &'_ str,
        alias: &'_ str,
        version: &'_ Version,
        asset_pattern: &'_ str,
        file_pattern: &'_ str,
    ) -> Self {
        let version = version.as_tag();
        Self {
            name: name.to_string(),
            alias: alias.to_string(),
            asset_pattern: asset_pattern.to_string(),
            file_pattern: file_pattern.to_string(),
            current_version: version.clone(),
            installed_versions: vec![version.clone()],
            known_versions: vec![version],
        }
    }

    pub fn set_current_version(&mut self, version: &'_ Version) {
        self.current_version = version.to_string()
    }

    pub fn add_version(&mut self, version: &'_ Version) {
        self.installed_versions.push(version.as_tag())
    }
}
