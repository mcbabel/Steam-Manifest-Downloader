use smd_core::ops::{history as history_ops, system};
use smd_core::services::followup::{self as followup_service, PendingFollowup};
use smd_core::services::history::{self as history_service, HistoryEntry};
use tauri::{command, AppHandle};

use super::app_data_dir;

#[command]
pub async fn get_history(app: AppHandle) -> Result<serde_json::Value, String> {
    let history = history_service::fill_missing_sizes(&app_data_dir(&app)).await;
    serde_json::to_value(&history.entries)
        .map_err(|e| format!("Failed to serialize history: {}", e))
}

#[command]
pub async fn remove_history_entry(
    app: AppHandle,
    entry_id: String,
    delete_files: Option<bool>,
) -> Result<(), String> {
    history_ops::remove_entry(
        &app_data_dir(&app),
        &entry_id,
        delete_files.unwrap_or(false),
    )
    .await
}

#[command]
pub async fn clear_history(
    app: AppHandle,
    delete_resumable_files: Option<bool>,
) -> Result<(), String> {
    history_ops::clear(&app_data_dir(&app), delete_resumable_files.unwrap_or(false)).await
}

#[command]
pub async fn record_history_entry(app: AppHandle, entry: HistoryEntry) -> Result<(), String> {
    history_service::add_entry(&app_data_dir(&app), entry).await
}

#[command]
pub async fn save_pending_followup(app: AppHandle, followup: PendingFollowup) -> Result<(), String> {
    followup_service::save(&app_data_dir(&app), &followup).await
}

#[command]
pub async fn get_pending_followup(app: AppHandle) -> Option<PendingFollowup> {
    followup_service::load(&app_data_dir(&app)).await
}

#[command]
pub async fn clear_pending_followup(app: AppHandle) {
    followup_service::clear(&app_data_dir(&app)).await
}

#[command]
pub async fn open_folder(path: String) -> Result<(), String> {
    system::open_folder(&path)
}

#[command]
pub fn get_installed_depots(dir: String) -> serde_json::Value {
    let list = smd_core::services::install_state::installed(std::path::Path::new(&dir));
    serde_json::to_value(list).unwrap_or(serde_json::Value::Null)
}

#[command]
pub async fn check_game_updates(
    app: AppHandle,
    state: tauri::State<'_, smd_core::services::AppState>,
) -> Result<serde_json::Value, String> {
    let list = history_ops::check_updates(&app_data_dir(&app), state.steam_session.clone()).await?;
    serde_json::to_value(list).map_err(|e| e.to_string())
}

#[command]
pub fn get_launch_exe(app: AppHandle, dir: String) -> Option<String> {
    smd_core::ops::launch::saved_exe(&app_data_dir(&app), &dir)
}

#[command]
pub async fn launch_game(app: AppHandle, dir: String, exe: String) -> Result<serde_json::Value, String> {
    let data_dir = app_data_dir(&app);
    let method = tauri::async_runtime::spawn_blocking(move || {
        let method = smd_core::ops::launch::launch(&exe)?;
        let _ = smd_core::ops::launch::remember_exe(&data_dir, &dir, &exe);
        Ok::<_, String>(method)
    })
    .await
    .map_err(|e| e.to_string())??;
    serde_json::to_value(method).map_err(|e| e.to_string())
}
