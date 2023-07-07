use octocrab;

#[derive(Debug)]
pub struct VersGithub {}

impl Default for VersGithub {
    fn default() -> Self {
        unimplemented!();
    }
}

impl VersPlugin for VersGithub {
    fn load(&self) {
        octocrab::initialize()
    }

    fn fetch_all_versions(&self) -> Result<Vec<Version>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    fn fetch_version_download(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::new())
    }
}

impl VersGithub {}
