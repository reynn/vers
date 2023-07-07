use crate::Version;

pub trait Plugin: Default {
    fn load(&self);
    fn fetch_all_versions(&self) -> Result<Vec<Version>, Box<dyn std::error::Error>>;
    fn fetch_version_download(&self) -> Result<String, Box<dyn std::error::Error>>;
}
