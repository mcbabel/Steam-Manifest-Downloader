use std::path::{Path, PathBuf};
use tauri::{command, AppHandle, Manager};

use smd_core::services::steamless::{self, DrmScanEntry, UnpackResult};
use smd_core::services::AppState;

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[command]
pub async fn steamless_scan(game_dir: String) -> Result<Vec<DrmScanEntry>, String> {
    let path = PathBuf::from(&game_dir);
    if !path.exists() {
        return Err(format!("Path not found: {}", game_dir));
    }
    Ok(steamless::scan_game_dir_for_drm(&path))
}

#[command]
pub async fn steamless_unpack(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<String>,
) -> Result<Vec<UnpackResult>, String> {
    let dir = app_data_dir(&app);
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let target = Path::new(t);
        let r = steamless::unpack_with_steamless(&state.http_client, &dir, target).await?;
        results.push(r);
    }
    Ok(results)
}
