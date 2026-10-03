use std::path::PathBuf;
use tauri::{command, AppHandle, Manager};
use smd_core::services::settings as settings_service;

#[command]
pub async fn get_settings(app: AppHandle) -> Result<serde_json::Value, String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
    let settings = settings_service::seed_defaults_if_needed(&app_data_dir).await;
    serde_json::to_value(&settings).map_err(|e| format!("Failed to serialize settings: {}", e))
}

#[command]
pub async fn save_settings(app: AppHandle, settings: serde_json::Value) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));

    let new_settings: settings_service::Settings = serde_json::from_value(settings)
        .map_err(|e| format!("Invalid settings format: {}", e))?;

    settings_service::save_settings(&app_data_dir, &new_settings).await
}
