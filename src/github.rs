use crate::{
    system::System,
    version::{parse_version, Version},
};
use octocrab::models::repos::{Asset, Release};
use regex::Regex;
use skim::prelude::*;
use std::io::Cursor;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("octocrab SDK encountered an error {0}")]
    SdkError(#[from] octocrab::Error),
}

type Result<T, E = GitHubError> = std::result::Result<T, E>;

pub async fn get_repo_releases(
    owner: &'_ str,
    repo: &'_ str,
    pre_release: bool,
) -> Result<Vec<String>> {
    Ok(octocrab::instance()
        .repos(owner, repo)
        .releases()
        .list()
        .per_page(100)
        .send()
        .await?
        .items
        .iter()
        .filter_map(|release| match pre_release {
            true => Some(release.tag_name.to_string()),
            false => match release.prerelease {
                true => None,
                false => Some(release.tag_name.to_string()),
            },
        })
        // .filter(|release| match pre_release {
        //     true => true, // if the user has requested pre-releases we want to return every discovered release
        //     false => !release.prerelease, // filter out the discovered releases, so we ignore all pre-releases
        // })
        // .map(|release| release.tag_name.to_string())
        .collect())
}

pub async fn get_specific_release_for_repo(
    owner: &'_ str,
    repo: &'_ str,
    version: &'_ Version,
) -> Result<Release> {
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
            Err(e) => Err(e.into()),
        }
    } else {
        match octo
            .repos(owner, repo)
            .releases()
            .get_by_tag(&version.as_tag())
            .await
        {
            Ok(tagged_release) => Ok(tagged_release),
            Err(_) => {
                match octo
                    .repos(owner, repo)
                    .releases()
                    .get_by_tag(&format!("v{}", version.as_tag()))
                    .await
                {
                    Ok(tagged_release) => Ok(tagged_release),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }
}

pub async fn get_latest_release_tag(owner: &'_ str, repo: &'_ str) -> Option<Version> {
    match octocrab::instance()
        .repos(owner, repo)
        .releases()
        .get_latest()
        .await
    {
        Ok(tag) => Some(parse_version(&tag.tag_name)),
        Err(_) => None,
    }
}

pub fn get_platform_specific_asset(
    release: &'_ Release,
    system: &'_ System,
    user_pattern: &'_ str,
) -> Option<Asset> {
    let platform_assets: Vec<Asset> = release
        .assets
        .iter()
        .filter_map(|asset| {
            if !user_pattern.is_empty() {
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
            // if we get multiple results for the platform show the user a selector
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
                            .find(|asset| asset.name == item.text())
                            .unwrap()
                    })
                    .collect()
            })
            .unwrap();
            Some(selected_item.get(0).unwrap().to_owned())
        }
        // if we only get one result we can just return that as is
        1 => Some(platform_assets.get(0).unwrap().clone()),
        _ => None,
    }
}
