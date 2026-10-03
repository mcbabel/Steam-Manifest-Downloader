use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const X64_URL: &str =
    "https://raw.githubusercontent.com/MCbabel/Steam-API-Check-Bypass/master/Release_dlls/SteamAPICheckBypass.dll";
const X32_URL: &str =
    "https://raw.githubusercontent.com/MCbabel/Steam-API-Check-Bypass/master/Release_dlls/SteamAPICheckBypass_x32.dll";
const HIJACK_NAMES: &[&str] = &["version.dll", "winmm.dll", "winhttp.dll"];
const CONFIG_NAME: &str = "SteamAPICheckBypass.json";

fn backup_name_for(hijack: &str) -> String {
    format!("{}.bypass.bak", hijack)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BypassApplyResult {
    pub target_dir: String,
    pub success: bool,
    pub installed_paths: Vec<String>,
    pub backup_paths: Vec<String>,
    pub config_path: Option<String>,
    pub error: Option<String>,
}

fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("steam_api_bypass_cache")
}

async fn download_to(client: &Client, url: &str, dest: &Path) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create cache dir: {}", e))?;
    }
    let resp = client
        .get(url)
        .header("User-Agent", "SteamManifestDownloader")
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("download failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("download HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await.map_err(|e| format!("read body: {}", e))?;
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| format!("write {}: {}", dest.display(), e))?;
    Ok(())
}

pub async fn ensure_cached(client: &Client, app_data_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    let root = cache_root(app_data_dir);
    let x64 = root.join("SteamAPICheckBypass.dll");
    let x32 = root.join("SteamAPICheckBypass_x32.dll");
    download_to(client, X64_URL, &x64).await?;
    download_to(client, X32_URL, &x32).await?;
    Ok((x64, x32))
}

pub fn is_installed_for_target(steam_api_target: &Path) -> bool {
    let Some(parent) = find_game_exe_dir(steam_api_target)
        .or_else(|| steam_api_target.parent().map(|p| p.to_path_buf()))
    else {
        return false;
    };
    parent.join(CONFIG_NAME).exists()
        || HIJACK_NAMES.iter().any(|n| parent.join(n).exists())
}

fn dir_has_exe(dir: &Path) -> bool {
    let Ok(rd) = std::fs::read_dir(dir) else { return false };
    for entry in rd.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            if name.to_lowercase().ends_with(".exe") {
                return true;
            }
        }
    }
    false
}

fn find_game_exe_dir(steam_api_target: &Path) -> Option<PathBuf> {
    let mut current = steam_api_target.parent()?.to_path_buf();
    for _ in 0..5 {
        if dir_has_exe(&current) {
            return Some(current);
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => return None,
        }
    }
    None
}

pub async fn apply_to_target(
    client: &Client,
    app_data_dir: &Path,
    steam_api_target: &Path,
    x64: bool,
) -> BypassApplyResult {
    let install_dir = match find_game_exe_dir(steam_api_target) {
        Some(d) => d,
        None => match steam_api_target.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                return BypassApplyResult {
                    target_dir: steam_api_target.to_string_lossy().to_string(),
                    success: false,
                    installed_paths: Vec::new(),
                    backup_paths: Vec::new(),
                    config_path: None,
                    error: Some("no game .exe found near target".into()),
                };
            }
        },
    };
    let parent = install_dir;

    let mut result = BypassApplyResult {
        target_dir: parent.to_string_lossy().to_string(),
        success: false,
        installed_paths: Vec::new(),
        backup_paths: Vec::new(),
        config_path: None,
        error: None,
    };

    if !parent.exists() {
        result.error = Some(format!("game folder not found: {}", parent.display()));
        return result;
    }

    let (x64_dll, x32_dll) = match ensure_cached(client, app_data_dir).await {
        Ok(pair) => pair,
        Err(e) => {
            result.error = Some(e);
            return result;
        }
    };
    let source = if x64 { &x64_dll } else { &x32_dll };

    for hijack in HIJACK_NAMES {
        let dest = parent.join(hijack);
        if dest.exists() {
            let backup = parent.join(backup_name_for(hijack));
            if !backup.exists() {
                if let Err(e) = std::fs::rename(&dest, &backup) {
                    result.error = Some(format!("backup existing {}: {}", hijack, e));
                    return result;
                }
                result.backup_paths.push(backup.to_string_lossy().to_string());
            }
        }
        if let Err(e) = std::fs::copy(source, &dest) {
            result.error = Some(format!("install {}: {}", hijack, e));
            return result;
        }
        result.installed_paths.push(dest.to_string_lossy().to_string());
    }

    let config_path = parent.join(CONFIG_NAME);
    if let Some(file_name) = steam_api_target.file_name().map(|n| n.to_string_lossy().to_string()) {
        let mut existing: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        let backup_name = format!("{}.steam.bak", file_name);
        existing.insert(file_name, serde_json::json!({
            "mode": "file_redirect",
            "to": backup_name,
            "hook_times_mode": "nth_time_only",
            "hook_time_n": [1, 2, 3],
            "bypass_loadlibrary": true,
        }));
        if let Ok(pretty) = serde_json::to_string_pretty(&serde_json::Value::Object(existing)) {
            match std::fs::write(&config_path, pretty) {
                Ok(()) => result.config_path = Some(config_path.to_string_lossy().to_string()),
                Err(e) => eprintln!("[steam_api_bypass] write config: {}", e),
            }
        }
    }

    result.success = true;
    result
}

pub fn revert_for_target(steam_api_target: &Path) -> Result<(), String> {
    let parent = find_game_exe_dir(steam_api_target)
        .or_else(|| steam_api_target.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| "target has no parent dir".to_string())?;

    for hijack in HIJACK_NAMES {
        let dest = parent.join(hijack);
        if dest.exists() {
            std::fs::remove_file(&dest)
                .map_err(|e| format!("remove {}: {}", hijack, e))?;
        }
        let backup = parent.join(backup_name_for(hijack));
        if backup.exists() {
            std::fs::rename(&backup, &dest)
                .map_err(|e| format!("restore backup {}: {}", hijack, e))?;
        }
    }

    let config = parent.join(CONFIG_NAME);
    if config.exists() {
        let _ = std::fs::remove_file(&config);
    }
    Ok(())
}
