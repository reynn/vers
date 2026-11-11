use crate::{
    environment::Environment,
    error::format_error_chain,
    github,
    system::System,
    tool::Tool,
    validation::{parse_tool_name, validate_alias, validate_version},
    version::{parse_version, Version},
};
use skim::{
    prelude::{SkimItemReader, SkimOptionsBuilder},
    Skim,
};
use std::io::Cursor;
use tracing::info;

pub struct Patterns {
    pub asset: Option<String>,
    pub file: Option<String>,
}

pub async fn add_new_tool(
    env: &mut Environment,
    name: &'_ str,
    system: &'_ System,
    patterns: Patterns,
    alias: Option<String>,
    show: bool,
    pre_release: bool,
) -> super::Result<()> {
    let (org_repo, requested_version) = match name.split_once('@') {
        Some((org_repo, version)) => (org_repo, Some(version)),
        None => (name, None),
    };
    let (owner, repo) = parse_tool_name(org_repo)?;
    let alias = alias.unwrap_or_else(|| repo.to_string());
    validate_alias(&alias)?;

    let asset_pattern = &patterns.asset.unwrap_or_default();
    let file_pattern = &patterns.file.unwrap_or_else(|| alias.clone());

    info!("Owner `{owner}`, Repo `{repo}`, Alias `{alias}`, Pattern `{asset_pattern}`, Filter `{file_pattern}`");

    let versions: Vec<String> = if let Some(version) = requested_version {
        vec![version.to_string()]
    } else {
        let versions = match github::get_repo_releases(owner, repo, pre_release).await {
            Ok(res) => res,
            Err(e) => return Err(e.into()),
        };

        // if the user wants a list of the releases show that, otherwise just get the first result
        if show {
            let item_reader =
                SkimItemReader::default().of_bufread(Cursor::new(versions.join("\n")));
            Skim::run_with(
                &SkimOptionsBuilder::default()
                    .height(Some("75%"))
                    .multi(true)
                    .reverse(true)
                    .build()
                    .unwrap(),
                Some(item_reader),
            )
            .map(|items| {
                items
                    .selected_items
                    .iter()
                    .map(|item| item.text().to_string())
                    .collect()
            })
            .unwrap_or_default()
        } else {
            match versions.first() {
                Some(version) => vec![version.into()],
                None => vec![],
            }
            // vec![versions.first().unwrap().to_string()]
        }
    };

    if versions.is_empty() {
        return Err(super::ActionsError::ReleaseNotFound(org_repo.to_string()));
    }

    let mut failures = Vec::new();
    for version in versions.iter() {
        validate_version(version)?;
        let parsed_version = parse_version(version);

        let tool = Tool::new(
            org_repo,
            &alias,
            &Version::Latest,
            asset_pattern,
            file_pattern,
        );

        match super::handle_tool_install(env, &tool, system, Some(parsed_version)).await {
            Ok(_) => println!("Installation of tool {} complete.", tool.name),
            Err(install_err) => failures.push(format!(
                "{}@{}: {}",
                tool.name,
                version,
                format_error_chain(&install_err)
            )),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(super::ActionsError::ToolFailures {
            operation: "install",
            failures: failures.join("; "),
        })
    }
}
