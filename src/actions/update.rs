use crate::{
    environment::Environment, error::format_error_chain, system::System, tool::Tool,
    version::parse_version,
};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;

pub enum UpdateType {
    All,
    Specific(String),
}

pub async fn update_tools(
    env: &mut Environment,
    system: &'_ System,
    update_type: UpdateType,
) -> super::Result<()> {
    match update_type {
        UpdateType::All => {
            let tools: Vec<Tool> = env.tools.to_vec();
            let progress_bar = ProgressBar::new(tools.len() as u64);
            progress_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{bar:75.cyan/blue} {pos:>7}/{len:7} {msg}")
                    .unwrap(),
            );

            let mut failed_tools = Vec::new();
            for tool in tools {
                progress_bar.set_message(tool.name.clone());
                match super::handle_tool_install(env, &tool, system, None).await {
                    Ok(_) => info!("Tool {} complete.", &tool.name),
                    Err(install_err) => failed_tools.push(format!(
                        "{}: {}",
                        tool.name,
                        format_error_chain(&install_err)
                    )),
                }
                progress_bar.inc(1);
            }

            progress_bar.finish_and_clear();
            if failed_tools.is_empty() {
                Ok(())
            } else {
                Err(super::ActionsError::ToolFailures {
                    operation: "update",
                    failures: failed_tools.join("; "),
                })
            }
        }
        UpdateType::Specific(tool_name) => {
            println!("-> Updating {tool_name}...");
            let tools = env.tools.to_vec();
            let split_name: Vec<&str> = tool_name.split('@').collect();
            let version = if split_name.len() == 2 {
                Some(parse_version(split_name[1]))
            } else {
                None
            };

            if let Some(tool) = tools
                .iter()
                .find(|t| t.name == split_name[0] || t.alias == split_name[0])
            {
                info!("Updating {}@{}", tool.name, tool.current_version);

                super::handle_tool_install(env, tool, system, version).await?;
                info!("{} has been updated.", &tool.name);
            } else {
                return Err(super::ActionsError::ToolNotFound {
                    tool_name,
                    env_name: env.name.clone(),
                });
            }
            Ok(())
        }
    }
}
