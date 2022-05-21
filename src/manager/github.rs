//! Implementation of the Manager trait pulling information from GitHub releases
use async_trait::async_trait;
use crate::{system::System, version::Version};
use super::{Manager, Asset};

#[derive(Debug)]
pub struct GitHubManager {}

#[async_trait]
impl Manager for GitHubManager {
    async fn get_all_versions(&self) -> crate::Result<Vec<Version>>{
        todo!()
    }
    async fn get_latest_version(&self) -> crate::Result<Version>{
        todo!()
    }
    async fn get_assets_for_version(&self, version: &'_ Version, system: &'_ System) -> crate::Result<Vec<Asset>>{
        todo!()
    }
}
