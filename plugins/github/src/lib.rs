use vers_plugin::VersPlugin;

#[no_mangle]
pub fn run() {
    println!("Running GitHub");
}

struct GitHubPlugin;

impl VersPlugin for GitHubPlugin {
    fn get_versions(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    fn get_versions_from_source(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    fn get_assets_for_version(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }
}
