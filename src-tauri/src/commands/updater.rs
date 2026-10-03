use tauri::{command, AppHandle};
use smd_core::services::settings as settings_service;

use super::app_data_dir;

const USER_AGENT: &str = "SteamManifestDownloader";

#[command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    let current_version = app.config().version.clone().unwrap_or_default();
    smd_core::ops::updater::check_for_updates(&current_version).await
}

#[command]
pub async fn install_update(app: AppHandle, installer_url: String) -> Result<(), String> {
    let client = reqwest::Client::new();

    let temp_dir = std::env::temp_dir().join("SteamManifestDownloader");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let installer_path = temp_dir.join("update-installer.exe");

    let response = client
        .get(&installer_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("Failed to save installer: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&installer_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    // No silent installer path on Linux — just open the release in the browser.
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(&installer_url)
            .spawn();
    }

    // Exit so the NSIS installer can replace the running binary.
    app.exit(0);
    Ok(())
}

#[command]
pub async fn get_auto_update_enabled(app: AppHandle) -> Result<bool, String> {
    let settings = settings_service::load_settings(&app_data_dir(&app)).await;
    Ok(settings.auto_update)
}
