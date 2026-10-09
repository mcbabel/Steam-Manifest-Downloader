use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use steam_vent::{Connection, ConnectionTrait};
use steam_vent_proto::steammessages_clientserver_appinfo::{
    cmsg_client_picsproduct_info_request, CMsgClientPICSProductInfoRequest,
    CMsgClientPICSProductInfoResponse,
};
use vdf_reader::entry::{Entry, Table};

use crate::services::steam_session::{call_with_retry, SteamSession};

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
    #[serde(rename = "lowViolence", default, skip_serializing_if = "std::ops::Not::not")]
    pub low_violence: bool,
    #[serde(rename = "optionalDlc", default, skip_serializing_if = "std::ops::Not::not")]
    pub optional_dlc: bool,
    #[serde(rename = "sharedInstall", default, skip_serializing_if = "std::ops::Not::not")]
    pub shared_install: bool,
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
    call_with_retry(&session, 2, PICS_TIMEOUT, "Steam PICS query timed out after 20s", |conn| async move {
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
}

fn public_gids(app: &Table) -> HashMap<String, String> {
    let Some(depots) = app.get("depots").and_then(as_table) else {
        return HashMap::new();
    };
    depots
        .iter()
        .filter(|(key, _)| key.chars().all(|c| c.is_ascii_digit()))
        .filter_map(|(key, value)| {
            let public = as_table(value)?.get("manifests").and_then(as_table)?.get("public")?;
            let gid = as_table(public)
                .and_then(|p| p.get("gid"))
                .and_then(as_str)
                .or_else(|| as_str(public))?;
            Some((key.clone(), gid.to_string()))
        })
        .collect()
}

pub async fn fetch_public_manifests(
    session: Arc<SteamSession>,
    app_ids: &[u32],
) -> Result<HashMap<u32, HashMap<String, String>>, String> {
    let ids = app_ids;
    call_with_retry(&session, 2, PICS_BATCH_TIMEOUT, "Steam PICS batch query timed out", |conn| async move {
        let vdf = pics_product_info_vdf(&conn, ids).await?;
        Ok::<_, String>(vdf.iter().map(|(id, app)| (*id, public_gids(app))).collect())
    })
    .await
}

pub async fn fetch_depots_with_names(
    session: Arc<SteamSession>,
    parent_app_id: u32,
) -> Result<Vec<DepotMetadata>, String> {
    call_with_retry(&session, 2, PICS_BATCH_TIMEOUT, "Steam PICS batch query timed out", |conn| async move {
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
            let flag = |tbl: Option<&Table>, name: &str| {
                tbl.and_then(|t| t.get(name)).and_then(as_str) == Some("1")
            };
            let low_violence = flag(config, "lowviolence");
            let optional_dlc = flag(config, "optionaldlc");
            let shared_install =
                flag(Some(depot_tbl), "sharedinstall") || depot_tbl.get("depotfromapp").is_some();
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
                low_violence,
                optional_dlc,
                shared_install,
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
}

pub fn parse_dlc_list(app: &Table) -> Vec<u32> {
    app.get("extended")
        .and_then(as_table)
        .and_then(|e| e.get("listofdlc"))
        .and_then(as_str)
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse::<u32>().ok())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default)]
pub struct DlcInfo {
    pub listed: Vec<u32>,
    pub names: HashMap<u32, String>,
}

pub async fn fetch_dlc_info(
    session: Arc<SteamSession>,
    app_id: u32,
    extra_ids: &[u32],
) -> Result<DlcInfo, String> {
    let extra = extra_ids;
    call_with_retry(&session, 2, PICS_BATCH_TIMEOUT, "Steam PICS batch query timed out", |conn| async move {
        let parent = pics_product_info_vdf(&conn, &[app_id]).await?;
        let listed = parent.get(&app_id).map(parse_dlc_list).unwrap_or_default();
        let mut ids: Vec<u32> = listed.iter().chain(extra.iter()).copied().collect();
        ids.sort_unstable();
        ids.dedup();
        let mut names = HashMap::new();
        for chunk in ids.chunks(100) {
            let info = pics_product_info_vdf(&conn, chunk).await?;
            for (id, app) in info {
                if let Some(name) = app
                    .get("common")
                    .and_then(as_table)
                    .and_then(|c| c.get("name"))
                    .and_then(as_str)
                {
                    names.insert(id, name.to_string());
                }
            }
        }
        Ok::<_, String>(DlcInfo { listed, names })
    })
    .await
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

pub async fn fetch_app_info(session: Arc<SteamSession>, app_id: u32) -> Result<Table, String> {
    call_with_retry(&session, 2, PICS_TIMEOUT, "Steam PICS query timed out after 20s", |conn| async move {
        pics_product_info_vdf(&conn, &[app_id])
            .await?
            .remove(&app_id)
            .ok_or_else(|| format!("PICS returned no info for app {}", app_id))
    })
    .await
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub description: String,
    pub protected: bool,
    pub build_id: u64,
    pub time_updated: u64,
}

pub fn branches(app: &Table) -> Vec<Branch> {
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let mut out: Vec<Branch> = app
        .get("depots")
        .and_then(as_table)
        .and_then(|d| d.get("branches"))
        .and_then(as_table)
        .map(|list| {
            list.iter()
                .filter_map(|(name, entry)| {
                    let b = as_table(entry)?;
                    let text = |key: &str| b.get(key).and_then(as_str).map(|s| s.trim().to_string());
                    let num = |key: &str| text(key).and_then(|v| v.parse::<u64>().ok());
                    Some(Branch {
                        name: name.to_string(),
                        description: text("description").unwrap_or_default(),
                        protected: text("pwdrequired").is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
                        build_id: num("buildid").unwrap_or(0),
                        time_updated: num("timeupdated").unwrap_or(now),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| (a.name != "public", &a.name).cmp(&(b.name != "public", &b.name)));
    out
}

pub fn depot_ids(app: &Table) -> Vec<String> {
    let mut ids: Vec<u64> = app
        .get("depots")
        .and_then(as_table)
        .map(|d| d.keys().filter_map(|k| k.parse::<u64>().ok()).collect())
        .unwrap_or_default();
    ids.sort_unstable();
    ids.into_iter().map(|id| id.to_string()).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControllerConfig {
    pub file_id: u64,
    pub controller_type: String,
    pub enabled_branches: String,
}

pub fn controller_configs(app: &Table) -> Vec<ControllerConfig> {
    const RANK: [&str; 3] = ["controller_xboxone", "controller_xbox360", "controller_steamcontroller_gordon"];
    let mut out: Vec<ControllerConfig> = app
        .get("config")
        .and_then(as_table)
        .and_then(|c| c.get("steamcontrollerconfigdetails"))
        .and_then(as_table)
        .map(|list| {
            list.iter()
                .filter_map(|(id, entry)| {
                    let d = as_table(entry)?;
                    let text = |key: &str| d.get(key).and_then(as_str).unwrap_or_default().trim().to_string();
                    Some(ControllerConfig {
                        file_id: id.trim().parse().ok()?,
                        controller_type: text("controller_type"),
                        enabled_branches: text("enabled_branches"),
                    })
                })
                .filter(|c| {
                    RANK.contains(&c.controller_type.as_str())
                        && c.enabled_branches
                            .split(',')
                            .any(|b| matches!(b.trim(), "default" | "public"))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|c| (RANK.iter().position(|r| *r == c.controller_type), c.file_id));
    out
}

pub fn launch_exe(app: &Table) -> Option<String> {
    const UNWANTED: [&str; 6] = ["launch", "start", "play", "try", "demo", "_vr"];
    let launch = app.get("config").and_then(as_table)?.get("launch").and_then(as_table)?;
    let mut entries: Vec<(u64, String)> = launch
        .iter()
        .filter_map(|(k, v)| {
            let exe = as_table(v)?.get("executable").and_then(as_str)?;
            Some((k.parse().unwrap_or(u64::MAX), exe.replace('\\', "/")))
        })
        .collect();
    entries.sort();
    let exes: Vec<String> = entries
        .into_iter()
        .filter(|(_, e)| e.to_ascii_lowercase().ends_with(".exe"))
        .filter_map(|(_, e)| e.rsplit('/').next().map(str::to_string))
        .collect();
    exes.iter()
        .find(|e| {
            let lower = e.to_ascii_lowercase();
            UNWANTED.iter().all(|u| !lower.contains(u))
        })
        .or_else(|| exes.first())
        .cloned()
}

pub fn common_value(app: &Table, key: &str) -> Option<String> {
    app.get("common")
        .and_then(as_table)?
        .get(key)
        .and_then(as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SaveFile {
    pub root: String,
    pub path: String,
    pub platforms: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RootOverride {
    pub root: String,
    pub use_instead: String,
    pub os: String,
    pub add_path: String,
    pub transforms: Vec<(String, String)>,
}

fn sorted_tables(t: &Table) -> Vec<&Table> {
    let mut list: Vec<(u64, &Table)> = t
        .iter()
        .filter_map(|(k, v)| Some((k.parse().unwrap_or(u64::MAX), as_table(v)?)))
        .collect();
    list.sort_by_key(|(k, _)| *k);
    list.into_iter().map(|(_, t)| t).collect()
}

pub fn cloud_saves(app: &Table) -> (Vec<SaveFile>, Vec<RootOverride>) {
    let Some(ufs) = app.get("ufs").and_then(as_table) else {
        return (Vec::new(), Vec::new());
    };
    let text = |t: &Table, key: &str| t.get(key).and_then(as_str).unwrap_or_default().to_string();
    let files = ufs
        .get("savefiles")
        .and_then(as_table)
        .map(sorted_tables)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            let root = text(f, "root");
            if root.is_empty() {
                return None;
            }
            let platforms = f
                .get("platforms")
                .and_then(as_table)
                .map(|p| p.values().filter_map(as_str).map(|s| s.to_string()).collect())
                .unwrap_or_default();
            Some(SaveFile { root, path: text(f, "path"), platforms })
        })
        .collect();
    let overrides = ufs
        .get("rootoverrides")
        .and_then(as_table)
        .map(sorted_tables)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|o| {
            let root = text(o, "root");
            if root.is_empty() {
                return None;
            }
            let transforms = o
                .get("pathtransforms")
                .and_then(as_table)
                .map(sorted_tables)
                .unwrap_or_default()
                .into_iter()
                .map(|t| (text(t, "find"), text(t, "replace")))
                .collect();
            Some(RootOverride {
                root,
                use_instead: text(o, "useinstead"),
                os: text(o, "os"),
                add_path: text(o, "addpath"),
                transforms,
            })
        })
        .collect();
    (files, overrides)
}

pub fn supported_languages(app: &Table) -> Vec<String> {
    let mut out: Vec<String> = app
        .get("common")
        .and_then(as_table)
        .and_then(|c| c.get("supported_languages"))
        .and_then(as_table)
        .map(|langs| {
            langs
                .iter()
                .filter(|(_, v)| {
                    as_table(v)
                        .and_then(|t| t.get("supported"))
                        .and_then(as_str)
                        .is_some_and(|s| s == "true" || s == "1")
                })
                .map(|(k, _)| k.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default();
    if out.is_empty() {
        if let Some(depots) = app.get("depots").and_then(as_table) {
            for depot in depots.values() {
                let lang = as_table(depot)
                    .and_then(|d| d.get("config"))
                    .and_then(as_table)
                    .and_then(|c| c.get("language"))
                    .and_then(as_str)
                    .map(|l| l.trim().to_ascii_lowercase())
                    .filter(|l| !l.is_empty());
                if let Some(l) = lang {
                    if !out.contains(&l) {
                        out.push(l);
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    if let Some(pos) = out.iter().position(|l| l == "english") {
        let en = out.remove(pos);
        out.insert(0, en);
    }
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_dlc_list() {
        let app: Table = vdf_reader::from_str(
            "\"common\" { \"name\" \"Game\" } \"extended\" { \"listofdlc\" \"20, 10,x\" }",
        )
        .unwrap();
        assert_eq!(parse_dlc_list(&app), vec![20, 10]);
        let empty: Table = vdf_reader::from_str("\"common\" { \"name\" \"Game\" }").unwrap();
        assert!(parse_dlc_list(&empty).is_empty());
    }

    #[test]
    fn reads_supported_languages_with_english_first() {
        let app: Table = vdf_reader::from_str(
            "\"common\" { \"supported_languages\" { \"german\" { \"supported\" \"true\" } \"french\" { \"supported\" \"false\" } \"english\" { \"supported\" \"true\" \"full_audio\" \"true\" } } }",
        )
        .unwrap();
        assert_eq!(supported_languages(&app), vec!["english", "german"]);
        let from_depots: Table = vdf_reader::from_str(
            "\"depots\" { \"11\" { \"config\" { \"language\" \"polish\" } } \"12\" { \"config\" { \"oslist\" \"windows\" } } }",
        )
        .unwrap();
        assert_eq!(supported_languages(&from_depots), vec!["polish"]);
    }
}
