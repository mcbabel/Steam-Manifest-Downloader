use crate::backend::{parse, reply, Cx};
use smd_core::ops::search::{self, LatestManifestResult};
use smd_core::services::depot_info::{self, DepotInfo};
use smd_core::services::depot_select::{self, Selection};
use smd_core::services::settings;
use smd_core::services::steam_pics::{self, DepotMetadata};
use smd_core::services::steam_store_api;


pub async fn search_repos(app: &Cx, app_id: String) -> Result<serde_json::Value, String> {
    let state = app.state();
    let result = search::search_repos(&state, &app.data_dir(), &app_id).await?;
    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize search result: {}", e))
}

pub async fn get_repo_manifests(app: &Cx, app_id: String, repo: String, sha: Option<String>) -> Result<serde_json::Value, String> {
    let state = app.state();
    let result =
        search::get_repo_manifests(&state, &app.data_dir(), &app_id, &repo, sha.as_deref())
            .await?;
    serde_json::to_value(&result).map_err(|e| format!("Failed to serialize manifests: {}", e))
}

pub async fn find_playtests(app: &Cx, app_id: String) -> Result<Vec<steam_pics::Playtest>, String> {
    let state = app.state();
    let app_id: u32 = app_id
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))?;
    Ok(search::find_playtests(&state, app_id).await)
}

pub async fn get_steam_app_info(app: &Cx, app_id: String) -> Result<serde_json::Value, String> {
    let state = app.state();
    let info =
        steam_store_api::get_game_info(&state.http_client, &state.steam_cache, &app_id).await?;

    match info {
        Some(game_info) => serde_json::to_value(&game_info)
            .map_err(|e| format!("Failed to serialize game info: {}", e)),
        None => Ok(serde_json::Value::Null),
    }
}

pub async fn fetch_depot_metadata(app: &Cx, app_id: String) -> Result<Vec<DepotInfo>, String> {
    let state = app.state();
    depot_info::fetch_depot_info(&state.http_client, &app.data_dir(), &app_id).await
}

pub async fn fetch_depot_metadata_steam(app: &Cx, app_id: String) -> Result<Vec<DepotMetadata>, String> {
    let state = app.state();
    let app_id_u: u32 = app_id
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))?;
    steam_pics::fetch_depots_with_names(state.steam_session.clone(), app_id_u).await
}

pub async fn recommend_depots(
    app: &Cx,
    depots: Vec<DepotMetadata>,
    candidates: Vec<String>,
    keyed: Option<Vec<String>>,
    ui_language: Option<String>,
    include_dlc: Option<bool>,
) -> Result<Selection, String> {
    let settings = settings::load_settings(&app.data_dir()).await;
    let mut prefs = depot_select::prefs_from_settings(&settings, ui_language.as_deref().unwrap_or(""));
    if let Some(dlc) = include_dlc {
        prefs.include_dlc = dlc;
    }
    Ok(match keyed {
        Some(keyed) => depot_select::recommend_keyed(&depots, &candidates, |id| keyed.iter().any(|k| k == id), &prefs),
        None => depot_select::recommend(&depots, &candidates, &prefs),
    })
}

pub async fn fetch_latest_manifest_id(app: &Cx, app_id: String, depot_id: String) -> Result<LatestManifestResult, String> {
    let state = app.state();
    search::fetch_latest_manifest_id(&state, &app.data_dir(), &app_id, &depot_id).await
}

pub async fn search_steam_games(app: &Cx, query: String) -> Result<serde_json::Value, String> {
    let state = app.state();
    let results = search::search_steam_games(&state.http_client, &query).await?;
    serde_json::to_value(&results).map_err(|e| format!("[SteamSearch] Failed to serialize: {}", e))
}

pub async fn fetch_latest_manifest_ids(app: &Cx, app_id: String) -> Result<search::LatestManifests, String> {
    let state = app.state();
    search::fetch_latest_manifest_ids(&state, &app.data_dir(), &app_id).await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "search_repos" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(search_repos(app, a.app_id).await)
        }
        "get_repo_manifests" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, repo: String, sha: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(get_repo_manifests(app, a.app_id, a.repo, a.sha).await)
        }
        "find_playtests" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(find_playtests(app, a.app_id).await)
        }
        "get_steam_app_info" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(get_steam_app_info(app, a.app_id).await)
        }
        "fetch_depot_metadata" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(fetch_depot_metadata(app, a.app_id).await)
        }
        "fetch_depot_metadata_steam" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(fetch_depot_metadata_steam(app, a.app_id).await)
        }
        "recommend_depots" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { depots: Vec<DepotMetadata>, candidates: Vec<String>, keyed: Option<Vec<String>>, ui_language: Option<String>, include_dlc: Option<bool>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(recommend_depots(app, a.depots, a.candidates, a.keyed, a.ui_language, a.include_dlc).await)
        }
        "fetch_latest_manifest_id" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, depot_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(fetch_latest_manifest_id(app, a.app_id, a.depot_id).await)
        }
        "search_steam_games" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { query: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(search_steam_games(app, a.query).await)
        }
        "fetch_latest_manifest_ids" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(fetch_latest_manifest_ids(app, a.app_id).await)
        }
        _ => return None,
    })
}
