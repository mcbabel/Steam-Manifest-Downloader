use smd_core::ops::files;
use tauri::command;

#[command]
pub async fn parse_lua_file(path: String, app_id: Option<u64>) -> Result<serde_json::Value, String> {
    let parsed = files::parse_source_file_for(&path, app_id).await?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[command]
pub async fn key_file_apps(path: String) -> Result<Vec<files::KeyFileApp>, String> {
    files::key_file_apps(&path).await
}

#[command]
pub async fn read_key_file(path: String) -> Result<std::collections::HashMap<String, String>, String> {
    files::read_key_file(&path).await
}

#[command]
pub async fn parse_lua_content(
    content: String,
    filename: String,
) -> Result<serde_json::Value, String> {
    let parsed = files::parse_lua_content(&content, &filename)?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}
