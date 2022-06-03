use crate::system::{OperatingSystem, PlatformArchitecture};

#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub download_url: String,
    pub architecture: PlatformArchitecture,
    pub operating_system: OperatingSystem,
    pub asset_type: AssetType,
}

#[derive(Debug, Clone)]
pub enum AssetType {
    Archive,
    Source,
    Binary,
    Unknown,
}

impl AssetType {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "archive" => Self::Archive,
            "installer" => Self::Binary,
            "source" | "src" => Self::Source,
            _ => Self::Unknown,
        }
    }
}
