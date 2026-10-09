use tauri::command;
use tauri::Manager;

use smd_core::services::AppState;

#[command]
pub async fn minimize_window(window: tauri::Window) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

#[command]
pub async fn maximize_window(window: tauri::Window) -> Result<(), String> {
    if window.is_maximized().unwrap_or(false) {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[command]
pub fn set_download_progress(
    window: tauri::Window,
    percent: Option<f64>,
    paused: Option<bool>,
    title: String,
) {
    use tauri::window::{ProgressBarState, ProgressBarStatus};
    let status = match (percent, paused.unwrap_or(false)) {
        (None, _) => ProgressBarStatus::None,
        (Some(_), true) => ProgressBarStatus::Paused,
        (Some(_), false) => ProgressBarStatus::Normal,
    };
    let _ = window.set_progress_bar(ProgressBarState {
        status: Some(status),
        progress: percent.map(|p| p.clamp(0.0, 100.0).round() as u64),
    });
    let _ = window.set_title(&title);
}

#[command]
pub async fn close_window(window: tauri::Window) -> Result<(), String> {
    if let Ok(dir) = window.app_handle().path().app_data_dir() {
        smd_core::services::telemetry::clear_active_downloads(&dir);
    }
    if let Some(state) = window.try_state::<AppState>() {
        if let Some(telemetry) = state.telemetry.clone() {
            tokio::time::timeout(std::time::Duration::from_secs(3), telemetry.end_session("close"))
                .await
                .ok();
        }
    }
    window.destroy().map_err(|e| e.to_string())
}

#[command]
pub async fn restart_app(app: tauri::AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(telemetry) = state.telemetry.clone() {
            tokio::time::timeout(std::time::Duration::from_secs(3), telemetry.end_session("restart"))
                .await
                .ok();
        }
    }
    #[cfg(debug_assertions)]
    {
        let _ = app;
        std::process::exit(0);
    }
    #[cfg(not(debug_assertions))]
    {
        app.restart();
    }
}
