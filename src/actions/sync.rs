use crate::{environment::Environment, system::System, tool::Tool, version::parse_version};
use indicatif::ProgressBar;
use tracing::{error, info};

pub async fn sync_tools(env: &mut Environment, system: &'_ System) -> super::Result<()> {
    let tools: Vec<Tool> = env.tools.to_vec();
    let progress_bar = ProgressBar::new(tools.len() as u64);

    for tool in tools.iter() {
        let parsed_version = parse_version(&tool.current_version);
        match super::handle_tool_install(env, tool, system, Some(parsed_version)).await {
            Ok(_) => info!(
                "Tool {} has been installed at version {}",
                &tool.name, tool.current_version
            ),
            Err(install_err) => error!("Failed to install {}. {:?}", &tool.name, install_err),
        }
        progress_bar.inc(1);
    }
    Ok(())
}
