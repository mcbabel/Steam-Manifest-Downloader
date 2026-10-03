use std::path::Path;

use crate::services::depot_info;
use crate::services::multi_repo_search::{self, RepoManifests, SearchResult};
use crate::services::settings as settings_service;
use crate::services::AppState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoreSearchHit {
    #[serde(rename = "appId")]
    pub app_id: u64,
    pub name: String,
    pub image: String,
}

pub async fn search_steam_games(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<StoreSearchHit>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let response = client
        .get("https://store.steampowered.com/api/storesearch/")
        .query(&[("term", query), ("l", "english"), ("cc", "US")])
        .send()
        .await
        .map_err(|e| format!("[SteamSearch] Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "[SteamSearch] API returned status {}",
            response.status()
        ));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("[SteamSearch] Failed to parse JSON: {}", e))?;

    let items = data
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(items
        .iter()
        .take(10)
        .filter_map(|item| {
            let id = item.get("id")?.as_u64()?;
            let name = item.get("name")?.as_str()?;
            let image = item
                .get("tiny_image")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            Some(StoreSearchHit {
                app_id: id,
                name: name.to_string(),
                image: image.to_string(),
            })
        })
        .collect())
}

pub async fn search_repos(
    state: &AppState,
    app_data_dir: &Path,
    app_id: &str,
) -> Result<SearchResult, String> {
    let settings = settings_service::load_settings(app_data_dir).await;
    multi_repo_search::search_repos(
        &state.http_client,
        &settings.depot_sources,
        app_id,
        &settings.ryuu_api_key,
        &settings.hubcap_api_key,
        app_data_dir,
    )
    .await
}

pub async fn get_repo_manifests(
    state: &AppState,
    app_data_dir: &Path,
    app_id: &str,
    repo: &str,
    sha: Option<&str>,
) -> Result<RepoManifests, String> {
    let settings = settings_service::load_settings(app_data_dir).await;
    multi_repo_search::get_repo_manifests(
        &state.http_client,
        &settings.depot_sources,
        app_id,
        repo,
        sha.unwrap_or_default(),
        &settings.ryuu_api_key,
        &settings.hubcap_api_key,
        app_data_dir,
    )
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LatestManifestResult {
    #[serde(rename = "manifestId")]
    pub manifest_id: String,
    pub source: String,
}

pub async fn fetch_latest_manifest_id(
    state: &AppState,
    app_data_dir: &Path,
    app_id: &str,
    depot_id: &str,
) -> Result<LatestManifestResult, String> {
    let app_id_u: u32 = app_id
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))?;
    let depot_id_u: u32 = depot_id
        .parse()
        .map_err(|_| format!("Invalid depot id '{}'", depot_id))?;

    match crate::services::steam_pics::fetch_public_manifest_gid(
        state.steam_session.clone(),
        app_id_u,
        depot_id_u,
    )
    .await
    {
        Ok(gid) => {
            return Ok(LatestManifestResult {
                manifest_id: gid,
                source: "steam".to_string(),
            });
        }
        Err(e) => {
            eprintln!(
                "[fetch_latest_manifest_id] Steam PICS failed ({}), falling back to api.steamcmd.net",
                e
            );
        }
    }

    let depots =
        depot_info::fetch_depot_info_fresh(&state.http_client, app_data_dir, app_id).await?;
    let depot = depots
        .into_iter()
        .find(|d| d.depot_id == depot_id)
        .ok_or_else(|| format!("depot {} not listed for app {}", depot_id, app_id))?;
    let gid = depot
        .manifest_gid
        .ok_or_else(|| format!("depot {} has no public manifest", depot_id))?;
    Ok(LatestManifestResult {
        manifest_id: gid,
        source: "steamcmd".to_string(),
    })
}
