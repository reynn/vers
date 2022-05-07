use std::io::Cursor;

use {
    crate::{
        system::System,
        version::{parse_version, Version},
    },
    log::*,
    octocrab::models::repos::{Asset, Release},
    regex::Regex,
    skim::prelude::*,
};

pub async fn get_repo_releases(owner: &'_ str, repo: &'_ str) -> super::Result<Vec<String>> {
    Ok(octocrab::instance()
        .repos(owner, repo)
        .releases()
        .list()
        .per_page(100)
        .send()
        .await?
        .items
        .iter()
        .map(|release| release.tag_name.to_string())
        .collect())
}

pub async fn get_specific_release_for_repo(
    owner: &'_ str,
    repo: &'_ str,
    version: &'_ Version,
    system_info: &'_ System,
) -> super::Result<Release> {
    info!(
        "Getting release({}) for {}/{}",
        version.as_tag(),
        owner,
        repo
    );
    let octo = octocrab::instance();
    if version == &Version::Latest {
        match octo.repos(owner, repo).releases().get_latest().await {
            Ok(latest_release) => Ok(latest_release),
            Err(e) => eyre::bail!(e),
        }
    } else {
        match octo
            .repos(owner, repo)
            .releases()
            .get_by_tag(&version.as_tag())
            .await
        {
            Ok(tagged_release) => Ok(tagged_release),
            Err(e) => {
                match octo
                    .repos(owner, repo)
                    .releases()
                    .get_by_tag(&format!("v{}", version.as_tag()))
                    .await
                {
                    Ok(tagged_release) => Ok(tagged_release),
                    Err(e) => eyre::bail!(e),
                }
            }
        }
    }
}

pub async fn get_latest_release_tag(owner: &'_ str, repo: &'_ str) -> Version {
    let tag = octocrab::instance()
        .repos(owner, repo)
        .releases()
        .get_latest()
        .await
        .unwrap();
    parse_version(&tag.tag_name)
}

pub fn get_platform_specific_asset(
    release: &'_ Release,
    system: &'_ System,
    user_pattern: Option<String>,
) -> Option<Asset> {
    let platform_assets: Vec<Asset> = release
        .assets
        .iter()
        .filter_map(|asset| {
            if let Some(user_pattern) = &user_pattern {
                let r = Regex::new(user_pattern).unwrap_or_else(|_| {
                    panic!("{} is not a valid Regular Expression", user_pattern)
                });
                debug!("Matching '{}' against '{}'", r.as_str(), &asset.name);
                if r.is_match(&asset.name) {
                    Some(asset.clone())
                } else {
                    None
                }
            } else if system.is_match(&asset.name) {
                debug!("Asset info: {:?}", asset.name);
                Some(asset.clone())
            } else {
                None
            }
        })
        .collect();
    match &platform_assets.len() {
        2.. => {
            let item_reader = SkimItemReader::default().of_bufread(Cursor::new(
                platform_assets
                    .iter()
                    .map(|a| a.name.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ));
            let selected_item: Vec<Asset> = Skim::run_with(
                &SkimOptionsBuilder::default()
                    .height(Some("75%"))
                    .build()
                    .unwrap(),
                Some(item_reader),
            )
            .map(|items| {
                items
                    .selected_items
                    .iter()
                    .map(|item| {
                        platform_assets
                            .clone()
                            .into_iter()
                            .find(|asset| asset.name == item.text().to_string())
                            .unwrap()
                    })
                    .collect()
            })
            .unwrap();
            Some(selected_item.get(0).unwrap().to_owned())
        }
        1 => Some(platform_assets.get(0).unwrap().clone()),
        _ => None,
    }
}
