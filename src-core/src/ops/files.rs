use crate::services::lua_parser::{self, LuaParseResult};
use crate::services::st_parser;

pub async fn parse_source_file(path: &str) -> Result<LuaParseResult, String> {
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
        _ => Err(format!(
            "Unsupported file type: .{}. Expected .lua or .st",
            ext
        )),
    }
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
