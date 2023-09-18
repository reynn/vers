use octocrab::models::repos::{Asset, Release};
use regex::Regex;
use std::path::PathBuf;
use vers_types::*;
use vers_plugin::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {}

#[derive(Debug, Default)]
struct VersGithub;

impl VersPlugin for VersGithub {
    fn load(&self) {
        todo!()
    }

    fn list_versions(&self, name: Option<&str>) -> PluginResult<Vec<Version>> {
        todo!()
    }

    fn get_version(&self, name: Option<&str>, version: &str) -> PluginResult<String> {
        todo!()
    }

    fn get_version_assets(&self, name: Option<&str>, version: &Version) -> PluginResult<Vec<vers_types::Asset>> {
        todo!()
    }

    fn get_download_path(&self, base_path: &PathBuf, name: Option<&str>, version: &Version) -> &PathBuf {
        todo!()
    }
}

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
        .collect())
}

pub async fn get_specific_release_for_repo(
    owner: &'_ str,
    repo: &'_ str,
    version: &'_ Version,
) -> Result<Release> {
    tracing::info!(
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
) -> Vec<Asset> {
    release
        .assets
        .iter()
        .filter_map(|asset| -> Option<Asset> {
            if !user_pattern.is_empty() {
                let r = Regex::new(user_pattern).unwrap_or_else(|_| {
                    panic!("{} is not a valid Regular Expression", user_pattern)
                });
                tracing::debug!("Matching '{}' against '{}'", r.as_str(), &asset.name);
                if r.is_match(&asset.name) {
                    Some(asset.clone())
                } else {
                    None
                }
            } else if system.is_match(&asset.name) {
                tracing::debug!("Asset info: {:?}", asset.name);
                Some(asset.clone())
            } else {
                None
            }
        })
        .collect()
}
