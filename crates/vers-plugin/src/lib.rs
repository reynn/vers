mod macros;

pub struct PluginConfig {}

pub trait VersPlugin {
    fn get_versions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn get_versions_from_source(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    fn get_assets_for_version(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
}
