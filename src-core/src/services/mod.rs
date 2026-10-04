pub mod lua_parser;
pub mod st_parser;
pub mod vdf_parser;
pub mod multi_repo_search;
pub mod manifest_hub_api;
pub mod hubcap_api;
pub mod ryuu_api;
pub mod depot_sources;
pub mod depot_keys_generator;
pub mod depot_runner;
pub mod steam_store_api;
pub mod settings;
pub mod embedded_tools;
pub mod history;
pub mod followup;
pub mod install_state;
pub mod telemetry;
pub mod emulator;
pub mod depot_info;
pub mod steam_library;
pub mod steamless;
pub mod steam_api_bypass;
pub mod steam_pics;
pub mod steam_session;
pub mod steam_manifest;
pub mod steam_cdn;
pub mod steam_chunks;
pub mod steam_downloader;
pub mod speed_limit;
pub mod disk_space;
pub mod manifest_code_provider;
pub mod debug_log;
pub mod diag;
pub mod events;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;
pub struct AppState {
    pub active_jobs: Arc<Mutex<HashMap<String, JobInfo>>>,
    pub http_client: reqwest::Client,
    pub steam_cache: Arc<Mutex<HashMap<String, serde_json::Value>>>,
    pub telemetry: Option<telemetry::Telemetry>,
    pub steam_session: Arc<steam_session::SteamSession>,
    pub shutdown_flush_done: Arc<AtomicBool>,
}

pub struct JobInfo {
    pub status: String,
    pub child_pid: Option<u32>,
    pub depot_dirs: Vec<String>,
    pub cancel_flag: Arc<AtomicBool>,
    pub pause_flag: Arc<AtomicBool>,
    pub config_snapshot: Option<serde_json::Value>,
    pub started_at: Option<String>,
    pub game_name: Option<String>,
    pub header_image: Option<String>,
    pub work_dir: Option<String>,
    pub history_written: bool,
    #[cfg(target_os = "windows")]
    pub job_object: Option<Arc<depot_runner::win_job::JobObject>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            steam_cache: Arc::new(Mutex::new(HashMap::new())),
            telemetry: None,
            steam_session: Arc::new(steam_session::SteamSession::new()),
            shutdown_flush_done: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn has_active_downloads(&self) -> bool {
        // try_lock so this stays callable from the UI thread; treat contention
        // as "active" because the mutex is only held during job mutation.
        if let Ok(jobs) = self.active_jobs.try_lock() {
            jobs.values().any(|j| j.status == "downloading" || j.status == "running")
        } else {
            true
        }
    }
}
