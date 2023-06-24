use crate::{cli, environment::Environment};
use serde::Serialize;
use tabled::{
    settings::{object::Segment, Alignment, Modify, Panel, Style},
    Table, Tabled,
};
use tracing::info;

pub async fn list_tools(
    env: &'_ Environment,
    installed: bool,
    output_type: cli::ListOutputType,
) -> super::Result<()> {
    info!("Listing all tools available in {}", env.name);
    let tools = &env.tools;

    if tools.is_empty() {
        return Err(super::ActionsError::EmptyEnvironment(env.name.to_string()));
    }

    #[derive(Tabled, Serialize, PartialEq, PartialOrd, Eq, Ord)]
    struct ListTool<'a> {
        #[tabled(rename = "Name")]
        name: &'a str,
        #[tabled(rename = "Alias")]
        alias: &'a str,
        #[tabled(rename = "Version")]
        version: &'a str,
    }
    impl<'a> std::fmt::Display for ListTool<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}@{}", self.name, self.version)
        }
    }

    let mut l: Vec<ListTool> = tools
        .iter()
        .flat_map(|t| {
            if installed {
                t.installed_versions
                    .iter()
                    .map(|tt| ListTool {
                        name: &t.name,
                        alias: &t.alias,
                        version: tt,
                    })
                    .collect()
            } else {
                vec![ListTool {
                    name: &t.name,
                    alias: &t.alias,
                    version: &t.current_version,
                }]
            }
        })
        .collect();

    l.sort();
    match output_type {
        cli::ListOutputType::Table => {
            println!(
                "{}",
                Table::new(&l)
                    .with(Panel::header(if installed {
                        "All Installed Versions"
                    } else {
                        "Current Versions Only"
                    }))
                    .with(Panel::footer(format!("{} tools installed", l.len())))
                    .with(Modify::new(Segment::all()).with(Alignment::center()))
                    .with(Style::rounded())
            );
        }
        cli::ListOutputType::Text => l.iter().for_each(|t| println!("{}", t)),
        cli::ListOutputType::Json => {
            println!("{}", serde_json::to_string_pretty(&l).unwrap())
        }
    }

    Ok(())
}
