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
