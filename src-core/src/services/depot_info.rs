use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const STEAMCMD_API: &str = "https://api.steamcmd.net/v1/info/";
const CACHE_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotInfo {
    pub depot_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub oslist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub osarch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub download_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub manifest_gid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheFile {
    fetched_at: i64,
    depots: Vec<DepotInfo>,
}

fn cache_path(app_data_dir: &Path, app_id: &str) -> PathBuf {
    app_data_dir
        .join("depot_info_cache")
        .join(format!("{}.json", app_id))
}

async fn read_cache(path: &Path) -> Option<CacheFile> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    serde_json::from_str(&contents).ok()
}

async fn write_cache(path: &Path, cache: &CacheFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create cache dir: {}", e))?;
    }
    let json = serde_json::to_string(cache).map_err(|e| format!("serialize cache: {}", e))?;
    tokio::fs::write(path, json)
        .await
        .map_err(|e| format!("write cache: {}", e))
}

pub async fn fetch_depot_info(
    client: &Client,
    app_data_dir: &Path,
    app_id: &str,
) -> Result<Vec<DepotInfo>, String> {
    fetch_depot_info_inner(client, app_data_dir, app_id, false).await
}

pub async fn fetch_depot_info_fresh(
    client: &Client,
    app_data_dir: &Path,
    app_id: &str,
) -> Result<Vec<DepotInfo>, String> {
    fetch_depot_info_inner(client, app_data_dir, app_id, true).await
}

async fn fetch_depot_info_inner(
    client: &Client,
    app_data_dir: &Path,
    app_id: &str,
    bypass_cache: bool,
) -> Result<Vec<DepotInfo>, String> {
    let cache_file = cache_path(app_data_dir, app_id);
    if !bypass_cache {
        if let Some(cached) = read_cache(&cache_file).await {
            let age = Utc::now().timestamp() - cached.fetched_at;
            if age >= 0 && age < CACHE_TTL_SECONDS {
                return Ok(cached.depots);
            }
        }
    }

    let url = format!("{}{}", STEAMCMD_API, app_id);
    let resp = client
        .get(&url)
        .header("User-Agent", "SteamManifestDownloader")
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("steamcmd API request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("steamcmd API returned HTTP {}", resp.status()));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("steamcmd API returned invalid JSON: {}", e))?;

    let depots_value = &json["data"][app_id]["depots"];
    let depots_obj = depots_value
        .as_object()
        .ok_or_else(|| format!("steamcmd response missing depots for app {}", app_id))?;

    let mut depots = Vec::new();
    for (key, value) in depots_obj.iter() {
        if !key.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let config = &value["config"];
        let manifest = &value["manifests"]["public"];
        depots.push(DepotInfo {
            depot_id: key.clone(),
            oslist: config["oslist"].as_str().map(String::from),
            osarch: config["osarch"].as_str().map(String::from),
            language: config["language"].as_str().map(String::from),
            size_bytes: manifest["size"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok()),
            download_bytes: manifest["download"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok()),
            manifest_gid: manifest["gid"].as_str().map(String::from),
        });
    }

    let cache = CacheFile {
        fetched_at: Utc::now().timestamp(),
        depots: depots.clone(),
    };
    let _ = write_cache(&cache_file, &cache).await;

    Ok(depots)
}
