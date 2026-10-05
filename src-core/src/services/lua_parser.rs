use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotInfo {
    pub depot_id: u64,
    pub depot_key: Option<String>,
    pub manifest_id: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuaParseResult {
    pub main_app_id: Option<u64>,
    pub depots: Vec<DepotInfo>,
    #[serde(default)]
    pub all_app_ids: Vec<u64>,
}

// Matches `addappid(id)` (main app) or `addappid(id, 0, "hexKey")` (depot with key).
fn add_app_id_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)addappid\((\d+)(?:\s*,\s*(\d+)\s*,\s*"([a-f0-9]+)")?\)"#)
            .expect("addappid pattern is a valid regex")
    })
}

// Matches `setManifestid(depotId, "manifestId")` and the Hubcap variant
// `setManifestid(depotId, "manifestId", sizeBytes)`.
fn set_manifest_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?i)setManifestid\((\d+)\s*,\s*"(\d+)"(?:\s*,\s*(\d+))?[^)]*\)"#)
            .expect("setManifestid pattern is a valid regex")
    })
}

/// Parse `.lua` file content, extracting `addappid()` and `setManifestid()` calls.
///
/// Returns `Err` when no app id and no depot entries were found. In that case the
/// file is either not in the expected format or its contents are corrupted.
pub fn parse_lua_file(content: &str) -> Result<LuaParseResult, String> {
    let mut result = LuaParseResult {
        main_app_id: None,
        depots: Vec::new(),
        all_app_ids: Vec::new(),
    };

    let mut depot_map: HashMap<u64, DepotInfo> = HashMap::new();
    let mut seen_app_ids: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();

    for cap in add_app_id_pattern().captures_iter(content) {
        let id: u64 = match cap[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let has_key = cap.get(3).is_some();

        if !has_key {
            if seen_app_ids.insert(id) {
                result.all_app_ids.push(id);
            }
            if result.main_app_id.is_none() {
                result.main_app_id = Some(id);
            }
        } else {
            let depot_key = cap[3].to_string();
            depot_map
                .entry(id)
                .and_modify(|d| d.depot_key = Some(depot_key.clone()))
                .or_insert(DepotInfo {
                    depot_id: id,
                    depot_key: Some(depot_key),
                    manifest_id: None,
                    size_bytes: None,
                });
        }
    }

    for cap in set_manifest_pattern().captures_iter(content) {
        let depot_id: u64 = match cap[1].parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        let manifest_id = cap[2].to_string();
        let size_bytes = cap.get(3).and_then(|m| m.as_str().parse::<u64>().ok());

        depot_map
            .entry(depot_id)
            .and_modify(|d| {
                d.manifest_id = Some(manifest_id.clone());
                if size_bytes.is_some() {
                    d.size_bytes = size_bytes;
                }
            })
            .or_insert(DepotInfo {
                depot_id,
                depot_key: None,
                manifest_id: Some(manifest_id),
                size_bytes,
            });
    }

    result.depots = depot_map.into_values().collect();

    // Fallback: if the script has no explicit main app id, use the smallest depot id.
    if result.main_app_id.is_none() && !result.depots.is_empty() {
        result.main_app_id = result.depots.iter().map(|d| d.depot_id).min();
    }

    if result.main_app_id.is_none() && result.depots.is_empty() {
        return Err(
            "The file does not contain any recognised entries. Expected at least one addappid(...) or setManifestid(...) call."
                .to_string(),
        );
    }

    Ok(result)
}
