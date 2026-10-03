use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use steam_vent::{Connection, ConnectionTrait};
use steam_vent_proto::steammessages_clientserver_appinfo::{
    cmsg_client_picsproduct_info_request, CMsgClientPICSProductInfoRequest,
    CMsgClientPICSProductInfoResponse,
};
use tokio::time::timeout;
use vdf_reader::entry::{Entry, Table};

use crate::services::steam_session::SteamSession;

const PICS_TIMEOUT: Duration = Duration::from_secs(20);
const PICS_BATCH_TIMEOUT: Duration = Duration::from_secs(40);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepotMetadata {
    #[serde(rename = "depotId")]
    pub depot_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "dlcAppId", skip_serializing_if = "Option::is_none")]
    pub dlc_app_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oslist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub osarch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(rename = "manifestGid", skip_serializing_if = "Option::is_none")]
    pub manifest_gid: Option<String>,
    pub role: DepotRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepotRole {
    Platform,
    SharedContent,
    Dlc,
    Language,
    Other,
}

pub async fn fetch_public_manifest_gid(
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
) -> Result<String, String> {
    timeout(PICS_TIMEOUT, async move {
        let conn = session.connection().await?;
        let vdf = pics_product_info_vdf(&conn, &[app_id]).await?;
        let app_vdf = vdf
            .get(&app_id)
            .ok_or_else(|| format!("PICS returned no info for app {}", app_id))?;
        let depot = depot_table(app_vdf, depot_id)?;
        let manifests = depot
            .get("manifests")
            .and_then(as_table)
            .ok_or_else(|| format!("Depot {} has no 'manifests' branch", depot_id))?;
        let public = manifests
            .get("public")
            .and_then(as_table)
            .ok_or_else(|| format!("Depot {} has no 'public' manifest branch", depot_id))?;
        let gid = public
            .get("gid")
            .and_then(as_str)
            .ok_or_else(|| format!("Depot {} public branch has no manifest gid", depot_id))?;
        Ok::<_, String>(gid.to_string())
    })
    .await
    .map_err(|_| "Steam PICS query timed out after 20s".to_string())?
}

pub async fn fetch_depots_with_names(
    session: Arc<SteamSession>,
    parent_app_id: u32,
) -> Result<Vec<DepotMetadata>, String> {
    timeout(PICS_BATCH_TIMEOUT, async move {
        let conn = session.connection().await?;
        let parent_vdf = pics_product_info_vdf(&conn, &[parent_app_id]).await?;
        let parent = parent_vdf
            .get(&parent_app_id)
            .ok_or_else(|| format!("PICS returned no info for app {}", parent_app_id))?;
        let depot_root = parent
            .get("depots")
            .and_then(as_table)
            .ok_or_else(|| format!("App {} has no 'depots' section", parent_app_id))?;

        let parent_game_name = parent
            .get("common")
            .and_then(as_table)
            .and_then(|c| c.get("name"))
            .and_then(as_str)
            .map(String::from);

        let mut depots: Vec<DepotMetadata> = Vec::new();
        let mut dlc_ids: HashSet<u32> = HashSet::new();
        for (key, value) in depot_root.iter() {
            if !key.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let Some(depot_tbl) = as_table(value) else {
                continue;
            };
            let direct_name = depot_tbl.get("name").and_then(as_str).map(String::from);
            let dlc_app_id = depot_tbl
                .get("dlcappid")
                .and_then(as_str)
                .and_then(|s| s.parse::<u32>().ok());
            if let Some(id) = dlc_app_id {
                if id != parent_app_id {
                    dlc_ids.insert(id);
                }
            }
            let config = depot_tbl.get("config").and_then(as_table);
            let oslist = config
                .and_then(|c| c.get("oslist"))
                .and_then(as_str)
                .map(String::from);
            let osarch = config
                .and_then(|c| c.get("osarch"))
                .and_then(as_str)
                .map(String::from);
            let language = config
                .and_then(|c| c.get("language"))
                .and_then(as_str)
                .map(String::from);
            let manifest_gid = depot_tbl
                .get("manifests")
                .and_then(as_table)
                .and_then(|m| m.get("public"))
                .and_then(as_table)
                .and_then(|p| p.get("gid"))
                .and_then(as_str)
                .map(String::from);

            let role = classify_role(dlc_app_id, &oslist, &language);

            depots.push(DepotMetadata {
                depot_id: key.clone(),
                name: direct_name,
                dlc_app_id,
                oslist,
                osarch,
                language,
                manifest_gid,
                role,
            });
        }

        if !dlc_ids.is_empty() {
            let dlc_ids_vec: Vec<u32> = dlc_ids.iter().copied().collect();
            let dlc_vdf = pics_product_info_vdf(&conn, &dlc_ids_vec).await?;
            for depot in depots.iter_mut() {
                if depot.name.is_some() {
                    continue;
                }
                if let Some(dlc_id) = depot.dlc_app_id {
                    if let Some(dlc_vdf_entry) = dlc_vdf.get(&dlc_id) {
                        if let Some(name) = dlc_vdf_entry
                            .get("common")
                            .and_then(as_table)
                            .and_then(|c| c.get("name"))
                            .and_then(as_str)
                        {
                            depot.name = Some(name.to_string());
                        }
                    }
                }
            }
        }

        for depot in depots.iter_mut() {
            if depot.name.is_some() {
                continue;
            }
            depot.name = synthesise_name(&depot, parent_game_name.as_deref());
        }

        Ok::<_, String>(depots)
    })
    .await
    .map_err(|_| "Steam PICS batch query timed out".to_string())?
}

fn classify_role(
    dlc_app_id: Option<u32>,
    oslist: &Option<String>,
    language: &Option<String>,
) -> DepotRole {
    if dlc_app_id.is_some() {
        return DepotRole::Dlc;
    }
    if language.as_deref().map(|l| !l.is_empty()).unwrap_or(false) {
        return DepotRole::Language;
    }
    if oslist.as_deref().map(|s| !s.is_empty()).unwrap_or(false) {
        return DepotRole::Platform;
    }
    DepotRole::SharedContent
}

fn synthesise_name(depot: &DepotMetadata, parent_game_name: Option<&str>) -> Option<String> {
    let game = parent_game_name?;
    let qualifier = match depot.role {
        DepotRole::Platform => platform_qualifier(depot.oslist.as_deref(), depot.osarch.as_deref()),
        DepotRole::Language => depot
            .language
            .as_deref()
            .map(|l| capitalise(l))
            .unwrap_or_else(|| "Language".to_string()),
        DepotRole::SharedContent => "Content".to_string(),
        DepotRole::Dlc => "DLC".to_string(),
        DepotRole::Other => "Depot".to_string(),
    };
    Some(format!("{} - {}", game, qualifier))
}

fn platform_qualifier(oslist: Option<&str>, osarch: Option<&str>) -> String {
    let os = match oslist.unwrap_or("").to_ascii_lowercase().as_str() {
        s if s.contains("windows") => "Windows",
        s if s.contains("macos") || s.contains("osx") || s.contains("mac") => "macOS",
        s if s.contains("linux") => "Linux",
        _ => "Platform",
    };
    match osarch {
        Some("64") => format!("{} 64-bit", os),
        Some("32") => format!("{} 32-bit", os),
        _ => os.to_string(),
    }
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

async fn pics_product_info_vdf(
    conn: &Connection,
    app_ids: &[u32],
) -> Result<HashMap<u32, Table>, String> {
    let apps = app_ids
        .iter()
        .map(|id| cmsg_client_picsproduct_info_request::AppInfo {
            appid: Some(*id),
            only_public_obsolete: Some(false),
            ..Default::default()
        })
        .collect();
    let request = CMsgClientPICSProductInfoRequest {
        apps,
        meta_data_only: Some(false),
        single_response: Some(true),
        ..Default::default()
    };
    let response: CMsgClientPICSProductInfoResponse = conn
        .job(request)
        .await
        .map_err(|e| format!("PICS request failed: {}", e))?;

    let mut out: HashMap<u32, Table> = HashMap::new();
    for app in response.apps {
        let Some(appid) = app.appid else {
            continue;
        };
        let Some(buffer) = app.buffer.as_deref() else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(buffer) else {
            continue;
        };
        let trimmed = text.trim().trim_matches('\0');
        let Ok(table) = vdf_reader::from_str::<Table>(trimmed) else {
            continue;
        };
        let app_table = table
            .get("appinfo")
            .and_then(as_table)
            .cloned()
            .unwrap_or(table);
        out.insert(appid, app_table);
    }
    Ok(out)
}

fn depot_table<'a>(app: &'a Table, depot_id: u32) -> Result<&'a Table, String> {
    let depots = app
        .get("depots")
        .and_then(as_table)
        .ok_or_else(|| format!("PICS info has no 'depots' section"))?;
    let key = depot_id.to_string();
    depots
        .get(&key)
        .and_then(as_table)
        .ok_or_else(|| format!("PICS info has no depot {}", depot_id))
}

fn as_table(entry: &Entry) -> Option<&Table> {
    match entry {
        Entry::Table(t) => Some(t),
        _ => None,
    }
}

fn as_str(entry: &Entry) -> Option<&str> {
    match entry {
        Entry::Value(s) => Some(&**s),
        _ => None,
    }
}
