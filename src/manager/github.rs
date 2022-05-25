//! Implementation of the Manager trait pulling information from GitHub releases

use {
    super::{Asset, Manager},
    crate::{
        system::System,
        version::{self, Version},
    },
    async_trait::async_trait,
};

#[derive(Debug)]
pub struct GitHubManager;

#[async_trait]
impl Manager for GitHubManager {
    async fn get_all_versions(&self, name: &'_ str) -> crate::Result<Vec<Version>> {
        let (owner, repo) = self.split_owner_and_repo(name);

        match octocrab::instance()
            .repos(&owner, &repo)
            .releases()
            .list()
            .per_page(100)
            .send()
            .await
        {
            Ok(releases) => Ok(releases
                .into_iter()
                .filter(|release| !release.prerelease)
                .map(|release| version::parse_version(&release.tag_name))
                .collect()),
            Err(release_err) => {
                eyre::bail!(
                    "failed to get releases for {owner}/{repo} error: {:?}",
                    release_err
                )
            }
        }
    }

    async fn get_latest_version(&self, name: &'_ str) -> crate::Result<Version> {
        let (owner, repo) = self.split_owner_and_repo(name);

        match octocrab::instance()
            .repos(&owner, &repo)
            .releases()
            .get_latest()
            .await
        {
            Ok(release) => Ok(version::parse_version(&release.tag_name)),
            Err(release_err) => eyre::bail!(
                "Failed to get latest release for {owner}/{repo} error: {:?}",
                release_err
            ),
        }
    }

    async fn get_assets_for_version(
        &self,
        name: &'_ str,
        version: &'_ Version,
        system: &'_ System,
    ) -> crate::Result<Vec<Asset>> {
        let (owner, repo) = self.split_owner_and_repo(name);
        let assets = match octocrab::instance()
            .repos(&owner, &repo)
            .releases()
            .get_by_tag(&version.as_tag())
            .await
        {
            Ok(tagged_release) => tagged_release.assets,
            Err(_) => {
                match octocrab::instance()
                    .repos(&owner, &repo)
                    .releases()
                    .get_by_tag(format!("v{}", version.as_tag()).as_str())
                    .await
                {
                    Ok(v_prefixed_release) => v_prefixed_release.assets,
                    Err(e) => {
                        eyre::bail!(
                            "Unable to get release {} from GitHub. {:?}",
                            version.as_tag(),
                            e
                        )
                    }
                }
            }
        };

        Ok(assets
            .into_iter()
            .filter_map(|a| Some(a))
            .map(|a| a.into())
            .collect())
    }
}

impl From<octocrab::models::repos::Asset> for Asset {
    fn from(octo_asset: octocrab::models::repos::Asset) -> Self {
        Self {
            name: octo_asset.name.clone(),
            download_url: octo_asset.browser_download_url.clone().into(),
        }
    }
}

impl GitHubManager {
    fn split_owner_and_repo(&self, name: &'_ str) -> (String, String) {
        let split_name = name.split('@').collect::<Vec<_>>()[0];
        let split_owner_repo = split_name
            .split('/')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let owner = split_owner_repo.get(0).unwrap();
        let repo = split_owner_repo.get(1).unwrap();
        (owner.to_string(), repo.to_string())
    }
}
