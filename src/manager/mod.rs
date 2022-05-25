mod github;

use crate::{system::System, version::Version};
use async_trait::async_trait;

pub use github::GitHubManager;

pub struct Asset {
    pub name: String,
    pub download_url: String,
}

#[async_trait]
pub trait Manager: std::fmt::Debug + Sync {
    async fn get_all_versions(&self, name: &'_ str) -> crate::Result<Vec<Version>>;
    async fn get_latest_version(&self, name: &'_ str) -> crate::Result<Version>;
    async fn get_assets_for_version(
        &self,
        name: &'_ str,
        version: &'_ Version,
        system: &'_ System,
    ) -> crate::Result<Vec<Asset>>;
}
