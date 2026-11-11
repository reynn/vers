use crate::{
    environment::Environment, error::format_error_chain, system::System, tool::Tool,
    version::Version,
};
use indicatif::ProgressBar;
use tracing::info;

pub async fn sync_tools(env: &mut Environment, system: &'_ System) -> super::Result<()> {
    let tools: Vec<Tool> = env.tools.to_vec();
    let progress_bar = ProgressBar::new(tools.len() as u64);

    let mut failures = Vec::new();
    for tool in tools.iter() {
        // Preserve the stored release tag exactly when repairing an install.
        let parsed_version = if tool.current_version == "latest" {
            Version::Latest
        } else {
            Version::Simple(tool.current_version.clone())
        };
        match super::handle_tool_install(env, tool, system, Some(parsed_version)).await {
            Ok(_) => info!(
                "Tool {} has been installed at version {}",
                &tool.name, tool.current_version
            ),
            Err(install_err) => failures.push(format!(
                "{}: {}",
                tool.name,
                format_error_chain(&install_err)
            )),
        }
        progress_bar.inc(1);
    }
    progress_bar.finish_and_clear();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(super::ActionsError::ToolFailures {
            operation: "sync",
            failures: failures.join("; "),
        })
    }
}
