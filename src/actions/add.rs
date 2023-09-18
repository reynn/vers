use crate::{environment::Environment, github};
use skim::{
    prelude::{SkimItemReader, SkimOptionsBuilder},
    Skim,
};
use std::io::Cursor;
use tracing::{error, info};
use vers_types::{parse_version, System, Version, Tool};

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
    let split_name: Vec<&str> = name.split('@').collect();
    let org_repo = if split_name.len() > 1 {
        split_name[0]
    } else {
        name
    };
    let split_org_repo: Vec<&str> = org_repo.split('/').collect();
    let owner = split_org_repo[0];
    let repo = split_org_repo[1];
    let alias = alias.unwrap_or_else(|| repo.to_string());

    let asset_pattern = &patterns.asset.unwrap_or_default();
    let file_pattern = &patterns.file.unwrap_or_else(|| alias.clone());

    info!("Owner `{owner}`, Repo `{repo}`, Alias `{alias}`, Pattern `{asset_pattern}`, Filter `{file_pattern}`");

    let versions: Vec<String> = if split_name.len() > 1 {
        vec![split_name[1].to_string()]
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
            match versions.get(0) {
                Some(version) => vec![version.into()],
                None => vec![],
            }
            // vec![versions.get(0).unwrap().to_string()]
        }
    };

    for version in versions.iter() {
        let parsed_version = parse_version(version);

        let tool = Tool::new(
            org_repo,
            &alias,
            &Version::Latest,
            asset_pattern,
            file_pattern,
        );

        match super::handle_tool_install(env, &tool, system, Some(parsed_version)).await {
            Ok(_) => println!("Installation of tool {} complete.", &tool.name),
            Err(install_err) => error!("{:?}", install_err),
        }
    }
    Ok(())
}
