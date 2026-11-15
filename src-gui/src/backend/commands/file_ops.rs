use crate::backend::{parse, reply, Cx};
use smd_core::ops::files;

pub async fn parse_lua_file(path: String, app_id: Option<u64>) -> Result<serde_json::Value, String> {
    let parsed = files::parse_source_file_for(&path, app_id).await?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}

pub async fn key_file_apps(path: String) -> Result<Vec<files::KeyFileApp>, String> {
    files::key_file_apps(&path).await
}

pub async fn read_key_file(path: String) -> Result<std::collections::HashMap<String, String>, String> {
    files::read_key_file(&path).await
}

pub async fn parse_lua_content(content: String, filename: String) -> Result<serde_json::Value, String> {
    let parsed = files::parse_lua_content(&content, &filename)?;
    serde_json::to_value(&parsed).map_err(|e| format!("Failed to serialize result: {}", e))
}

pub(crate) async fn dispatch(_app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "parse_lua_file" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, app_id: Option<u64>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(parse_lua_file(a.path, a.app_id).await)
        }
        "key_file_apps" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(key_file_apps(a.path).await)
        }
        "read_key_file" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(read_key_file(a.path).await)
        }
        "parse_lua_content" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { content: String, filename: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(parse_lua_content(a.content, a.filename).await)
        }
        _ => return None,
    })
}
