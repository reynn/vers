use std::path::PathBuf;
use vers_types::{Asset, Version};

pub type PluginResult<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub trait VersPlugin: Default {
    ///
    fn load(&self);
    /// Retrieve a list of versions
    fn list_versions(&self, name: Option<&str>) -> PluginResult<Vec<Version>>;
    ///
    fn get_version(&self, name: Option<&str>, version: &str) -> PluginResult<String>;
    ///
    fn get_version_assets(&self, name: Option<&str>, version: &Version) -> PluginResult<Vec<Asset>>;
    ///
    fn get_download_path(&self, base_path: &PathBuf, name: Option<&str>, version: &Version) -> &PathBuf;
}

#[macro_export]
macro_rules! register_plugin {
    () => {};
}
