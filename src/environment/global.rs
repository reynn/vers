use super::Env;

#[derive(Debug)]
pub struct GlobalEnv;

impl Env for GlobalEnv {
    fn add_tool(
        &self,
        name: &'_ str,
        version: &'_ crate::version::Version,
        asset: &'_ octocrab::models::repos::Asset,
    ) -> crate::Result<()> {
        todo!()
    }

    fn remove_tool(&self, name: &'_ str) -> crate::Result<()> {
        todo!()
    }

    fn change_tool_version(
        &self,
        name: &'_ str,
        new_version: &'_ crate::version::Version,
    ) -> crate::Result<()> {
        todo!()
    }
}
