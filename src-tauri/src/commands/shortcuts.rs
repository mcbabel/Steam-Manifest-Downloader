use smd_core::ops::shortcuts;
use tauri::command;

#[command]
pub async fn is_shortcut_supported() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "supported": shortcuts::is_shortcut_supported() }))
}

#[command]
pub async fn detect_executables(download_dir: String) -> Result<serde_json::Value, String> {
    let executables = shortcuts::detect_executables(&download_dir).await?;
    Ok(serde_json::json!({ "executables": executables }))
}

#[command]
pub async fn create_shortcuts(
    exe_path: String,
    game_name: String,
    icon_path: Option<String>,
    create_desktop: bool,
    create_start_menu: bool,
) -> Result<serde_json::Value, String> {
    let result = shortcuts::create_shortcuts(
        exe_path,
        game_name,
        icon_path,
        create_desktop,
        create_start_menu,
    )
    .await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}
