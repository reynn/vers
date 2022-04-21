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
    pub current_version: Version,
    pub installed_versions: Vec<Version>,
    pub known_versions: Vec<Version>,
}

impl Tool {
    pub async fn get(env: &'_ Environment, name: &'_ str) -> crate::Result<Self> {
        let tool_path = Path::new(&env.base_dir).join("..").join("tools");
        if !tool_path.exists() {
            return Err("no tools installed in the current environment".into());
        }

        match read_to_string(tool_path.join(format!("{}.json", name))).await {
            Ok(file_contents) => {
                from_str(&file_contents).map_err(|serde_err| format!("{}", serde_err).into())
            }
            Err(read_err) => match read_err.kind() {
                std::io::ErrorKind::NotFound => {
                    Err(format!("No versions of {} are installed in {}", name, env.name).into())
                }
                _ => Err(format!("Unable to load {}.json: {}", name, read_err).into()),
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
