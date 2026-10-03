use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryConsent {
    Pending,
    Accepted,
    Declined,
}

impl Default for TelemetryConsent {
    fn default() -> Self {
        TelemetryConsent::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_download_location")]
    pub download_location: String,
    #[serde(default = "default_dd_extra_args")]
    pub dd_extra_args: Vec<String>,
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_notification_sound")]
    pub notification_sound: bool,
    #[serde(default)]
    pub download_speed_limit: String,
    #[serde(default = "default_chunk_concurrency")]
    pub native_chunk_concurrency: u32,
    #[serde(default)]
    pub proxy: String,
    #[serde(default)]
    pub telemetry_consent: TelemetryConsent,
    #[serde(default)]
    pub installation_id: String,
    #[serde(default, alias = "manifest_sources")]
    pub depot_sources: Vec<String>,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub auto_seeded: bool,
    #[serde(default)]
    pub pristine_default_sources: Vec<String>,
    #[serde(default)]
    pub hubcap_api_key: String,
    #[serde(default = "default_use_native_downloader")]
    pub use_native_downloader: bool,
    #[serde(default = "default_cancel_keep_files")]
    pub cancel_keep_files: bool,
    #[serde(default)]
    pub ryuu_api_key: String,
}

fn default_use_native_downloader() -> bool {
    true
}

fn default_cancel_keep_files() -> bool {
    false
}

fn default_download_location() -> String {
    if let Some(home) = dirs_next_home() {
        let docs = PathBuf::from(&home).join("Documents").join("SteamDownloads");
        return docs.to_string_lossy().to_string();
    }
    "./downloads".to_string()
}

fn dirs_next_home() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}

fn default_dd_extra_args() -> Vec<String> {
    vec![
        "-max-downloads".to_string(),
        "8".to_string(),
        "-verify-all".to_string(),
    ]
}

fn default_auto_update() -> bool {
    true
}

fn default_max_retries() -> u32 {
    3
}

fn default_chunk_concurrency() -> u32 {
    crate::services::steam_downloader::DEFAULT_CHUNK_CONCURRENCY
}

fn default_notification_sound() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_location: default_download_location(),
            dd_extra_args: default_dd_extra_args(),
            auto_update: default_auto_update(),
            max_retries: default_max_retries(),
            native_chunk_concurrency: default_chunk_concurrency(),
            notification_sound: default_notification_sound(),
            download_speed_limit: String::new(),
            proxy: String::new(),
            telemetry_consent: TelemetryConsent::Pending,
            installation_id: String::new(),
            depot_sources: Vec::new(),
            language: String::new(),
            auto_seeded: false,
            pristine_default_sources: Vec::new(),
            hubcap_api_key: String::new(),
            use_native_downloader: default_use_native_downloader(),
            cancel_keep_files: default_cancel_keep_files(),
            ryuu_api_key: String::new(),
        }
    }
}

pub fn default_depot_sources() -> Vec<String> {
    #[cfg(feature = "default-sources")]
    {
        vec![
            "https://archive.org/download/manifest-hub-repo/NEW-depot-keys.zip/".to_string(),
            "https://archive.org/download/manifest-hub-repo/branches.zip/".to_string(),
        ]
    }
    #[cfg(not(feature = "default-sources"))]
    {
        Vec::new()
    }
}

pub async fn seed_defaults_if_needed(app_data_dir: &Path) -> Settings {
    let mut settings = load_settings(app_data_dir).await;
    if !settings.auto_seeded {
        let defaults = default_depot_sources();
        if !defaults.is_empty() && settings.depot_sources.is_empty() {
            settings.depot_sources = defaults.clone();
            settings.pristine_default_sources = defaults;
        }
        settings.auto_seeded = true;
        let _ = save_settings(app_data_dir, &settings).await;
    }
    settings
}

fn settings_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("settings.json")
}

pub async fn load_settings(app_data_dir: &Path) -> Settings {
    let path = settings_path(app_data_dir);
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub async fn save_settings(app_data_dir: &Path, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app_data_dir);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }

    let mut to_persist = settings.clone();
    to_persist
        .pristine_default_sources
        .retain(|u| to_persist.depot_sources.contains(u));

    let content = serde_json::to_string_pretty(&to_persist)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}
