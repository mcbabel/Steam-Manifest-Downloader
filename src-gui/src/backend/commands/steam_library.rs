use crate::backend::{parse, plain, reply, Cx};

use smd_core::services::steam_library::{self, ShortcutAdded, SteamInstall};

pub async fn steam_library_detect() -> Result<SteamInstall, String> {
    steam_library::detect_steam()
}

pub async fn local_steam_depot_keys(depot_ids: Vec<String>) -> smd_core::services::steam_local_keys::LocalKeys {
    smd_core::services::steam_local_keys::read(&depot_ids).await
}

pub async fn steam_library_check_dir(path: String) -> Result<SteamInstall, String> {
    steam_library::check_steam_dir(&path)
}

pub async fn steam_library_add(
    app: &Cx,
    app_id: String,
    app_name: String,
    exe_path: String,
    start_dir: String,
    launch_options: Option<String>,
    close_steam: Option<bool>,
) -> Result<ShortcutAdded, String> {
    let state = app.state();
    steam_library::add_to_steam_library(
        &state.http_client,
        &app_id,
        &app_name,
        &exe_path,
        &start_dir,
        launch_options.as_deref().unwrap_or(""),
        close_steam.unwrap_or(false),
    )
    .await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "steam_library_detect" => reply(steam_library_detect().await),
        "local_steam_depot_keys" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { depot_ids: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            plain(local_steam_depot_keys(a.depot_ids).await)
        }
        "steam_library_check_dir" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steam_library_check_dir(a.path).await)
        }
        "steam_library_add" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_id: String, app_name: String, exe_path: String, start_dir: String, launch_options: Option<String>, close_steam: Option<bool>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steam_library_add(app, a.app_id, a.app_name, a.exe_path, a.start_dir, a.launch_options, a.close_steam).await)
        }
        _ => return None,
    })
}
