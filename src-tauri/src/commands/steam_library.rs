use tauri::command;

use smd_core::services::steam_library::{self, ShortcutAdded, SteamInstall};
use smd_core::services::AppState;

#[command]
pub async fn steam_library_detect() -> Result<SteamInstall, String> {
    steam_library::detect_steam()
}

#[command]
pub async fn steam_library_add(
    state: tauri::State<'_, AppState>,
    app_id: String,
    app_name: String,
    exe_path: String,
    start_dir: String,
    launch_options: Option<String>,
) -> Result<ShortcutAdded, String> {
    steam_library::add_to_steam_library(
        &state.http_client,
        &app_id,
        &app_name,
        &exe_path,
        &start_dir,
        launch_options.as_deref().unwrap_or(""),
    )
    .await
}
