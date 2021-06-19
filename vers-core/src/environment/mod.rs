mod errors;
mod tool;

use log::*;
use std::path::{Path, PathBuf};

pub use self::{errors::*, tool::Tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment<'env> {
    name: &'env str,
    tools: Vec<&'env Tool<'env>>,
    directory: Option<PathBuf>,
}

impl<'env> Drop for Environment<'env> {
    fn drop(&mut self) {
        self.save().unwrap();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvFindOptions<'find_tool> {
    Name(String),
    Tool(Tool<'find_tool>),
}

impl<'env> Environment<'env> {
    ///
    pub fn new(name: &'env str) -> Result<Environment<'env>> {
        Ok(Self {
            name,
            tools: Vec::new(),
            directory: None,
        })
    }
    ///
    pub fn save(&self) -> Result<&'env Environment> {
        if let Some(dir) = &self.directory {
            std::fs::create_dir_all(dir)?;
        }
        Ok(self)
    }

    ///
    pub fn find_envs(f: &'_ EnvFindOptions, base_path: &Path) -> Result<Vec<Environment<'env>>> {
        match f {
            EnvFindOptions::Name(s) => Ok(vec![Self::find_env_by_name(s, base_path)?]),
            EnvFindOptions::Tool(t) => Self::find_envs_by_tool(t, base_path), // _ => Err(EnvironmentError::General("".into())),
        }
    }

    /// Attempts to find an existing [`Environment`]
    ///
    /// If unable to find an environment in the provided base path,
    /// returns a [`EnvironmentError::EnvironmentNotFoundByName`]
    fn find_env_by_name(s: &'_ str, base_path: &Path) -> Result<Environment<'env>> {
        info!("Finding environment named {} in {:?}", s, &base_path);
        // base_path
        //     .read_dir()
        //     .expect("read_dir call failed")
        //     .filter(|entry| {
        //         if let Ok(entry) = entry {
        //             println!("{:?}", entry.path());
        //         }
        //     })
        Err(EnvironmentError::EnvironmentNotFoundByName(s.into()))
    }

    /// Attempts to find existing [`Environment`]s that contain a tool, if a tool is provided will only return
    ///
    fn find_envs_by_tool(t: &'_ Tool, base_path: &Path) -> Result<Vec<Environment<'env>>> {
        info!(
            "Finding environments containing tool {:?} in {:?}",
            t, &base_path
        );
        Err(EnvironmentError::EnvironmentNotFoundByTool(t.name.into()))
    }

    // fn get_environments() -> Result<Vec<Environment<'env>>> {
    //     Ok()
    // }
}

impl<'env> Default for Environment<'env> {
    fn default() -> Self {
        Self::new("default").unwrap()
    }
}
