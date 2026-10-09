use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::{safe_file_name, COMMUNITY_IMAGES};
use crate::services::steam_assets;

pub const MEDIA_DIR: &str = "steam_media";
const STORE_IMAGES: &str = "https://cdn.akamai.steamstatic.com/steam/apps";
const CONCURRENCY: usize = 8;
const PREFERRED_VIDEOS: [&str; 3] = ["trailer", "gameplay", "announcement"];
const APP_IMAGES: [&str; 19] = [
    "capsule_184x69.jpg",
    "capsule_231x87.jpg",
    "capsule_231x87_alt_assets_0.jpg",
    "capsule_467x181.jpg",
    "capsule_616x353.jpg",
    "capsule_616x353_alt_assets_0.jpg",
    "library_600x900.jpg",
    "library_600x900_2x.jpg",
    "library_hero.jpg",
    "broadcast_left_panel.jpg",
    "broadcast_right_panel.jpg",
    "page.bg.jpg",
    "page_bg_raw.jpg",
    "page_bg_generated.jpg",
    "page_bg_generated_v6b.jpg",
    "header.jpg",
    "header_alt_assets_0.jpg",
    "hero_capsule.jpg",
    "logo.png",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaLevel {
    #[default]
    Off,
    Images,
    All,
}

impl MediaLevel {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "images" => Self::Images,
            "all" => Self::All,
            _ => Self::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Images => "images",
            Self::All => "all",
        }
    }
}

pub struct CommunityHashes {
    pub clienticon: Option<String>,
    pub icon: Option<String>,
    pub logo: Option<String>,
    pub logo_small: Option<String>,
}

fn strip_query(url: &str) -> &str {
    url.split(['?', '#']).next().unwrap_or(url)
}

pub fn screenshots(details: &Value) -> Vec<String> {
    details["screenshots"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|s| s["path_full"].as_str())
                .map(|u| strip_query(u).to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub fn video(details: &Value) -> Option<(String, String)> {
    let list = details["movies"].as_array()?;
    let mut first = None;
    for m in list {
        let pick = |fmt: &str| {
            m[fmt]["max"]
                .as_str()
                .or_else(|| m[fmt]["480"].as_str())
                .map(|u| (strip_query(u).to_string(), fmt.to_string()))
        };
        let Some((url, ext)) = pick("mp4").or_else(|| pick("webm")) else { continue };
        let name: String = m["name"]
            .as_str()
            .unwrap_or_default()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let name = name.trim_matches('_').to_string();
        let file = if name.is_empty() {
            safe_file_name(&url)?
        } else {
            format!("{}.{}", name, ext)
        };
        let lower = file.to_ascii_lowercase();
        if PREFERRED_VIDEOS.iter().any(|p| lower.contains(p)) {
            return Some((url, file));
        }
        first.get_or_insert((url, file));
    }
    first
}

async fn store_details(http: &reqwest::Client, app_id: &str) -> Option<Value> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={}", app_id);
    let body: Value = http.get(&url).send().await.ok()?.error_for_status().ok()?.json().await.ok()?;
    let entry = &body[app_id];
    entry["success"].as_bool().unwrap_or(false).then(|| entry["data"].clone())
}

async fn download_all(http: &reqwest::Client, jobs: Vec<(String, PathBuf)>) -> usize {
    stream::iter(jobs)
        .map(|(url, path)| async move {
            if path.is_file() {
                return Some(());
            }
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok()?;
            }
            let bytes = http.get(&url).send().await.ok()?.error_for_status().ok()?.bytes().await.ok()?;
            if bytes.is_empty() {
                return None;
            }
            tokio::fs::write(&path, &bytes).await.ok()
        })
        .buffer_unordered(CONCURRENCY)
        .filter(|r| std::future::ready(r.is_some()))
        .count()
        .await
}

pub async fn download(
    http: &reqwest::Client,
    root: &Path,
    app_id: &str,
    level: MediaLevel,
    hashes: &CommunityHashes,
    inventory_icons: &[String],
) -> usize {
    if level == MediaLevel::Off {
        return 0;
    }
    let base = root.join(MEDIA_DIR);
    let images = base.join("images");
    let mut jobs: Vec<(String, PathBuf)> = Vec::new();
    let assets = steam_assets::game_assets(http, app_id).await;
    for url in [assets.capsule, assets.hero, assets.logo, assets.header].into_iter().flatten() {
        if let Some(name) = safe_file_name(strip_query(&url)) {
            jobs.push((url, images.join(name)));
        }
    }
    for name in APP_IMAGES {
        jobs.push((format!("{}/{}/{}", STORE_IMAGES, app_id, name), images.join(name)));
    }
    let community = [
        (&hashes.clienticon, "clienticon", "ico"),
        (&hashes.icon, "icon", "jpg"),
        (&hashes.logo, "logo_community", "jpg"),
        (&hashes.logo_small, "logo_small", "jpg"),
    ];
    for (hash, name, ext) in community {
        if let Some(h) = hash.as_deref().filter(|h| h.chars().all(|c| c.is_ascii_hexdigit())) {
            jobs.push((format!("{}/{}/{}.{}", COMMUNITY_IMAGES, app_id, h, ext), images.join(format!("{}.{}", name, ext))));
        }
    }
    for url in inventory_icons {
        if let Some(name) = safe_file_name(strip_query(url)) {
            jobs.push((url.clone(), base.join("inventory").join(name)));
        }
    }
    let details = store_details(http, app_id).await;
    if let Some(d) = &details {
        for url in screenshots(d) {
            if let Some(name) = safe_file_name(&url) {
                jobs.push((url, base.join("screenshots").join(name)));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    jobs.retain(|(_, p)| seen.insert(p.clone()));
    let mut count = download_all(http, jobs).await;
    if level == MediaLevel::All {
        if let Some((url, file)) = details.as_ref().and_then(video) {
            count += download_all(http, vec![(url, base.join("videos").join(file))]).await;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_screenshots_and_the_trailer() {
        let d = serde_json::json!({
            "screenshots": [ { "path_full": "https://cdn/ss_1.jpg?t=1" } ],
            "movies": [
                { "name": "Teaser", "webm": { "480": "https://cdn/a.webm?t=2" } },
                { "name": "Launch Trailer", "mp4": { "max": "https://cdn/b.mp4" } }
            ]
        });
        assert_eq!(screenshots(&d), vec!["https://cdn/ss_1.jpg"]);
        assert_eq!(video(&d), Some(("https://cdn/b.mp4".into(), "Launch_Trailer.mp4".into())));
        assert_eq!(MediaLevel::parse("ALL"), MediaLevel::All);
        assert_eq!(MediaLevel::parse("x"), MediaLevel::Off);
    }
}
