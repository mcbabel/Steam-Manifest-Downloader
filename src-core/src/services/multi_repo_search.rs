use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::services::depot_sources;
use crate::services::hubcap_api;
use crate::services::ryuu_api;

pub const HUBCAP_REPO_NAME: &str = "Hubcap (hubcapmanifest.com)";
pub const RYUU_REPO_NAME: &str = "Ryuu (generator.ryuu.lol)";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoResult {
    pub repo: String,
    pub date: Option<String>,
    pub sha: Option<String>,
    #[serde(rename = "type")]
    pub source_type: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub repos: Vec<RepoResult>,
    #[serde(rename = "sourceProbe", skip_serializing_if = "Option::is_none")]
    pub source_probe: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestWithKey {
    pub depot_id: String,
    pub manifest_id: String,
    pub filename: String,
    pub depot_key: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoManifests {
    pub manifests: Vec<ManifestWithKey>,
    pub has_key_vdf: bool,
    pub key_vdf_filename: Option<String>,
    pub lua_filename: Option<String>,
    pub files: Vec<String>,
    pub depot_keys: HashMap<String, String>,
}

pub async fn search_repos(
    client: &Client,
    sources: &[String],
    app_id: &str,
    ryuu_api_key: &str,
    hubcap_api_key: &str,
    app_data_dir: &Path,
) -> Result<SearchResult, String> {
    let mut found = Vec::new();

    if !ryuu_api_key.is_empty() {
        match ryuu_api::fetch_app_data(client, ryuu_api_key, app_data_dir, app_id).await {
            Ok(_) => {
                found.push(RepoResult {
                    repo: RYUU_REPO_NAME.to_string(),
                    date: None,
                    sha: None,
                    source_type: "ryuu".to_string(),
                    source: Some("Ryuu".to_string()),
                });
            }
            Err(e) => {
                eprintln!("[Search] Ryuu lookup failed: {}", e);
            }
        }
    }

    if !hubcap_api_key.is_empty() {
        match hubcap_api::fetch_app_data(client, hubcap_api_key, app_data_dir, app_id).await {
            Ok(_) => {
                found.push(RepoResult {
                    repo: HUBCAP_REPO_NAME.to_string(),
                    date: None,
                    sha: None,
                    source_type: "hubcap".to_string(),
                    source: Some("hubcap".to_string()),
                });
            }
            Err(e) => {
                eprintln!("[Search] Hubcap lookup failed: {}", e);
            }
        }
    }

    let mut source_probe: Option<String> = None;
    if !sources.is_empty() {
        match depot_sources::probe_app(client, sources, app_id).await {
            depot_sources::ProbeOutcome::Found => {
                source_probe = Some("found".to_string());
                found.push(RepoResult {
                    repo: "configured".to_string(),
                    date: None,
                    sha: None,
                    source_type: "remote".to_string(),
                    source: Some("remote".to_string()),
                });
            }
            depot_sources::ProbeOutcome::Missing => {
                source_probe = Some("missing".to_string());
            }
            depot_sources::ProbeOutcome::Inconclusive(class) => {
                source_probe = Some(format!("unreachable:{}", class));
                eprintln!(
                    "[Search] manifest sources unreachable ({}) — app {} may exist but could not be verified",
                    class, app_id
                );
            }
            depot_sources::ProbeOutcome::NoSources => {
                source_probe = Some("no_sources".to_string());
            }
        }
    }

    Ok(SearchResult { repos: found, source_probe })
}

pub async fn get_repo_manifests(
    client: &Client,
    sources: &[String],
    app_id: &str,
    repo: &str,
    _sha: &str,
    ryuu_api_key: &str,
    hubcap_api_key: &str,
    app_data_dir: &Path,
) -> Result<RepoManifests, String> {
    let app_data = if repo == RYUU_REPO_NAME {
        ryuu_api::fetch_app_data(client, ryuu_api_key, app_data_dir, app_id).await?
    } else if repo == HUBCAP_REPO_NAME {
        hubcap_api::fetch_app_data(client, hubcap_api_key, app_data_dir, app_id).await?
    } else {
        depot_sources::get_app_data(client, sources, app_id).await?
    };

    let lua_filename = Some(format!("{}.lua", app_id));

    let mut files = Vec::new();
    if let Some(ref lua_file) = lua_filename {
        files.push(lua_file.clone());
    }
    if app_data.has_key_vdf {
        files.push("key.vdf".to_string());
    }

    let manifests_with_keys: Vec<ManifestWithKey> = app_data
        .manifests
        .into_iter()
        .map(|m| {
            let filename = format!("{}_{}.manifest", m.depot_id, m.manifest_id);
            files.push(filename.clone());
            ManifestWithKey {
                depot_id: m.depot_id,
                manifest_id: m.manifest_id,
                filename,
                depot_key: m.depot_key,
                size_bytes: m.size_bytes,
            }
        })
        .collect();

    let key_vdf_filename = if app_data.has_key_vdf {
        Some("key.vdf".to_string())
    } else {
        None
    };

    Ok(RepoManifests {
        manifests: manifests_with_keys,
        has_key_vdf: app_data.has_key_vdf,
        key_vdf_filename,
        lua_filename,
        files,
        depot_keys: app_data.depot_keys,
    })
}
