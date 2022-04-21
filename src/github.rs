use crate::version;

use {
    crate::{system::System, version::Version},
    log::*,
    octocrab::models::repos::{Asset, Release},
    regex::Regex,
};

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
        octo.repos(owner, repo)
            .releases()
            .get_latest()
            .await
            .map_err(|octo_err| format!("{}", octo_err).into())
    } else {
        octo.repos(owner, repo)
            .releases()
            .get_by_tag(&version.as_tag())
            .await
            .map_err(|octo_err| format!("{}", octo_err).into())
    }
}

pub async fn get_latest_release_tag(owner: &'_ str, repo: &'_ str) -> crate::Result<Version> {
    let tag = octocrab::instance()
        .repos(owner, repo)
        .releases()
        .get_latest()
        .await?;
    version::parse_version(&tag.tag_name)
}

pub fn get_platform_specific_asset(
    release: &'_ Release,
    system: &'_ System,
    user_pattern: Option<String>,
) -> Option<Asset> {
    for asset in release.assets.iter() {
        if let Some(user_pattern) = &user_pattern {
            let r = Regex::new(user_pattern)
                .unwrap_or_else(|_| panic!("{} is not a valid Regular Expression", user_pattern));
            log::debug!("Matching '{}' against '{}'", r.as_str(), &asset.name);
            if r.is_match(&asset.name) {
                return Some(asset.clone());
            }
        } else if system.is_match(&asset.name) {
            info!("Asset info: {:?}", asset.name);
            return Some(asset.clone());
        }
    }
    None
}
