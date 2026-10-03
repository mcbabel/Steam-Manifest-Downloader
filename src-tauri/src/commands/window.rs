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
pub async fn close_window(window: tauri::Window) -> Result<(), String> {
    if let Some(state) = window.try_state::<AppState>() {
        if let Some(telemetry) = state.telemetry.clone() {
            tokio::time::timeout(std::time::Duration::from_secs(3), telemetry.flush())
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
            tokio::time::timeout(std::time::Duration::from_secs(3), telemetry.flush())
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
