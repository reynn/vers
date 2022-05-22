//! Implementation of the Manager trait pulling information from GitHub releases
use {
    super::{Asset, Manager},
    crate::{system::System, version::Version},
    async_trait::async_trait,
};

#[derive(Debug)]
pub struct GitHubManager;

#[async_trait]
impl Manager for GitHubManager {
    async fn get_all_versions(&self) -> crate::Result<Vec<Version>> {
        todo!()
    }
    async fn get_latest_version(&self) -> crate::Result<Version> {
        todo!()
    }
    async fn get_assets_for_version(
        &self,
        _version: &'_ Version,
        _system: &'_ System,
    ) -> crate::Result<Vec<Asset>> {
        todo!()
    }
}
