use smd_core::ops::files;
use tauri::command;

#[command]
pub async fn parse_lua_file(path: String) -> Result<serde_json::Value, String> {
    let parsed = files::parse_source_file(&path).await?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}

#[command]
pub async fn parse_lua_content(
    content: String,
    filename: String,
) -> Result<serde_json::Value, String> {
    let parsed = files::parse_lua_content(&content, &filename)?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}
