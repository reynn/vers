mod github;
mod go;

use {crate::version::Version, async_trait::async_trait};

pub use {github::GitHubManager, go::GoManager};

pub struct Asset {
    pub name: String,
    pub download_url: String,
}

#[async_trait]
pub trait Manager: std::fmt::Debug + Sync + Send {
    async fn get_all_versions(&self, name: &'_ str) -> crate::Result<Vec<Version>>;
    async fn get_latest_version(&self, name: &'_ str) -> crate::Result<Version>;
    async fn get_assets_for_version(
        &self,
        name: &'_ str,
        version: &'_ Version,
    ) -> crate::Result<Vec<Asset>>;
    fn name(&self) -> &'static str;
    fn name_required(&self) -> bool {
        true
    }
}
