use crate::{
    manager::AssetType,
    system::{OperatingSystem, PlatformArchitecture},
};

use {
    super::Asset,
    crate::version::{self, Version},
    async_trait::async_trait,
    log::*,
    scraper::{Html, Selector},
};

const GO_DEV_BASE: &str = "https://go.dev";

#[derive(Debug)]
pub struct GoManager;

// pub struct GoOpts {
//     version: Option<Version>,
// }

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
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::get(&dl_list).await {
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
            Err(e) => eyre::bail!("Failed to get list of versions from {}. {:?}", dl_list, e),
        }
    }
    async fn latest_version() -> crate::Result<Version> {
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::get(&dl_list).await {
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
                dl_list,
                e
            ),
        }
    }
    async fn version_assets(version: &'_ Version) -> crate::Result<Vec<Asset>> {
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::get(&dl_list).await {
            Ok(resp) => {
                let text = resp.text().await?;
                let document = Html::parse_document(&text);
                let text_selector = format!(
                    "#go{} table.downloadtable > tbody > tr",
                    version.as_tag().replace('.', "\\.")
                );
                debug!("Text Selector: {}", text_selector);
                let selector = Selector::parse(text_selector.as_str()).unwrap();
                debug!("Parsed Selector {:?}", selector);

                Ok(document
                    .select(&selector)
                    .into_iter()
                    .filter_map(|item| {
                        let name = if let Some(i) = item
                            .select(&Selector::parse("td.filename").unwrap())
                            .into_iter()
                            .next()
                        {
                            i.text().next().unwrap_or_default().to_string()
                        } else {
                            return None;
                        };
                        let download_url = if let Some(i) =
                            item.select(&Selector::parse("a.download").unwrap()).next()
                        {
                            format!(
                                "{}{}",
                                GO_DEV_BASE,
                                i.value().attr("href").unwrap_or_default()
                            )
                        } else {
                            return None;
                        };
                        let s = Selector::parse("td").unwrap();
                        let mut selections = item.select(&s);

                        Some(Asset {
                            name,
                            download_url,
                            asset_type: AssetType::parse(
                                if let Some(t) = selections
                                    .nth(1)
                                    .map(|i| i.text().next().unwrap_or_default())
                                {
                                    t
                                } else {
                                    "src"
                                },
                            ),
                            operating_system: OperatingSystem::parse(
                                selections
                                    .next()
                                    .map(|i| i.text().next().unwrap_or_default())
                                    .unwrap_or_default(),
                            ),
                            architecture: PlatformArchitecture::parse(
                                selections
                                    .next()
                                    .map(|i| i.text().next().unwrap_or_default())
                                    .unwrap_or_default(),
                            ),
                        })
                    })
                    .collect())
            }
            Err(e) => eyre::bail!("Failed to get assets for Go {}. {:?}", version.as_tag(), e),
        }
    }
}
