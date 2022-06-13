mod assets;

use crate::version::Version;
pub use assets::*;

pub trait Manager: std::fmt::Debug + Sync + Send {
    fn get_all_versions(&self, name: &'_ str) -> eyre::Result<Vec<Version>>;
    fn get_latest_version(&self, name: &'_ str) -> eyre::Result<Version>;
    fn get_assets_for_version(
        &self,
        name: &'_ str,
        version: &'_ Version,
    ) -> eyre::Result<Vec<Asset>>;
    fn name(&self) -> &'static str;
    fn name_required(&self) -> bool {
        true
    }
}
