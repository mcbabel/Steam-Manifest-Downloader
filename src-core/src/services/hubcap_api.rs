use reqwest::Client;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::services::depot_sources::{AppArchiveData, ManifestInfo};
use crate::services::lua_parser;

const HUBCAP_BASE: &str = "https://hubcapmanifest.com/api/v1/manifest";

pub fn cache_dir(app_data_dir: &Path, app_id: &str) -> PathBuf {
    app_data_dir.join("hubcap_cache").join(app_id)
}

/// Download the zip for `app_id` from hubcap, extract `.lua` and `.manifest`
/// files into the per-app cache dir, and return the resulting AppArchiveData
/// (depot ids, manifest ids, depot keys parsed from the lua file).
pub async fn fetch_app_data(
    client: &Client,
    api_key: &str,
    app_data_dir: &Path,
    app_id: &str,
) -> Result<AppArchiveData, String> {
    if api_key.is_empty() {
        return Err("Hubcap API key is not set".into());
    }

    let url = format!("{}/{}", HUBCAP_BASE, app_id);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("User-Agent", "SteamManifestDownloader")
        .send()
        .await
        .map_err(|e| format!("Hubcap request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Hubcap returned HTTP {}", resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read hubcap response: {}", e))?;

    let out_dir = cache_dir(app_data_dir, app_id);

    // Wipe any stale cache so we don't mix manifests from different downloads.
    let _ = tokio::fs::remove_dir_all(&out_dir).await;
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("Failed to create hubcap cache dir: {}", e))?;

    // Extract everything synchronously into memory first so the non-Send
    // ZipFile reader is fully dropped before any await point.
    let extracted: Vec<(String, Vec<u8>)> = {
        let reader = Cursor::new(bytes.to_vec());
        let mut zip = zip::ZipArchive::new(reader)
            .map_err(|e| format!("Hubcap response is not a valid zip: {}", e))?;

        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..zip.len() {
            let mut file = zip
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            if file.is_dir() {
                continue;
            }
            let raw_name = file
                .enclosed_name()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
            let Some(name) = raw_name else { continue };
            let lower = name.to_lowercase();
            if !lower.ends_with(".lua") && !lower.ends_with(".manifest") {
                continue;
            }
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("Failed to extract '{}': {}", name, e))?;
            out.push((name, buf));
        }
        out
    };

    let mut lua_content: Option<String> = None;
    for (name, buf) in &extracted {
        let dest = out_dir.join(name);
        tokio::fs::write(&dest, buf)
            .await
            .map_err(|e| format!("Failed to write '{}': {}", name, e))?;
        if name.to_lowercase().ends_with(".lua") && lua_content.is_none() {
            lua_content = Some(String::from_utf8_lossy(buf).to_string());
        }
    }

    let lua_content = lua_content
        .ok_or_else(|| "Hubcap zip did not contain a .lua file".to_string())?;

    let parsed = lua_parser::parse_lua_file(&lua_content)
        .map_err(|e| format!("Failed to parse hubcap .lua: {}", e))?;

    let mut depot_keys: HashMap<String, String> = HashMap::new();
    for depot in &parsed.depots {
        if let Some(ref key) = depot.depot_key {
            depot_keys.insert(depot.depot_id.to_string(), key.clone());
        }
    }

    let manifests: Vec<ManifestInfo> = parsed
        .depots
        .iter()
        .filter_map(|d| {
            d.manifest_id.as_ref().map(|mid| ManifestInfo {
                depot_id: d.depot_id.to_string(),
                manifest_id: mid.clone(),
                depot_key: depot_keys.get(&d.depot_id.to_string()).cloned(),
                size_bytes: None,
            })
        })
        .collect();

    Ok(AppArchiveData {
        app_id: app_id.to_string(),
        manifests,
        depot_keys,
        has_key_vdf: false,
    })
}

/// Copy a cached manifest file from the hubcap cache into `output_dir`.
/// Returns the destination path on success.
pub async fn copy_cached_manifest(
    app_data_dir: &Path,
    app_id: &str,
    depot_id: &str,
    manifest_id: &str,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = format!("{}_{}.manifest", depot_id, manifest_id);
    let src = cache_dir(app_data_dir, app_id).join(&filename);
    if !src.exists() {
        return Err(format!("Cached hubcap manifest not found: {}", filename));
    }
    tokio::fs::create_dir_all(output_dir)
        .await
        .map_err(|e| format!("Failed to create dir: {}", e))?;
    let dest = output_dir.join(&filename);
    tokio::fs::copy(&src, &dest)
        .await
        .map_err(|e| format!("Failed to copy cached manifest: {}", e))?;
    Ok(dest)
}
