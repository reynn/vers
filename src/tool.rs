use {
    crate::{environment::Environment, version::Version},
    serde::{Deserialize, Serialize},
    serde_json::{from_str, to_string_pretty},
    std::path::Path,
    tokio::fs::read_to_string,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub alias: String,
    #[serde(skip)]
    pub current_version: Version,
    #[serde(skip)]
    pub installed_versions: Vec<Version>,
    #[serde(skip)]
    pub known_versions: Vec<Version>,
}

impl Tool {
    pub fn new(name: &'_ str, alias: &'_ str, version: &'_ Version) -> Self {
        Self {
            name: name.to_string(),
            current_version: version.clone(),
            installed_versions: vec![version.clone()],
            known_versions: vec![version.clone()],
            alias: alias.to_string(),
        }
    }

    pub async fn get(env: &'_ Environment, name: &'_ str) -> crate::Result<Self> {
        let tool_path = Path::new(&env.base_dir).join("..").join("tools");
        if !tool_path.exists() {
            eyre::bail!("no tools installed in the current environment");
        }

        match read_to_string(tool_path.join(format!("{}.json", name))).await {
            Ok(file_contents) => match from_str(&file_contents) {
                Ok(s) => Ok(s),
                Err(e) => eyre::bail!(e),
            },
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => eyre::bail!(format!(
                    "No versions of {} are installed in {}",
                    name, env.name
                )),
                _ => eyre::bail!(format!("Unable to load {}.json: {}", name, read_err)),
            },
        }
    }

    pub fn set_current_version(&mut self, version: Version) {
        self.current_version = version;
    }

    pub fn add_version(&mut self, version: &'_ Version) -> crate::Result<()> {
        Ok(())
    }
}
