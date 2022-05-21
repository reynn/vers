mod github;

use async_trait::async_trait;
use crate::{version::Version, system::System};

pub struct Asset {
    
}

#[async_trait]
pub trait Manager: std::fmt::Debug + Sync {
    async fn get_all_versions(&self) -> crate::Result<Vec<Version>>;
    async fn get_latest_version(&self) -> crate::Result<Version>;
    async fn get_assets_for_version(&self, version: &'_ Version, system: &'_ System) -> crate::Result<Vec<Asset>>;
}
