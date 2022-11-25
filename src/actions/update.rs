use crate::{environment::Environment, system::System, tool::Tool, version::parse_version};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{error, info};

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
                    Err(install_err) => failed_tools.push(install_err.to_string()),
                }
                progress_bar.inc(1);
            }
            error!("{}", failed_tools.join("\n"));

            Ok(())
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
            if let Some(tool) = tools.iter().find(|t| t.name == split_name[0]) {
                info!("Tool: {:?}", tool);

                match super::handle_tool_install(env, tool, system, version).await {
                    Ok(_) => info!("Tool {} has been updated.", &tool.name),
                    Err(install_err) => error!("{:?}", install_err),
                }
            } else {
                error!("{} is not found in the environment.", tool_name);
            }
            Ok(())
        }
    }
}
