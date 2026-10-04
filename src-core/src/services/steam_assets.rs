use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

const STORE_ASSETS: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
const LEGACY_APPS: &str = "https://cdn.akamai.steamstatic.com/steam/apps";
const COMMUNITY_ICONS: &str = "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps";
const CACHE_TTL_SECS: i64 = 7 * 24 * 3600;
const STORE_BATCH: usize = 50;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameAssets {
    #[serde(default)]
    pub capsule: Option<String>,
    #[serde(default)]
    pub hero: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub header: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

impl GameAssets {
    fn fill_from(&mut self, other: GameAssets) {
        self.capsule = self.capsule.take().or(other.capsule);
        self.hero = self.hero.take().or(other.hero);
        self.logo = self.logo.take().or(other.logo);
        self.header = self.header.take().or(other.header);
        self.icon = self.icon.take().or(other.icon);
    }
}

pub fn legacy(app_id: &str) -> GameAssets {
    GameAssets {
        capsule: Some(format!("{}/{}/library_600x900.jpg", LEGACY_APPS, app_id)),
        hero: Some(format!("{}/{}/library_hero.jpg", LEGACY_APPS, app_id)),
        logo: Some(format!("{}/{}/logo.png", LEGACY_APPS, app_id)),
        header: Some(format!("{}/{}/header.jpg", LEGACY_APPS, app_id)),
        icon: None,
    }
}

fn store_url(format: &str, file: &str) -> String {
    format!("{}{}", STORE_ASSETS, format.replace("${FILENAME}", file))
}

pub fn parse_store_items(json: &serde_json::Value) -> HashMap<String, GameAssets> {
    let mut out = HashMap::new();
    let Some(items) = json["response"]["store_items"].as_array() else {
        return out;
    };
    for item in items {
        let Some(app) = item["appid"].as_u64().map(|a| a.to_string()) else {
            continue;
        };
        let assets = &item["assets"];
        let Some(format) = assets["asset_url_format"].as_str() else {
            continue;
        };
        let pick = |keys: &[&str]| {
            keys.iter()
                .find_map(|k| assets[*k].as_str().filter(|v| !v.is_empty()))
                .map(|file| store_url(format, file))
        };
        let icon = assets["community_icon"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(|hash| format!("{}/{}/{}.jpg", COMMUNITY_ICONS, app, hash));
        out.insert(
            app,
            GameAssets {
                capsule: pick(&["library_capsule_2x", "library_capsule"]),
                hero: pick(&["library_hero_2x", "library_hero"]),
                logo: pick(&["library_logo_2x", "library_logo"]),
                header: pick(&["header"]),
                icon,
            },
        );
    }
    out
}

pub fn parse_app_info(app_id: &str, common: &serde_json::Value) -> GameAssets {
    let full = &common["library_assets_full"];
    let image = |key: &str| {
        let node = &full[key];
        ["image2x", "image"].iter().find_map(|variant| {
            let v = &node[*variant];
            v["english"]
                .as_str()
                .or_else(|| v.as_object().and_then(|m| m.values().find_map(|x| x.as_str())))
                .filter(|p| !p.is_empty())
                .map(|path| format!("{}steam/apps/{}/{}", STORE_ASSETS, app_id, path))
        })
    };
    GameAssets {
        capsule: image("library_capsule"),
        hero: image("library_hero"),
        logo: image("library_logo"),
        header: image("library_header"),
        icon: common["icon"]
            .as_str()
            .filter(|v| !v.is_empty())
            .map(|hash| format!("{}/{}/{}.jpg", COMMUNITY_ICONS, app_id, hash)),
    }
}

async fn fetch_store(client: &Client, app_ids: &[String]) -> HashMap<String, GameAssets> {
    let mut out = HashMap::new();
    for chunk in app_ids.chunks(STORE_BATCH) {
        let ids: Vec<serde_json::Value> = chunk
            .iter()
            .filter_map(|id| id.parse::<u32>().ok())
            .map(|id| serde_json::json!({ "appid": id }))
            .collect();
        if ids.is_empty() {
            continue;
        }
        let input = serde_json::json!({
            "ids": ids,
            "context": { "language": "english", "country_code": "US" },
            "data_request": { "include_assets": true },
        });
        let resp = client
            .get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
            .query(&[("input_json", input.to_string())])
            .timeout(Duration::from_secs(15))
            .send()
            .await;
        let Ok(resp) = resp else { continue };
        if !resp.status().is_success() {
            continue;
        }
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            out.extend(parse_store_items(&json));
        }
    }
    out
}

async fn fetch_app_info(client: &Client, app_id: &str) -> Option<GameAssets> {
    let resp = client
        .get(format!("https://api.steamcmd.net/v1/info/{}", app_id))
        .header("User-Agent", "SteamManifestDownloader")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    Some(parse_app_info(app_id, &json["data"][app_id]["common"]))
}

pub async fn game_assets(client: &Client, app_id: &str) -> GameAssets {
    let mut assets = fetch_store(client, &[app_id.to_string()])
        .await
        .remove(app_id)
        .unwrap_or_default();
    if assets.logo.is_none() || assets.capsule.is_none() || assets.icon.is_none() {
        if let Some(info) = fetch_app_info(client, app_id).await {
            assets.fill_from(info);
        }
    }
    assets
}

#[derive(Default, Serialize, Deserialize)]
struct CoverCache {
    #[serde(default)]
    covers: HashMap<String, (Option<String>, i64)>,
}

fn cache_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cover_cache.json")
}

pub async fn covers(
    client: &Client,
    app_data_dir: &Path,
    app_ids: &[String],
) -> HashMap<String, String> {
    let now = chrono::Utc::now().timestamp();
    let mut cache: CoverCache = tokio::fs::read(cache_path(app_data_dir))
        .await
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let missing: Vec<String> = app_ids
        .iter()
        .filter(|id| {
            cache
                .covers
                .get(*id)
                .is_none_or(|(_, at)| now - at > CACHE_TTL_SECS)
        })
        .cloned()
        .collect();
    if !missing.is_empty() {
        let found = fetch_store(client, &missing).await;
        let reached = !found.is_empty();
        for id in &missing {
            let cover = found.get(id).and_then(|a| a.capsule.clone());
            if cover.is_some() || reached {
                cache.covers.insert(id.clone(), (cover, now));
            }
        }
        if let Ok(json) = serde_json::to_vec(&cache) {
            let _ = tokio::fs::write(cache_path(app_data_dir), json).await;
        }
    }
    app_ids
        .iter()
        .filter_map(|id| {
            let cover = cache
                .covers
                .get(id)
                .and_then(|(url, _)| url.clone())
                .or_else(|| legacy(id).capsule)?;
            Some((id.clone(), cover))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_items_resolve_hashed_asset_paths() {
        let json = serde_json::json!({ "response": { "store_items": [{
            "appid": 1817070,
            "assets": {
                "asset_url_format": "steam/apps/1817070/${FILENAME}?t=1700000000",
                "library_capsule": "abc123/library_600x900.jpg",
                "library_hero": "library_hero.jpg",
                "header": "header.jpg",
                "community_icon": "ffee"
            }
        }]}});
        let a = &parse_store_items(&json)["1817070"];
        assert_eq!(
            a.capsule.as_deref(),
            Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/1817070/abc123/library_600x900.jpg?t=1700000000")
        );
        assert!(a.hero.as_deref().unwrap().ends_with("/library_hero.jpg?t=1700000000"));
        assert_eq!(a.logo, None);
        assert!(a.icon.as_deref().unwrap().ends_with("/1817070/ffee.jpg"));
    }

    #[test]
    fn app_info_fills_the_logo() {
        let common = serde_json::json!({
            "icon": "aa",
            "library_assets_full": {
                "library_logo": { "image": { "english": "h1/logo.png" } },
                "library_capsule": { "image2x": { "english": "h2/library_600x900_2x.jpg" } }
            }
        });
        let a = parse_app_info("70", &common);
        assert_eq!(
            a.logo.as_deref(),
            Some("https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/70/h1/logo.png")
        );
        assert!(a.capsule.as_deref().unwrap().ends_with("h2/library_600x900_2x.jpg"));
    }

    #[test]
    fn legacy_paths_are_the_fallback() {
        assert!(legacy("70").capsule.unwrap().ends_with("/70/library_600x900.jpg"));
    }
}
