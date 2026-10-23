use std::collections::BTreeMap;

use serde::Serialize;

use crate::services::lua_parser::{DepotInfo, LuaParseResult};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyDepot {
    pub depot_id: u64,
    pub manifest_id: Option<String>,
    pub size_bytes: Option<u64>,
    pub key: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyApp {
    pub app_id: u64,
    pub name: Option<String>,
    pub depots: Vec<KeyDepot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeyFile {
    pub apps: Vec<KeyApp>,
    pub keys: BTreeMap<u64, String>,
}

pub const EXTENSIONS: &[&str] = &["json", "csv", "vdf", "txt"];

fn clean_key(raw: &str) -> Option<String> {
    let k = raw.trim().trim_matches('"').to_ascii_lowercase();
    (k.len() == 64 && k.bytes().all(|b| b.is_ascii_hexdigit())).then_some(k)
}

fn number(v: &serde_json::Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str()?.trim().parse().ok())
}

fn text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn field<'a>(obj: &'a serde_json::Value, names: &[&str]) -> Option<&'a serde_json::Value> {
    let map = obj.as_object()?;
    names.iter().find_map(|n| {
        map.iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(n))
            .map(|(_, v)| v)
    })
}

fn json_depot(d: &serde_json::Value) -> Option<KeyDepot> {
    Some(KeyDepot {
        depot_id: number(field(d, &["depotid", "depot_id", "depotId", "id"])?)?,
        manifest_id: field(d, &["manifest", "manifest_id", "manifestId", "gid"]).and_then(text),
        size_bytes: field(d, &["size", "size_bytes", "sizeBytes"]).and_then(number),
        key: clean_key(field(d, &["key", "decryptionkey", "depot_key", "depotKey"])?.as_str()?)?,
    })
}

fn parse_json(text_in: &str) -> Result<KeyFile, String> {
    let root: serde_json::Value =
        serde_json::from_str(text_in).map_err(|e| format!("Invalid key file (JSON): {}", e))?;
    let mut out = KeyFile::default();
    if let Some(apps) = field(&root, &["apps"]).and_then(|a| a.as_array()) {
        for app in apps {
            let Some(app_id) = field(app, &["appid", "app_id", "appId"]).and_then(number) else {
                continue;
            };
            let depots: Vec<KeyDepot> = field(app, &["depots"])
                .and_then(|d| d.as_array())
                .map(|list| list.iter().filter_map(json_depot).collect())
                .unwrap_or_default();
            out.apps.push(KeyApp {
                app_id,
                name: field(app, &["name", "app_name"]).and_then(text),
                depots,
            });
        }
    } else if let Some(list) = root.as_array() {
        for d in list.iter().filter_map(json_depot) {
            out.keys.insert(d.depot_id, d.key);
        }
    } else if let Some(map) = root.as_object() {
        for (k, v) in map {
            if let (Ok(id), Some(key)) = (k.trim().parse::<u64>(), v.as_str().and_then(clean_key)) {
                out.keys.insert(id, key);
            }
        }
    }
    for app in &out.apps {
        for d in &app.depots {
            out.keys.insert(d.depot_id, d.key.clone());
        }
    }
    Ok(out)
}

fn split_csv(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' | ';' if !quoted => cells.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    cells.push(cur);
    cells.into_iter().map(|c| c.trim().to_string()).collect()
}

fn parse_csv(text_in: &str) -> Result<KeyFile, String> {
    let mut lines = text_in.lines().filter(|l| !l.trim().is_empty());
    let Some(first) = lines.next() else {
        return Ok(KeyFile::default());
    };
    let head: Vec<String> = split_csv(first).into_iter().map(|h| h.to_ascii_lowercase()).collect();
    let col = |names: &[&str]| head.iter().position(|h| names.contains(&h.as_str()));
    let depot_col = col(&["depotid", "depot_id", "depot"]);
    let key_col = col(&["key", "decryptionkey", "depot_key"]);
    let mut out = KeyFile::default();
    let (Some(depot_col), Some(key_col)) = (depot_col, key_col) else {
        return parse_txt(text_in);
    };
    let app_col = col(&["appid", "app_id", "app"]);
    let name_col = col(&["app_name", "name"]);
    let manifest_col = col(&["manifest", "manifest_id", "gid"]);
    let size_col = col(&["size", "size_bytes"]);
    let mut apps: Vec<KeyApp> = Vec::new();
    for line in lines {
        let cells = split_csv(line);
        let cell = |i: Option<usize>| i.and_then(|i| cells.get(i)).map(String::as_str).filter(|s| !s.is_empty());
        let (Some(depot_id), Some(key)) = (
            cell(Some(depot_col)).and_then(|s| s.parse::<u64>().ok()),
            cell(Some(key_col)).and_then(clean_key),
        ) else {
            continue;
        };
        out.keys.insert(depot_id, key.clone());
        let Some(app_id) = cell(app_col).and_then(|s| s.parse::<u64>().ok()) else {
            continue;
        };
        let depot = KeyDepot {
            depot_id,
            manifest_id: cell(manifest_col).map(str::to_string),
            size_bytes: cell(size_col).and_then(|s| s.parse().ok()),
            key,
        };
        match apps.iter_mut().find(|a| a.app_id == app_id) {
            Some(app) => app.depots.push(depot),
            None => apps.push(KeyApp {
                app_id,
                name: cell(name_col).map(str::to_string),
                depots: vec![depot],
            }),
        }
    }
    out.apps = apps;
    Ok(out)
}

fn parse_txt(text_in: &str) -> Result<KeyFile, String> {
    let mut out = KeyFile::default();
    for line in text_in.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let mut parts = line.split(|c: char| c == ';' || c == ',' || c == ':' || c == '=' || c.is_whitespace()).filter(|p| !p.is_empty());
        if let (Some(id), Some(key)) = (parts.next(), parts.next()) {
            if let (Ok(id), Some(key)) = (id.trim_matches('"').parse::<u64>(), clean_key(key)) {
                out.keys.insert(id, key);
            }
        }
    }
    Ok(out)
}

fn parse_vdf(text_in: &str) -> Result<KeyFile, String> {
    let keys = crate::services::steam_local_keys::parse_keys(text_in)
        .into_iter()
        .filter_map(|(id, key)| Some((id.parse().ok()?, key)))
        .collect();
    Ok(KeyFile { apps: Vec::new(), keys })
}

pub fn parse(text_in: &str, ext: &str) -> Result<KeyFile, String> {
    let file = match ext.to_ascii_lowercase().as_str() {
        "json" => parse_json(text_in)?,
        "csv" => parse_csv(text_in)?,
        "vdf" => parse_vdf(text_in)?,
        "txt" | "keys" => parse_txt(text_in)?,
        other => return Err(format!("Unsupported key file: .{}", other)),
    };
    if file.keys.is_empty() {
        return Err("The key file contains no depot keys".into());
    }
    Ok(file)
}

pub async fn read(path: &std::path::Path) -> Result<KeyFile, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("Failed to read file: {}", e))?;
    parse(&content, ext)
}

pub fn playable_apps(file: &KeyFile) -> Vec<&KeyApp> {
    file.apps.iter().filter(|a| !a.depots.is_empty()).collect()
}

pub fn to_parse_result(file: &KeyFile, app_id: Option<u64>) -> Result<LuaParseResult, String> {
    let apps = playable_apps(file);
    if apps.is_empty() {
        return Err("This key file has only depot keys, no game. Load the game by search, a .lua file or your account and add the keys in step 2".into());
    }
    let app = match app_id {
        Some(id) => apps
            .iter()
            .find(|a| a.app_id == id)
            .ok_or_else(|| format!("App {} is not in the key file", id))?,
        None if apps.len() == 1 => &apps[0],
        None => return Err(format!("The key file contains {} games. Choose one", apps.len())),
    };
    Ok(LuaParseResult {
        main_app_id: Some(app.app_id),
        depots: app
            .depots
            .iter()
            .map(|d| DepotInfo {
                depot_id: d.depot_id,
                depot_key: Some(d.key.clone()),
                manifest_id: d.manifest_id.clone(),
                size_bytes: d.size_bytes,
            })
            .collect(),
        all_app_ids: vec![app.app_id],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    #[test]
    fn reads_all_four_formats() {
        let vdf = format!("\"depots\"\n{{\n\t\"101\"\n\t{{\n\t\t\"DecryptionKey\"\t\t\"{KEY}\"\n\t}}\n}}\n");
        let txt = format!("101;{KEY}\n");
        let json = format!(r#"{{"apps":[{{"appid":100,"name":"Game","depots":[{{"depotid":101,"manifest":"5000","size":42,"key":"{KEY}"}}]}}]}}"#);
        let csv = format!("appid,app_name,depotid,manifest,size,key\n100,Game,101,5000,42,{KEY}\n");
        for (text, ext) in [(vdf.as_str(), "vdf"), (txt.as_str(), "txt"), (json.as_str(), "json"), (csv.as_str(), "csv")] {
            let file = parse(text, ext).unwrap();
            assert_eq!(file.keys.get(&101).map(String::as_str), Some(KEY), "{}", ext);
        }
        for (text, ext) in [(json.as_str(), "json"), (csv.as_str(), "csv")] {
            let parsed = to_parse_result(&parse(text, ext).unwrap(), None).unwrap();
            assert_eq!(parsed.main_app_id, Some(100));
            assert_eq!(parsed.depots[0].manifest_id.as_deref(), Some("5000"));
            assert_eq!(parsed.depots[0].size_bytes, Some(42));
        }
        assert!(to_parse_result(&parse(&txt, "txt").unwrap(), None).is_err());
    }

    #[test]
    fn several_games_need_a_choice() {
        let json = format!(r#"{{"apps":[{{"appid":1,"depots":[{{"depotid":11,"key":"{KEY}"}}]}},{{"appid":2,"depots":[{{"depotid":21,"key":"{KEY}"}}]}}]}}"#);
        let file = parse(&json, "json").unwrap();
        assert!(to_parse_result(&file, None).is_err());
        assert_eq!(to_parse_result(&file, Some(2)).unwrap().depots[0].depot_id, 21);
    }

    #[test]
    fn skips_bad_keys() {
        assert!(parse("101;nothex\n", "txt").is_err());
    }
}
