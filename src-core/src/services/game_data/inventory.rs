use serde_json::{Map, Value};
use std::sync::Arc;
use std::time::Duration;

use steam_vent::ConnectionTrait;
use steam_vent_proto::steammessages_inventory_steamclient::CInventory_GetItemDefMeta_Request;

use super::{get_json, API};
use crate::services::steam_session::{call_with_retry, SteamSession};

const CM_TIMEOUT: Duration = Duration::from_secs(15);

pub struct Inventory {
    pub items: Value,
    pub defaults: Value,
    pub count: usize,
    pub icons: Vec<String>,
}

async fn digest_from_cm(session: &Arc<SteamSession>, app_id: u32) -> Option<String> {
    call_with_retry(session, 1, CM_TIMEOUT, "Steam inventory query timed out", |conn| async move {
        let req = CInventory_GetItemDefMeta_Request {
            appid: Some(app_id),
            ..Default::default()
        };
        conn.service_method(req)
            .await
            .map(|r| r.digest.unwrap_or_default())
            .map_err(|e| e.to_string())
    })
    .await
    .ok()
    .filter(|d| !d.trim().is_empty())
}

async fn digest_from_web(http: &reqwest::Client, app_id: u32, key: &str) -> Option<String> {
    let url = format!("{}/IInventoryService/GetItemDefMeta/v1/?key={}&appid={}", API, key, app_id);
    get_json(http, &url)
        .await
        .ok()?
        .pointer("/response/digest")
        .and_then(|d| d.as_str())
        .map(str::to_string)
        .filter(|d| !d.trim().is_empty())
}

pub fn parse_archive(raw: &[u8]) -> Option<Vec<Value>> {
    let end = raw.iter().rposition(|b| *b != 0).map(|i| i + 1).unwrap_or(0);
    serde_json::from_slice::<Value>(&raw[..end]).ok()?.as_array().cloned()
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::String(s) => s.clone(),
        Value::Null => "None".into(),
        other => other.to_string(),
    }
}

pub fn build(list: &[Value]) -> Inventory {
    let mut items = Map::new();
    let mut defaults = Map::new();
    let mut icons = Vec::new();
    for def in list {
        let Some(obj) = def.as_object() else { continue };
        let Some(id) = obj.get("itemdefid").map(as_text).filter(|s| !s.is_empty()) else {
            continue;
        };
        let fields: Map<String, Value> = obj
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(as_text(v))))
            .collect();
        for key in ["icon_url", "icon_url_large"] {
            if let Some(url) = obj.get(key).and_then(|v| v.as_str()).filter(|u| u.starts_with("http")) {
                icons.push(url.to_string());
            }
        }
        items.insert(id.clone(), Value::Object(fields));
        defaults.insert(id, serde_json::json!({ "quantity": 1 }));
    }
    icons.sort();
    icons.dedup();
    Inventory {
        count: items.len(),
        items: Value::Object(items),
        defaults: Value::Object(defaults),
        icons,
    }
}

pub async fn fetch(
    http: &reqwest::Client,
    session: &Arc<SteamSession>,
    app_id: u32,
    key: Option<&str>,
    notes: &mut Vec<String>,
) -> Option<Inventory> {
    let digest = match digest_from_cm(session, app_id).await {
        Some(d) => Some(d),
        None => match key {
            Some(k) => digest_from_web(http, app_id, k).await,
            None => None,
        },
    }?;
    let url = format!(
        "{}/IGameInventory/GetItemDefArchive/v0001?appid={}&digest={}",
        API,
        app_id,
        digest.trim()
    );
    let raw = match http.get(&url).send().await.and_then(|r| r.error_for_status()) {
        Ok(resp) => resp.bytes().await.ok(),
        Err(_) => None,
    };
    let Some(list) = raw.as_deref().and_then(parse_archive) else {
        notes.push("gameData.inventoryFailed".into());
        return None;
    };
    let inv = build(&list);
    (inv.count > 0).then_some(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_items_like_the_generator() {
        let raw = b"[{\"itemdefid\":2001,\"type\":\"item\",\"tradable\":false,\"name\":\"Hat\",\"icon_url\":\"https://x/hat.png\"},{\"type\":\"broken\"}]\0\0";
        let list = parse_archive(raw).unwrap();
        let inv = build(&list);
        assert_eq!(inv.count, 1);
        assert_eq!(inv.items["2001"]["itemdefid"], "2001");
        assert_eq!(inv.items["2001"]["tradable"], "false");
        assert_eq!(inv.defaults["2001"]["quantity"], 1);
        assert_eq!(inv.icons, vec!["https://x/hat.png".to_string()]);
        assert!(parse_archive(b"not json").is_none());
    }
}
