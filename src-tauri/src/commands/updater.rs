use tauri::{command, AppHandle, Emitter};
use smd_core::services::settings as settings_service;

use super::app_data_dir;

const USER_AGENT: &str = "SteamManifestDownloader";

#[command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    let current_version = app.config().version.clone().unwrap_or_default();
    smd_core::ops::updater::check_for_updates(&current_version).await
}

#[command]
pub async fn install_update(
    app: AppHandle,
    installer_url: String,
    update_kind: Option<String>,
    asset_digest: Option<String>,
) -> Result<(), String> {
    if update_kind.as_deref() == Some("appimage") {
        let progress_app = app.clone();
        let path = smd_core::ops::updater::install_appimage(&installer_url, asset_digest.as_deref(), move |done, total| {
            let _ = progress_app.emit("update-progress", serde_json::json!({ "done": done, "total": total }));
        })
        .await?;
        smd_core::ops::updater::relaunch_appimage(&path)?;
        app.exit(0);
        return Ok(());
    }
    if !cfg!(target_os = "windows") {
        return Err("Updates for this build are installed from the release page".into());
    }

    let client = smd_core::services::net::client();

    let temp_dir = std::env::temp_dir().join("SteamManifestDownloader");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let installer_path = temp_dir.join("update-installer.exe");

    let response = client
        .get(&installer_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", smd_core::services::net::describe(&e)))?;

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

    // Exit so the NSIS installer can replace the running binary.
    app.exit(0);
    Ok(())
}

#[command]
pub async fn get_auto_update_enabled(app: AppHandle) -> Result<bool, String> {
    let settings = settings_service::load_settings(&app_data_dir(&app)).await;
    Ok(settings.auto_update)
}
