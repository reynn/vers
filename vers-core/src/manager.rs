pub use async_trait::async_trait;

use crate::{errors::*, machine::Details};

#[async_trait]
pub trait Manager {
    async fn list_releases() -> Result<()>;
    async fn list_assets() -> Result<()>;
    async fn install_release(machine: &'_ Details) -> Result<()>;
    async fn update() -> Result<()>;
    async fn delete() -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum ListType {
    Local,
    Remote(String),
}
