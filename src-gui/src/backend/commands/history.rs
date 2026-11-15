use crate::backend::{parse, plain, reply, Cx};
use smd_core::ops::{history as history_ops, system};
use smd_core::services::followup::{self as followup_service, PendingFollowup};
use smd_core::services::history::{self as history_service, HistoryEntry};


pub async fn get_history(app: &Cx) -> Result<serde_json::Value, String> {
    let history = history_service::fill_missing_sizes(&app.data_dir()).await;
    serde_json::to_value(&history.entries)
        .map_err(|e| format!("Failed to serialize history: {}", e))
}

pub async fn remove_history_entry(app: &Cx, entry_id: String, delete_files: Option<bool>) -> Result<(), String> {
    history_ops::remove_entry(
        &app.data_dir(),
        &entry_id,
        delete_files.unwrap_or(false),
    )
    .await
}

pub async fn clear_history(app: &Cx, delete_resumable_files: Option<bool>) -> Result<(), String> {
    history_ops::clear(&app.data_dir(), delete_resumable_files.unwrap_or(false)).await
}

pub async fn record_history_entry(app: &Cx, entry: HistoryEntry) -> Result<(), String> {
    history_service::add_entry(&app.data_dir(), entry).await
}

pub async fn save_pending_followup(app: &Cx, followup: PendingFollowup) -> Result<(), String> {
    followup_service::save(&app.data_dir(), &followup).await
}

pub async fn get_pending_followup(app: &Cx) -> Option<PendingFollowup> {
    followup_service::load(&app.data_dir()).await
}

pub async fn clear_pending_followup(app: &Cx, download_dir: Option<String>) {
    match download_dir {
        Some(dir) => followup_service::remove(&app.data_dir(), &dir).await,
        None => followup_service::clear(&app.data_dir()).await,
    }
}

pub async fn open_folder(path: String) -> Result<(), String> {
    system::open_folder(&path)
}

pub fn get_installed_depots(dir: String) -> serde_json::Value {
    let list = smd_core::services::install_state::installed(std::path::Path::new(&dir));
    serde_json::to_value(list).unwrap_or(serde_json::Value::Null)
}

pub async fn check_game_updates(app: &Cx) -> Result<serde_json::Value, String> {
    let state = app.state();
    let list = history_ops::check_updates(&app.data_dir(), state.steam_session.clone()).await?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

pub fn get_launch_exe(app: &Cx, dir: String) -> Option<String> {
    smd_core::ops::launch::saved_exe(&app.data_dir(), &dir)
}

pub async fn launch_game(app: &Cx, dir: String, exe: String) -> Result<serde_json::Value, String> {
    let data_dir = app.data_dir();
    let method = tokio::task::spawn_blocking(move || {
        let method = smd_core::ops::launch::launch(&exe)?;
        let _ = smd_core::ops::launch::remember_exe(&data_dir, &dir, &exe);
        Ok::<_, String>(method)
    })
    .await
    .map_err(|e| e.to_string())??;
    serde_json::to_value(method).map_err(|e| e.to_string())
}

pub async fn get_covers(app: &Cx, app_ids: Vec<String>) -> Result<std::collections::HashMap<String, String>, String> {
    let state = app.state();
    Ok(smd_core::services::steam_assets::covers(&state.http_client, &app.data_dir(), &app_ids).await)
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "get_history" => reply(get_history(app).await),
        "remove_history_entry" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { entry_id: String, delete_files: Option<bool>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(remove_history_entry(app, a.entry_id, a.delete_files).await)
        }
        "clear_history" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { delete_resumable_files: Option<bool>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(clear_history(app, a.delete_resumable_files).await)
        }
        "record_history_entry" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { entry: HistoryEntry, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(record_history_entry(app, a.entry).await)
        }
        "save_pending_followup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { followup: PendingFollowup, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(save_pending_followup(app, a.followup).await)
        }
        "get_pending_followup" => plain(get_pending_followup(app).await),
        "clear_pending_followup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { download_dir: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            clear_pending_followup(app, a.download_dir).await;
            plain(())
        }
        "open_folder" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(open_folder(a.path).await)
        }
        "get_installed_depots" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            plain(get_installed_depots(a.dir))
        }
        "check_game_updates" => reply(check_game_updates(app).await),
        "get_launch_exe" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            plain(get_launch_exe(app, a.dir))
        }
        "launch_game" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { dir: String, exe: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(launch_game(app, a.dir, a.exe).await)
        }
        "get_covers" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { app_ids: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(get_covers(app, a.app_ids).await)
        }
        _ => return None,
    })
}
