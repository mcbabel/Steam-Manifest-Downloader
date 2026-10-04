use smd_core::ops::search::{self, LatestManifestResult};
use smd_core::services::depot_info::{self, DepotInfo};
use smd_core::services::depot_select::{self, Selection};
use smd_core::services::settings;
use smd_core::services::steam_pics::{self, DepotMetadata};
use smd_core::services::steam_store_api;
use smd_core::services::AppState;
use tauri::{command, AppHandle};

use super::app_data_dir;

#[command]
pub async fn search_repos(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<serde_json::Value, String> {
    let result = search::search_repos(&state, &app_data_dir(&app), &app_id).await?;
    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize search result: {}", e))
}

#[command]
pub async fn get_repo_manifests(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
    repo: String,
    sha: Option<String>,
) -> Result<serde_json::Value, String> {
    let result =
        search::get_repo_manifests(&state, &app_data_dir(&app), &app_id, &repo, sha.as_deref())
            .await?;
    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize manifests: {}", e))
}

#[command]
pub async fn get_steam_app_info(
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<serde_json::Value, String> {
    let info =
        steam_store_api::get_game_info(&state.http_client, &state.steam_cache, &app_id).await?;

    match info {
        Some(game_info) => serde_json::to_value(&game_info)
            .map_err(|e| format!("Failed to serialize game info: {}", e)),
        None => Ok(serde_json::Value::Null),
    }
}

#[command]
pub async fn fetch_depot_metadata(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<Vec<DepotInfo>, String> {
    depot_info::fetch_depot_info(&state.http_client, &app_data_dir(&app), &app_id).await
}

#[command]
pub async fn fetch_depot_metadata_steam(
    state: tauri::State<'_, AppState>,
    app_id: String,
) -> Result<Vec<DepotMetadata>, String> {
    let app_id_u: u32 = app_id
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))?;
    steam_pics::fetch_depots_with_names(state.steam_session.clone(), app_id_u).await
}

#[command]
pub async fn recommend_depots(
    app: AppHandle,
    depots: Vec<DepotMetadata>,
    candidates: Vec<String>,
    ui_language: Option<String>,
) -> Result<Selection, String> {
    let settings = settings::load_settings(&app_data_dir(&app)).await;
    let prefs = depot_select::prefs_from_settings(&settings, ui_language.as_deref().unwrap_or(""));
    Ok(depot_select::recommend(&depots, &candidates, &prefs))
}

#[command]
pub async fn fetch_latest_manifest_id(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    app_id: String,
    depot_id: String,
) -> Result<LatestManifestResult, String> {
    search::fetch_latest_manifest_id(&state, &app_data_dir(&app), &app_id, &depot_id).await
}

#[command]
pub async fn search_steam_games(
    state: tauri::State<'_, AppState>,
    query: String,
) -> Result<serde_json::Value, String> {
    let results = search::search_steam_games(&state.http_client, &query).await?;
    serde_json::to_value(&results).map_err(|e| format!("[SteamSearch] Failed to serialize: {}", e))
}
