use smd_core::ops::download::{self, DownloadConfig};
use smd_core::services::AppState;
use tauri::{command, AppHandle};

use super::{app_data_dir, sink};

#[command]
pub async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    config: DownloadConfig,
) -> Result<serde_json::Value, String> {
    download::start_download(sink(&app), &state, &app_data_dir(&app), config).await
}

#[command]
pub async fn cancel_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    download::cancel_download(&sink(&app), &state, &app_data_dir(&app), job_id).await
}

#[command]
pub async fn pause_download(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    job_id: String,
    paused: bool,
) -> Result<(), String> {
    download::pause_download(&sink(&app), &state, job_id, paused).await
}
