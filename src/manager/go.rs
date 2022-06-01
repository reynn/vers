use scraper::Node;

use {
    super::Asset,
    crate::version::{self, Version},
    async_trait::async_trait,
    scraper::{Html, Selector},
};

const GO_VERSION_LIST: &str = "https://go.dev/dl";

#[derive(Debug)]
pub struct GoManager;

pub struct GoOpts {
    version: Option<Version>,
}

#[async_trait]
impl super::Manager for GoManager {
    async fn get_all_versions(&self, _name: &'_ str) -> crate::Result<Vec<Version>> {
        Self::all_versions().await
    }
    async fn get_latest_version(&self, _name: &'_ str) -> crate::Result<Version> {
        Self::latest_version().await
    }
    async fn get_assets_for_version(
        &self,
        _name: &'_ str,
        version: &'_ Version,
    ) -> crate::Result<Vec<Asset>> {
        Self::version_assets(version).await
    }
    fn name(&self) -> &'static str {
        "go"
    }
}

impl GoManager {
    async fn all_versions() -> crate::Result<Vec<Version>> {
        match reqwest::get(GO_VERSION_LIST).await {
            Ok(resp) => {
                let html_text = resp.text().await?;
                let html = Html::parse_document(&html_text);
                let html_selector = Selector::parse("div.toggle").unwrap();
                Ok(html
                    .select(&html_selector)
                    .into_iter()
                    .filter_map(|v| {
                        let element_value = v.value().clone().id.unwrap_or_default().to_string();
                        if element_value.starts_with("go") {
                            Some(version::parse_version(
                                element_value.strip_prefix("go").unwrap_or_default(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect())
            }
            Err(e) => eyre::bail!(
                "Failed to get list of versions from {}. {:?}",
                GO_VERSION_LIST,
                e
            ),
        }
    }
    async fn latest_version() -> crate::Result<Version> {
        match reqwest::get(GO_VERSION_LIST).await {
            Ok(resp) => {
                let html_text = resp.text().await?;
                let html = Html::parse_document(&html_text);
                let html_selector = Selector::parse("#stable + .toggleVisible").unwrap();

                let list = html
                    .select(&html_selector)
                    .into_iter()
                    .filter_map(|el| {
                        let v = el.value().clone().id.unwrap_or_default().to_string();
                        if v.starts_with("go") {
                            Some(version::parse_version(
                                v.strip_prefix("go").unwrap_or_default(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<Version>>();

                Ok(list.get(0).unwrap().clone())
            }
            Err(e) => eyre::bail!(
                "Failed to get latest version of Go from {}. {:?}",
                GO_VERSION_LIST,
                e
            ),
        }
    }
    async fn version_assets(version: &'_ Version) -> crate::Result<Vec<Asset>> {
        match reqwest::get(GO_VERSION_LIST).await {
            Ok(resp) => {
                // let text = resp.text().await?;
                // let html = Html::parse_document(&text);
                // let html_selector =
                //     Selector::parse(&format!("#{} > .download", version.as_tag())).unwrap();

                // let list = html
                //     .select(&html_selector)
                //     .into_iter()
                //     .filter_map()
                //     .collect();
                Ok(Vec::new())
            }
            Err(e) => eyre::bail!("Failed to get assets for Go {}. {:?}", version.as_tag(), e),
        }
    }
}
