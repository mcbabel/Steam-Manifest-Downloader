use crate::services::lua_parser::{self, LuaParseResult};
use crate::services::{key_files, st_parser};
use std::collections::HashMap;

pub async fn parse_source_file(path: &str) -> Result<LuaParseResult, String> {
    parse_source_file_for(path, None).await
}

pub async fn parse_source_file_for(path: &str, app_id: Option<u64>) -> Result<LuaParseResult, String> {
    let file_path = std::path::Path::new(path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "lua" => {
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            lua_parser::parse_lua_file(&content).map_err(|e| format!("Invalid .lua file: {}", e))
        }
        "st" => {
            let buffer = tokio::fs::read(path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            st_parser::parse_st_file(&buffer).map_err(|e| format!("Invalid .st file: {}", e))
        }
        e if key_files::EXTENSIONS.contains(&e) => {
            key_files::to_parse_result(&key_files::read(file_path).await?, app_id)
        }
        _ => Err(format!(
            "Unsupported file type: .{}. Expected .lua, .st or a depot key file (.json, .csv, .vdf, .txt)",
            ext
        )),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyFileApp {
    pub app_id: u64,
    pub name: Option<String>,
    pub depots: usize,
}

pub async fn key_file_apps(path: &str) -> Result<Vec<KeyFileApp>, String> {
    let file = key_files::read(std::path::Path::new(path)).await?;
    Ok(key_files::playable_apps(&file)
        .into_iter()
        .map(|a| KeyFileApp { app_id: a.app_id, name: a.name.clone(), depots: a.depots.len() })
        .collect())
}

pub async fn read_key_file(path: &str) -> Result<HashMap<String, String>, String> {
    let file = key_files::read(std::path::Path::new(path)).await?;
    Ok(file.keys.into_iter().map(|(id, key)| (id.to_string(), key)).collect())
}

pub fn parse_lua_content(content: &str, filename: &str) -> Result<LuaParseResult, String> {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "st" {
        return Err(
            ".st files must be parsed from disk (use parse_lua_file with a path).".to_string(),
        );
    }

    lua_parser::parse_lua_file(content).map_err(|e| format!("Invalid lua content: {}", e))
}
