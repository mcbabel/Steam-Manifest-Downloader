use std::path::{Path, PathBuf};
use tauri::{command, AppHandle, Manager};

use smd_core::services::emulator::ScannedFile;
use smd_core::services::steam_api_bypass::{self, BypassApplyResult};
use smd_core::services::AppState;

fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

#[command]
pub async fn steam_api_bypass_apply(
    state: tauri::State<'_, AppState>,
    app: AppHandle,
    targets: Vec<ScannedFile>,
) -> Result<Vec<BypassApplyResult>, String> {
    let dir = app_data_dir(&app);
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = Path::new(&t.path);
        let x64 = t.arch == "x64";
        let r = steam_api_bypass::apply_to_target(&state.http_client, &dir, path, x64).await;
        results.push(r);
    }
    Ok(results)
}

#[command]
pub async fn steam_api_bypass_revert(targets: Vec<String>) -> Result<Vec<bool>, String> {
    let mut out = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = PathBuf::from(t);
        out.push(steam_api_bypass::revert_for_target(&path).is_ok());
    }
    Ok(out)
}

#[command]
pub async fn steam_api_bypass_status(targets: Vec<String>) -> Result<bool, String> {
    for t in &targets {
        let path = PathBuf::from(t);
        if steam_api_bypass::is_installed_for_target(&path) {
            return Ok(true);
        }
    }
    Ok(false)
}
