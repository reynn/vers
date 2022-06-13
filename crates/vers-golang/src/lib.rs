use {
    log::*,
    vers::{
        manager::{Asset, AssetType, Manager},
        system::{OperatingSystem, PlatformArchitecture},
        version::{self, Version},
    },
    wasm_bindgen::prelude::*,
};

const GO_DEV_BASE: &str = "https://go.dev";

#[derive(Debug)]
pub struct GoManager;

impl Manager for GoManager {
    #[wasm_bindgen]
    fn get_all_versions(&self, _: &'_ str) -> eyre::Result<Vec<Version>> {
        Self::all_versions()
    }
    #[wasm_bindgen]
    fn get_latest_version(&self, _: &'_ str) -> eyre::Result<Version> {
        Self::latest_version()
    }
    #[wasm_bindgen]
    fn get_assets_for_version(
        &self,
        _name: &'_ str,
        version: &'_ Version,
    ) -> eyre::Result<Vec<Asset>> {
        Self::version_assets(version)
    }
    #[wasm_bindgen]
    fn name(&self) -> &'static str {
        "go"
    }
}

impl GoManager {
    fn all_versions() -> eyre::Result<Vec<Version>> {
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::blocking::get(&dl_list) {
            Ok(resp) => {
                let html_text = resp.text()?;
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
    fn latest_version() -> eyre::Result<Version> {
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::blocking::get(&dl_list) {
            Ok(resp) => {
                let html_text = resp.text()?;
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
    fn version_assets(version: &'_ Version) -> eyre::Result<Vec<Asset>> {
        let dl_list = format!("{}/dl", GO_DEV_BASE);
        match reqwest::blocking::get(&dl_list) {
            Ok(resp) => {
                let text = resp.text()?;
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
