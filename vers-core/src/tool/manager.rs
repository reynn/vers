use async_trait::async_trait;

use crate::{errors::*, machine::Details};

#[async_trait]
/// TODO: write docs
pub trait Manager {
    /// TODO: write docs
    async fn list_releases() -> Result<()>;
    /// TODO: write docs
    async fn list_assets() -> Result<()>;
    /// TODO: write docs
    async fn install_release(machine: &'_ Details) -> Result<()>;
    /// TODO: write docs
    async fn update() -> Result<()>;
    /// TODO: write docs
    async fn delete() -> Result<()>;
}

#[derive(Debug, Clone)]
/// TODO: write docs
pub enum ListType {
    /// TODO: write docs
    Local,
    /// TODO: write docs
    Remote(String),
}
