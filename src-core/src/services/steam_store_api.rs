use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    pub name: Option<String>,
    #[serde(rename = "headerImage")]
    pub header_image: Option<String>,
    #[serde(rename = "shortDescription")]
    pub short_description: Option<String>,
    #[serde(rename = "type")]
    pub app_type: Option<String>,
}

const MAX_CACHE_SIZE: usize = 500;

pub async fn get_game_info(
    client: &reqwest::Client,
    cache: &Arc<Mutex<HashMap<String, serde_json::Value>>>,
    app_id: &str,
) -> Result<Option<GameInfo>, String> {
    let id = app_id.to_string();

    {
        let mut cache_lock = cache.lock().await;
        if cache_lock.len() > MAX_CACHE_SIZE {
            cache_lock.clear();
        }
        if let Some(cached) = cache_lock.get(&id) {
            let info: Option<GameInfo> = serde_json::from_value(cached.clone()).ok();
            return Ok(info);
        }
    }

    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}",
        id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("[SteamAPI] Request failed for appId {}: {}", id, e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("[SteamAPI] Failed to parse JSON for appId {}: {}", id, e))?;

    let app_data = match data.get(&id) {
        Some(entry) => {
            let success = entry.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            if !success {
                return Ok(None);
            }
            match entry.get("data") {
                Some(d) => d,
                None => return Ok(None),
            }
        }
        None => return Ok(None),
    };

    let info = GameInfo {
        name: app_data
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        header_image: app_data
            .get("header_image")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        short_description: app_data
            .get("short_description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        app_type: app_data
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };

    {
        let mut cache_lock = cache.lock().await;
        if let Ok(val) = serde_json::to_value(&info) {
            cache_lock.insert(id, val);
        }
    }

    Ok(Some(info))
}

// Strips chars forbidden in Windows folder names and collapses whitespace.
pub fn sanitize_game_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .collect();

    let mut result = String::new();
    let mut prev_space = false;
    for c in cleaned.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result = result.trim().to_string();
    result.trim_end_matches(|c: char| c == '.' || c == ' ').to_string()
}
