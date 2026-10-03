use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use uuid::Uuid;

use crate::services::depot_keys_generator;
use crate::services::depot_runner::{self, emit_progress, DepotRunConfig, ProgressEvent};
use crate::services::depot_sources;
use crate::services::diag;
use crate::services::events::{Sink, DOWNLOAD_PROGRESS};
use crate::services::history;
use crate::services::hubcap_api;
use crate::services::install_state;
use crate::services::lua_parser::DepotInfo;
use crate::services::manifest_hub_api;
use crate::services::ryuu_api;
use crate::services::settings as settings_service;
use crate::services::steam_downloader::{
    download_depot_from_local_manifest, download_depot_native, NativeDownloadProgress,
};
use crate::services::steam_store_api;
use crate::services::{AppState, JobInfo};
use std::sync::Arc;

const JOB_RETENTION: std::time::Duration = std::time::Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct DownloadConfig {
    #[serde(rename = "mainAppId", alias = "app_id")]
    pub app_id: String,
    #[serde(rename = "gameName", alias = "game_name")]
    pub game_name: Option<String>,
    #[serde(rename = "selectedDepots", alias = "depots")]
    pub depots: Vec<DepotConfig>,
    #[allow(dead_code)] // Deserialized from frontend JSON but not read directly by backend
    pub mode: Option<String>,
    #[serde(rename = "keyVdfKeys", alias = "key_vdf_keys")]
    pub key_vdf_keys: Option<HashMap<String, String>>,
    #[serde(rename = "downloadDir", alias = "download_location")]
    pub download_location: Option<String>,
    #[serde(rename = "manifestHubApiKey")]
    pub manifest_hub_api_key: Option<String>,
    #[serde(rename = "headerImage", default)]
    pub header_image: Option<String>,
    #[serde(rename = "sourceType", default)]
    pub source_type: Option<String>,
    #[serde(rename = "updateDir", alias = "update_dir", default)]
    pub update_dir: Option<String>,
    #[serde(rename = "speedLimit", alias = "speed_limit", default)]
    pub speed_limit: Option<String>,
}

impl DownloadConfig {
    pub fn update_target(&self) -> Option<PathBuf> {
        self.update_dir
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
    }
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct DepotConfig {
    #[serde(rename = "depotId", alias = "depot_id")]
    pub depot_id: String,
    #[serde(rename = "manifestId", alias = "manifest_id")]
    pub manifest_id: String,
    #[serde(rename = "customManifestId", alias = "custom_manifest_id")]
    pub custom_manifest_id: Option<String>,
    #[serde(rename = "depotKey", alias = "depot_key")]
    pub depot_key: Option<String>,
    #[serde(rename = "uploadedManifestPath")]
    pub uploaded_manifest_path: Option<String>,
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,
}

// Trust-boundary validation: pipeline assumes all IDs parse cleanly.
fn validate_download_ids(config: &DownloadConfig) -> Result<(), String> {
    config.app_id.parse::<u64>().map_err(|_| {
        format!(
            "Invalid App ID '{}': expected a numeric Steam App ID",
            config.app_id
        )
    })?;

    for depot in &config.depots {
        depot.depot_id.parse::<u64>().map_err(|_| {
            format!(
                "Invalid depot ID '{}': expected a numeric Steam depot ID",
                depot.depot_id
            )
        })?;
    }

    Ok(())
}

pub async fn start_download(
    sink: Sink,
    state: &AppState,
    app_data_dir: &Path,
    config: DownloadConfig,
) -> Result<serde_json::Value, String> {
    validate_download_ids(&config)?;

    let job_id = Uuid::new_v4().to_string();
    crate::dlog!(
        "ipc",
        "start_download(app_id={}, depots={}) -> job {}",
        config.app_id,
        config.depots.len(),
        job_id
    );

    let base_dir = resolve_download_dir(config.download_location.as_deref()).unwrap_or_else(|| {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Documents").join("SteamDownloads")
    });

    tokio::fs::create_dir_all(&base_dir)
        .await
        .map_err(|e| format!("Cannot create download directory: {}", e))?;

    let mut folder_name = config.app_id.clone();
    let mut game_name = config.game_name.clone();
    let mut header_image: Option<String> = config.header_image.clone();

    if game_name.is_none() {
        match steam_store_api::get_game_info(&state.http_client, &state.steam_cache, &config.app_id)
            .await
        {
            Ok(Some(info)) => {
                game_name = info.name.clone();
                header_image = info.header_image.clone();
                if let Some(ref name) = info.name {
                    let sanitized = steam_store_api::sanitize_game_name(name);
                    if !sanitized.is_empty() {
                        folder_name = format!("{} - {}", config.app_id, sanitized);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!(
                    "[Download] Steam Store lookup failed for app {}: {}. Using App ID as folder name.",
                    config.app_id, e
                );
            }
        }
    } else if let Some(ref name) = game_name {
        let sanitized = steam_store_api::sanitize_game_name(name);
        if !sanitized.is_empty() {
            folder_name = format!("{} - {}", config.app_id, sanitized);
        }
    }

    let (base_dir, folder_name) = match config.update_target() {
        Some(target) => {
            if !target.is_dir() {
                return Err(format!(
                    "Folder to update not found: {}",
                    target.display()
                ));
            }
            let name = target
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .ok_or_else(|| format!("Invalid folder to update: {}", target.display()))?;
            let parent = target
                .parent()
                .map(Path::to_path_buf)
                .ok_or_else(|| format!("Invalid folder to update: {}", target.display()))?;
            (parent, name)
        }
        None => (base_dir, folder_name),
    };
    let download_dir = base_dir.join(&folder_name);

    {
        let mut jobs = state.active_jobs.lock().await;
        jobs.insert(
            job_id.clone(),
            JobInfo {
                status: "running".to_string(),
                child_pid: None,
                depot_dirs: Vec::new(),
                cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                pause_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                config_snapshot: serde_json::to_value(&config).ok(),
                started_at: Some(chrono::Utc::now().to_rfc3339()),
                game_name: game_name.clone(),
                header_image: header_image.clone(),
                work_dir: Some(download_dir.to_string_lossy().to_string()),
                history_written: false,
                #[cfg(target_os = "windows")]
                job_object: None,
            },
        );
    }

    let response = serde_json::json!({
        "jobId": job_id,
        "downloadDir": download_dir.to_string_lossy(),
        "folderName": folder_name,
    });

    let job_id_clone = job_id.clone();
    let sink_clone = sink.clone();
    let http_client = state.http_client.clone();
    let active_jobs = state.active_jobs.clone();
    let steam_cache = state.steam_cache.clone();
    let steam_session = state.steam_session.clone();
    let app_data_dir = app_data_dir.to_path_buf();
    let download_dir_for_history = download_dir.to_string_lossy().to_string();
    tokio::spawn(async move {
        let started_at = chrono::Utc::now();

        let state_ref = AppState {
            active_jobs: active_jobs.clone(),
            http_client: http_client.clone(),
            steam_cache: steam_cache.clone(),
            telemetry: None,
            steam_session: steam_session.clone(),
            shutdown_flush_done: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };

        let result = run_download_pipeline(
            &sink_clone,
            &state_ref,
            &job_id_clone,
            &config,
            &base_dir,
            &folder_name,
            game_name.as_deref(),
            header_image.as_deref(),
            &app_data_dir,
        )
        .await;

        let (is_cancelled, history_already_written) = {
            let jobs = active_jobs.lock().await;
            jobs.get(&job_id_clone)
                .map(|j| (j.status == "cancelled", j.history_written))
                .unwrap_or((false, false))
        };

        match result {
            Ok(_) => {
                if is_cancelled && !history_already_written {
                    let entry = history::HistoryEntry {
                        id: Uuid::new_v4().to_string(),
                        app_id: config.app_id.clone(),
                        game_name: game_name.clone(),
                        header_image: header_image.clone(),
                        depot_count: config.depots.len(),
                        depots_downloaded: 0,
                        status: "cancelled".to_string(),
                        download_dir: download_dir_for_history.clone(),
                        started_at: started_at.to_rfc3339(),
                        completed_at: Some(chrono::Utc::now().to_rfc3339()),
                        source_repo: None,
                        depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
                        resume_payload: None,
                    };
                    if let Err(err) = history::add_entry(&app_data_dir, entry).await {
                        eprintln!(
                            "[Download] Failed to record cancelled job in history: {}",
                            err
                        );
                    }
                }
            }
            Err(e) => {
                if !is_cancelled {
                    let mut event = ProgressEvent::new("error", &job_id_clone);
                    event.message = Some(format!("Unexpected error: {}", e));
                    emit_progress(&sink_clone, &event);
                }

                if !history_already_written {
                    let entry = history::HistoryEntry {
                        id: Uuid::new_v4().to_string(),
                        app_id: config.app_id.clone(),
                        game_name: game_name.clone(),
                        header_image: header_image.clone(),
                        depot_count: config.depots.len(),
                        depots_downloaded: 0,
                        status: if is_cancelled { "cancelled" } else { "failed" }.to_string(),
                        download_dir: download_dir_for_history.clone(),
                        started_at: started_at.to_rfc3339(),
                        completed_at: Some(chrono::Utc::now().to_rfc3339()),
                        source_repo: None,
                        depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
                        resume_payload: None,
                    };
                    if let Err(err) = history::add_entry(&app_data_dir, entry).await {
                        eprintln!("[Download] Failed to record failed job in history: {}", err);
                    }
                }
            }
        }

        let active_jobs_cleanup = active_jobs.clone();
        let job_id_cleanup = job_id_clone.clone();
        tokio::spawn(async move {
            tokio::time::sleep(JOB_RETENTION).await;
            let mut jobs = active_jobs_cleanup.lock().await;
            jobs.remove(&job_id_cleanup);
        });
    });

    Ok(response)
}

async fn run_download_pipeline(
    sink: &Sink,
    state: &AppState,
    job_id: &str,
    config: &DownloadConfig,
    base_dir: &Path,
    folder_name: &str,
    _game_name: Option<&str>,
    _header_image: Option<&str>,
    app_data_dir: &Path,
) -> Result<(), String> {
    let _started_at = chrono::Utc::now();
    let started = std::time::Instant::now();
    let work_dir = base_dir.join(folder_name);
    let loaded_settings = settings_service::load_settings(app_data_dir).await;
    let depot_sources_list = loaded_settings.depot_sources.clone();
    let use_native = loaded_settings.use_native_downloader;
    let engine_label: &'static str = if use_native { "native" } else { "ddm" };
    let is_hubcap = config.source_type.as_deref() == Some("hubcap");
    let is_ryuu = config.source_type.as_deref() == Some("ryuu");
    let is_cached_manifest_source = is_hubcap || is_ryuu;
    let cached_source_label = if is_ryuu {
        Some("Ryuu")
    } else if is_hubcap {
        Some("Hubcap")
    } else {
        None
    };

    tokio::fs::create_dir_all(&work_dir)
        .await
        .map_err(|e| format!("Failed to create download directory: {}", e))?;

    if let Some(disk_info) = get_disk_space_info(base_dir) {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("disk_space".to_string());
        event.free_gb = Some(disk_info.0);
        event.drive = Some(disk_info.1);
        emit_progress(sink, &event);
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let uploaded_depots: Vec<&DepotConfig> = config
        .depots
        .iter()
        .filter(|d| d.uploaded_manifest_path.is_some())
        .collect();
    let custom_depots: Vec<&DepotConfig> = config
        .depots
        .iter()
        .filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_some())
        .collect();
    let standard_depots: Vec<&DepotConfig> = config
        .depots
        .iter()
        .filter(|d| d.uploaded_manifest_path.is_none() && d.custom_manifest_id.is_none())
        .collect();

    if !standard_depots.is_empty() && !use_native {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("checking_branch".to_string());
        event.app_id = Some(config.app_id.clone());
        emit_progress(sink, &event);

        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        if let Some(source_label) = cached_source_label {
            let mut event = ProgressEvent::new("status", job_id);
            event.step = Some("branch_found".to_string());
            event.app_id = Some(config.app_id.clone());
            event.last_updated = Some(format!("Source: {}", source_label));
            emit_progress(sink, &event);
        } else {
            if depot_sources_list.is_empty() {
                let mut event = ProgressEvent::new("error", job_id);
                event.message = Some(
                    "No manifest sources configured. Add one in Settings → Advanced Settings → Manifest Sources."
                        .to_string(),
                );
                emit_progress(sink, &event);
                return Ok(());
            }

            match depot_sources::probe_app(&state.http_client, &depot_sources_list, &config.app_id)
                .await
            {
                depot_sources::ProbeOutcome::Found => {
                    let mut event = ProgressEvent::new("status", job_id);
                    event.step = Some("branch_found".to_string());
                    event.app_id = Some(config.app_id.clone());
                    event.last_updated = Some("Source: configured manifest source".to_string());
                    emit_progress(sink, &event);
                }
                depot_sources::ProbeOutcome::Missing => {
                    let mut event = ProgressEvent::new("error", job_id);
                    event.message = Some(format!(
                        "App {} is not present in any configured manifest source.",
                        config.app_id
                    ));
                    event.diag = Some(diag::summarize(
                        &[diag::DepotDiag::failed_at(
                            diag::Stage::SourceProbe,
                            diag::NOT_FOUND,
                            diag::SourceKind::DepotSource,
                        )],
                        &[],
                        started.elapsed().as_secs(),
                        engine_label,
                    ));
                    emit_progress(sink, &event);
                    return Ok(());
                }
                depot_sources::ProbeOutcome::Inconclusive(class) => {
                    let mut event = ProgressEvent::new("error", job_id);
                    event.message = Some(format!(
                        "Could not reach your manifest sources to check for app {} ({}). \
                         This is a problem with the source, not with the app — try again shortly.",
                        config.app_id, class
                    ));
                    event.diag = Some(diag::summarize(
                        &[diag::DepotDiag::failed_at(
                            diag::Stage::SourceProbe,
                            class,
                            diag::SourceKind::DepotSource,
                        )],
                        &[],
                        started.elapsed().as_secs(),
                        engine_label,
                    ));
                    emit_progress(sink, &event);
                    return Ok(());
                }
                depot_sources::ProbeOutcome::NoSources => {
                    let mut event = ProgressEvent::new("error", job_id);
                    event.message = Some(
                        "No usable manifest sources configured. Add one in Settings → Advanced Settings → Manifest Sources."
                            .to_string(),
                    );
                    event.diag = Some(diag::summarize(
                        &[diag::DepotDiag::failed(
                            diag::Stage::SourceProbe,
                            diag::NO_SOURCES,
                        )],
                        &[],
                        started.elapsed().as_secs(),
                        engine_label,
                    ));
                    emit_progress(sink, &event);
                    return Ok(());
                }
            }
        }
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let total_manifests = config.depots.len();
    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("downloading_manifests".to_string());
    event.total = Some(total_manifests);
    emit_progress(sink, &event);

    let mut manifest_results: Vec<(String, bool)> = Vec::new();
    let mut manifest_diags: Vec<diag::DepotDiag> = Vec::new();

    let manifest_source_kind = if is_ryuu {
        diag::SourceKind::Ryuu
    } else if is_hubcap {
        diag::SourceKind::Hubcap
    } else {
        diag::SourceKind::DepotSource
    };

    for depot in &uploaded_depots {
        if let Some(ref uploaded_path) = depot.uploaded_manifest_path {
            let manifest_id = depot
                .custom_manifest_id
                .as_deref()
                .unwrap_or(&depot.manifest_id);
            let filename = format!("{}_{}.manifest", depot.depot_id, manifest_id);
            let dest_path = work_dir.join(&filename);

            match tokio::fs::copy(uploaded_path, &dest_path).await {
                Ok(_) => {
                    let mut event = ProgressEvent::new("status", job_id);
                    event.step = Some("downloading_manifest".to_string());
                    event.depot_id = Some(depot.depot_id.clone());
                    event.manifest_id = Some(manifest_id.to_string());
                    event.filename = Some(filename);
                    event.message = Some("Using uploaded manifest file".to_string());
                    emit_progress(sink, &event);
                    manifest_results.push((depot.depot_id.clone(), true));
                    manifest_diags.push(diag::DepotDiag::ok(diag::SourceKind::Uploaded));
                }
                Err(e) => {
                    let mut event = ProgressEvent::new("error", job_id);
                    event.depot_id = Some(depot.depot_id.clone());
                    event.message = Some(format!(
                        "Failed to use uploaded manifest for depot {}: {}",
                        depot.depot_id, e
                    ));
                    emit_progress(sink, &event);
                    manifest_results.push((depot.depot_id.clone(), false));
                    manifest_diags.push(diag::DepotDiag::failed_at(
                        diag::Stage::Disk,
                        diag::IO,
                        diag::SourceKind::Uploaded,
                    ));
                }
            }
        }
    }

    for depot in &standard_depots {
        if use_native {
            manifest_results.push((depot.depot_id.clone(), true));
            manifest_diags.push(diag::DepotDiag::ok(diag::SourceKind::SteamDirect));
            continue;
        }
        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_manifest".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.manifest_id = Some(depot.manifest_id.clone());
        emit_progress(sink, &event);

        let manifest_result = if is_ryuu {
            ryuu_api::copy_cached_manifest(
                app_data_dir,
                &config.app_id,
                &depot.depot_id,
                &depot.manifest_id,
                &work_dir,
            )
            .await
        } else if is_hubcap {
            hubcap_api::copy_cached_manifest(
                app_data_dir,
                &config.app_id,
                &depot.depot_id,
                &depot.manifest_id,
                &work_dir,
            )
            .await
        } else {
            depot_sources::download_manifest_file(
                &state.http_client,
                &depot_sources_list,
                &config.app_id,
                &depot.depot_id,
                &depot.manifest_id,
                &work_dir,
            )
            .await
        };

        match manifest_result {
            Ok(_) => {
                manifest_results.push((depot.depot_id.clone(), true));
                manifest_diags.push(diag::DepotDiag::ok(manifest_source_kind));
            }
            Err(e) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.depot_id = Some(depot.depot_id.clone());
                event.message = Some(format!(
                    "Failed to download manifest for depot {}: {}",
                    depot.depot_id, e
                ));
                emit_progress(sink, &event);
                manifest_results.push((depot.depot_id.clone(), false));
                let (stage, class) = diag::classify_pipeline_error(&e);
                manifest_diags.push(diag::DepotDiag::failed_at(
                    stage,
                    class,
                    manifest_source_kind,
                ));
            }
        }
    }

    const MANIFEST_HUB_SPACING_MS: u64 = 2000;
    for (idx, depot) in custom_depots.iter().enumerate() {
        if use_native {
            manifest_results.push((depot.depot_id.clone(), true));
            manifest_diags.push(diag::DepotDiag::ok(diag::SourceKind::SteamDirect));
            continue;
        }
        if check_cancelled(state, job_id).await {
            return Ok(());
        }

        if idx > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(MANIFEST_HUB_SPACING_MS)).await;
            if check_cancelled(state, job_id).await {
                return Ok(());
            }
        }

        let manifest_id = depot
            .custom_manifest_id
            .as_deref()
            .unwrap_or(&depot.manifest_id);

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_manifest_hub".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.manifest_id = Some(manifest_id.to_string());
        emit_progress(sink, &event);

        let api_key = config.manifest_hub_api_key.as_deref().unwrap_or_default();

        let mut last_err: Option<String> = None;
        for attempt in 0..3 {
            match manifest_hub_api::download_from_manifest_hub(
                &state.http_client,
                &config.app_id,
                &depot.depot_id,
                manifest_id,
                &work_dir,
                api_key,
            )
            .await
            {
                Ok(_) => {
                    last_err = None;
                    break;
                }
                Err(e) => {
                    let is_rate_limited = e.contains("429")
                        || e.to_lowercase().contains("too many requests")
                        || e.contains("error code: 1015");
                    last_err = Some(e);
                    if !is_rate_limited || attempt == 2 {
                        break;
                    }
                    let backoff_ms = 4000u64 << attempt; // 4s, 8s
                    let mut event = ProgressEvent::new("status", job_id);
                    event.step = Some("manifest_hub_rate_limited".to_string());
                    event.depot_id = Some(depot.depot_id.clone());
                    event.message = Some(format!(
                        "ManifestHub rate-limited depot {} — backing off {}s before retry",
                        depot.depot_id,
                        backoff_ms / 1000
                    ));
                    emit_progress(sink, &event);
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    if check_cancelled(state, job_id).await {
                        return Ok(());
                    }
                }
            }
        }

        match last_err {
            None => {
                manifest_results.push((depot.depot_id.clone(), true));
                manifest_diags.push(diag::DepotDiag::ok(diag::SourceKind::ManifestHub));
            }
            Some(e) => {
                let mut event = ProgressEvent::new("error", job_id);
                event.depot_id = Some(depot.depot_id.clone());
                event.message = Some(format!(
                    "Failed to download custom manifest for depot {}: {}",
                    depot.depot_id, e
                ));
                emit_progress(sink, &event);
                manifest_results.push((depot.depot_id.clone(), false));
                let class = if e.contains("429")
                    || e.to_lowercase().contains("too many requests")
                    || e.contains("error code: 1015")
                {
                    diag::RATE_LIMITED
                } else {
                    diag::classify_pipeline_error(&e).1
                };
                manifest_diags.push(diag::DepotDiag::failed_at(
                    diag::Stage::ManifestFetch,
                    class,
                    diag::SourceKind::ManifestHub,
                ));
            }
        }
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let success_count = manifest_results.iter().filter(|(_, s)| *s).count();
    if success_count == 0 && !manifest_results.is_empty() {
        let error_msg = "All manifest downloads failed".to_string();
        let mut event = ProgressEvent::new("error", job_id);
        event.message = Some(error_msg.clone());
        event.diag = Some(diag::summarize(
            &manifest_diags,
            &diag::tried_from_diags(&manifest_diags),
            started.elapsed().as_secs(),
            engine_label,
        ));
        emit_progress(sink, &event);

        let entry = history::HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: config.app_id.clone(),
            game_name: _game_name.map(|s| s.to_string()),
            header_image: _header_image.map(|s| s.to_string()),
            depot_count: config.depots.len(),
            depots_downloaded: 0,
            status: "failed".to_string(),
            download_dir: work_dir.to_string_lossy().to_string(),
            started_at: _started_at.to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            source_repo: None,
            depot_ids: config.depots.iter().map(|d| d.depot_id.clone()).collect(),
            resume_payload: None,
        };
        if let Err(err) = history::add_entry(app_data_dir, entry).await {
            eprintln!(
                "[Download] Failed to record failed download in history: {}",
                err
            );
        }

        return Ok(());
    }

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("generating_keys".to_string());
    emit_progress(sink, &event);

    let mut depot_infos: Vec<DepotInfo> = config
        .depots
        .iter()
        .map(|d| {
            let mut key = d.depot_key.clone();

            if key.is_none() {
                if let Some(ref kvk) = config.key_vdf_keys {
                    key = kvk.get(&d.depot_id).cloned();
                }
            }

            DepotInfo {
                depot_id: d
                    .depot_id
                    .parse()
                    .expect("depot IDs are validated at entry"),
                depot_key: key,
                manifest_id: Some(
                    d.custom_manifest_id
                        .as_deref()
                        .unwrap_or(&d.manifest_id)
                        .to_string(),
                ),
                size_bytes: None,
            }
        })
        .collect();

    if !is_cached_manifest_source && depot_infos.iter().any(|d| d.depot_key.is_none()) {
        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("downloading_keyvdf".to_string());
        emit_progress(sink, &event);

        match depot_sources::download_text_file(
            &state.http_client,
            &depot_sources_list,
            &config.app_id,
            "key.vdf",
        )
        .await
        {
            Ok(vdf_content) => {
                let vdf_keys = crate::services::vdf_parser::parse_key_vdf(&vdf_content, None);
                for depot in &mut depot_infos {
                    if depot.depot_key.is_none() {
                        if let Some(key) = vdf_keys.get(&depot.depot_id.to_string()) {
                            depot.depot_key = Some(key.clone());
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[Download] Key.vdf download/parse skipped: {}", e);
            }
        }
    }

    let keys_result = depot_keys_generator::generate_depot_keys(
        config.app_id.parse().expect("app_id is validated at entry"),
        &depot_infos,
        Some(folder_name),
        base_dir,
    )
    .await?;

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("keys_generated".to_string());
    event.depot_count = Some(keys_result.depot_count);
    emit_progress(sink, &event);

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let successful_depot_ids: Vec<String> = manifest_results
        .iter()
        .filter(|(_, s)| *s)
        .map(|(id, _)| id.clone())
        .collect();

    let run_depots: Vec<DepotRunConfig> = config
        .depots
        .iter()
        .filter(|d| successful_depot_ids.contains(&d.depot_id))
        .map(|d| DepotRunConfig {
            depot_id: d.depot_id.clone(),
            manifest_id: d
                .custom_manifest_id
                .as_deref()
                .unwrap_or(&d.manifest_id)
                .to_string(),
            display_name: d.display_name.clone(),
        })
        .collect();

    let mut event = ProgressEvent::new("status", job_id);
    event.step = Some("starting_downloader".to_string());
    event.total = Some(run_depots.len());
    emit_progress(sink, &event);

    let settings = crate::services::settings::load_settings(app_data_dir).await;
    let speed_limit_text = config
        .speed_limit
        .as_deref()
        .unwrap_or(&settings.download_speed_limit);
    crate::services::speed_limit::set_limit(
        crate::services::speed_limit::parse_speed_limit(speed_limit_text)?,
    );
    let extra_args = if settings.dd_extra_args.is_empty() {
        vec![
            "-max-downloads".to_string(),
            "8".to_string(),
            "-verify-all".to_string(),
        ]
    } else {
        settings.dd_extra_args.clone()
    };

    let download_results = if settings.use_native_downloader {
        run_native_pipeline(
            sink,
            state,
            &config.app_id,
            &run_depots,
            &depot_infos,
            &work_dir,
            job_id,
            config.manifest_hub_api_key.as_deref(),
            &depot_sources_list,
            is_hubcap,
            app_data_dir,
            settings.native_chunk_concurrency,
            config.update_target().is_some(),
        )
        .await?
    } else {
        {
            let mut jobs = state.active_jobs.lock().await;
            if let Some(job) = jobs.get_mut(job_id) {
                for depot in run_depots.iter() {
                    let p = work_dir
                        .join("depots")
                        .join(&depot.depot_id)
                        .to_string_lossy()
                        .to_string();
                    if !job.depot_dirs.contains(&p) {
                        job.depot_dirs.push(p);
                    }
                }
                let p = work_dir
                    .join("depots")
                    .join(".DepotDownloader")
                    .to_string_lossy()
                    .to_string();
                if !job.depot_dirs.contains(&p) {
                    job.depot_dirs.push(p);
                }
            }
        }
        let exe_path = depot_runner::get_exe_path_async().await?;
        depot_runner::run_all_depots(
            sink,
            &exe_path,
            &config.app_id,
            &run_depots,
            &work_dir,
            &extra_args,
            job_id,
            state,
        )
        .await?
    };

    if check_cancelled(state, job_id).await {
        return Ok(());
    }

    let dd_cache = work_dir.join("depots").join(".DepotDownloader");
    if dd_cache.exists() {
        if let Err(e) = tokio::fs::remove_dir_all(&dd_cache).await {
            eprintln!("[Cleanup] Failed to remove .DepotDownloader cache: {}", e);
        }
    }

    let dl_success_count = download_results
        .iter()
        .filter(|r| r["success"].as_bool().unwrap_or(false))
        .count();
    let selected_total = config.depots.len();
    let outcome = diag::Outcome::from_counts(dl_success_count, selected_total);

    let mut all_diags = diag::from_download_results(
        &download_results,
        if use_native {
            diag::SourceKind::SteamDirect
        } else {
            manifest_source_kind
        },
    );
    all_diags.extend(manifest_diags.iter().filter(|d| !d.ok).copied());

    let mut event = ProgressEvent::new("complete", job_id);
    event.message = Some(match outcome {
        diag::Outcome::Complete => format!(
            "Download complete. {}/{} depots downloaded successfully.",
            dl_success_count, selected_total
        ),
        _ => format!(
            "Download incomplete — {}/{} depots downloaded. The rest could not be fetched.",
            dl_success_count, selected_total
        ),
    });
    event.results = Some(serde_json::Value::Array(download_results));
    event.diag = Some(diag::summarize(
        &all_diags,
        &diag::tried_from_results(
            event
                .results
                .as_ref()
                .and_then(|v| v.as_array())
                .map(|a| a.as_slice())
                .unwrap_or(&[]),
        ),
        started.elapsed().as_secs(),
        engine_label,
    ));
    emit_progress(sink, &event);

    let status = outcome.as_str();
    if status == "failed" {
        let entry = history::HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: config.app_id.clone(),
            game_name: _game_name.map(|s| s.to_string()),
            header_image: _header_image.map(|s| s.to_string()),
            depot_count: run_depots.len(),
            depots_downloaded: dl_success_count,
            status: status.to_string(),
            download_dir: work_dir.to_string_lossy().to_string(),
            started_at: _started_at.to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            source_repo: None,
            depot_ids: run_depots.iter().map(|d| d.depot_id.clone()).collect(),
            resume_payload: None,
        };
        if let Err(err) = history::add_entry(app_data_dir, entry).await {
            eprintln!("[Download] Failed to record failed job in history: {}", err);
        }
    }

    {
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = "complete".to_string();
        }
    }

    Ok(())
}

pub async fn cancel_download(
    sink: &Sink,
    state: &AppState,
    app_data_dir: &Path,
    job_id: String,
) -> Result<(), String> {
    crate::dlog!("ipc", "cancel_download({}) entry", job_id);
    let (
        depot_dirs,
        cancel_flag,
        snapshot,
        started_at,
        game_name,
        header_image,
        work_dir_str,
        claim_ok,
    ) = {
        let mut jobs = state.active_jobs.lock().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or_else(|| "Job not found".to_string())?;
        let claim_ok = job.status == "running" && !job.history_written;
        if claim_ok {
            job.history_written = true;
        }
        (
            job.depot_dirs.clone(),
            job.cancel_flag.clone(),
            job.config_snapshot.clone(),
            job.started_at.clone(),
            job.game_name.clone(),
            job.header_image.clone(),
            job.work_dir.clone(),
            claim_ok,
        )
    };

    if !claim_ok {
        crate::dlog!(
            "ipc",
            "cancel_download({}) early-return: not running or already claimed",
            job_id
        );
        return Ok(());
    }
    crate::dlog!(
        "ipc",
        "cancel_download({}) claim acquired, setting flags",
        job_id
    );

    cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    depot_runner::kill_job(state, &job_id).await;

    let is_update = snapshot
        .as_ref()
        .and_then(|p| p.get("updateDir"))
        .and_then(|v| v.as_str())
        .is_some_and(|d| !d.trim().is_empty());
    let keep_files = is_update
        || crate::services::settings::load_settings(app_data_dir)
            .await
            .cancel_keep_files;

    let mut event = ProgressEvent::new("cancelled", &job_id);
    event.step = Some(if keep_files {
        "cancelled_kept".to_string()
    } else {
        "cancelled_cleanup".to_string()
    });
    event.diag = Some(serde_json::json!({
        "outcome": diag::Outcome::Cancelled.as_str(),
        "keep_files": keep_files,
    }));
    emit_progress(sink, &event);

    let snapshot_app_id = snapshot
        .as_ref()
        .and_then(|p| p.get("mainAppId"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let snapshot_depot_ids: Vec<String> = snapshot
        .as_ref()
        .and_then(|p| p.get("selectedDepots"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| d.get("depotId").and_then(|v| v.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let entry = crate::services::history::HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: snapshot_app_id,
        game_name,
        header_image,
        depot_count: snapshot_depot_ids.len(),
        depots_downloaded: 0,
        status: if keep_files {
            "cancelled_resumable"
        } else {
            "cancelled"
        }
        .to_string(),
        download_dir: work_dir_str.clone().unwrap_or_default(),
        started_at: started_at.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        source_repo: None,
        depot_ids: snapshot_depot_ids,
        resume_payload: if keep_files { snapshot } else { None },
    };
    if let Err(err) = crate::services::history::add_entry(app_data_dir, entry).await {
        eprintln!("[Cancel] Failed to record history entry: {}", err);
        let mut jobs = state.active_jobs.lock().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.history_written = false;
        }
    }

    if keep_files {
        return Ok(());
    }

    let depot_dirs_final: Vec<String> = if depot_dirs.is_empty() {
        let jobs = state.active_jobs.lock().await;
        jobs.get(&job_id)
            .map(|j| j.depot_dirs.clone())
            .unwrap_or_default()
    } else {
        depot_dirs
    };

    if !depot_dirs_final.is_empty() {
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
            for dir in depot_dirs_final {
                let dir_path = std::path::PathBuf::from(&dir);
                if !is_safe_depot_cleanup_path(&dir_path) {
                    eprintln!("[Cancel] Refusing to delete unsafe path: {:?}", dir_path);
                    continue;
                }
                if !dir_path.exists() {
                    continue;
                }
                for attempt in 0..6 {
                    match tokio::fs::remove_dir_all(&dir_path).await {
                        Ok(_) => {
                            eprintln!("[Cancel] Cleaned up depot directory: {:?}", dir_path);
                            break;
                        }
                        Err(e) => {
                            eprintln!(
                                "[Cancel] Attempt {} to delete {:?} failed: {}",
                                attempt + 1,
                                dir_path,
                                e
                            );
                            if attempt < 5 {
                                tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

pub async fn pause_download(
    sink: &Sink,
    state: &AppState,
    job_id: String,
    paused: bool,
) -> Result<(), String> {
    crate::dlog!("ipc", "pause_download({}, paused={}) entry", job_id, paused);
    let pause_flag = {
        let jobs = state.active_jobs.lock().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| "Job not found".to_string())?;
        job.pause_flag.clone()
    };
    pause_flag.store(paused, std::sync::atomic::Ordering::SeqCst);
    let mut event = ProgressEvent::new("status", &job_id);
    event.step = Some(if paused {
        "paused".to_string()
    } else {
        "resumed".to_string()
    });
    emit_progress(sink, &event);
    Ok(())
}

async fn check_cancelled(state: &AppState, job_id: &str) -> bool {
    let jobs = state.active_jobs.lock().await;
    jobs.get(job_id)
        .map(|j| j.status == "cancelled")
        .unwrap_or(false)
}

async fn run_native_pipeline(
    sink: &Sink,
    state: &AppState,
    app_id: &str,
    run_depots: &[DepotRunConfig],
    depot_infos: &[DepotInfo],
    work_dir: &Path,
    job_id: &str,
    mh_api_key: Option<&str>,
    depot_sources_list: &[String],
    is_hubcap: bool,
    app_data_dir: &Path,
    chunk_concurrency: u32,
    update_mode: bool,
) -> Result<Vec<serde_json::Value>, String> {
    let app_id_u: u32 = app_id
        .parse()
        .map_err(|_| format!("invalid app id '{}'", app_id))?;
    let session = state.steam_session.clone();
    let mut results: Vec<serde_json::Value> = Vec::with_capacity(run_depots.len());

    let (cancel_flag, pause_flag) = {
        let mut jobs = state.active_jobs.lock().await;
        match jobs.get_mut(job_id) {
            Some(j) => {
                for depot in run_depots.iter() {
                    let folder = depot
                        .display_name
                        .as_deref()
                        .filter(|n| !n.trim().is_empty())
                        .map(|n| format!("{} - {}", sanitize_folder_segment(n), depot.depot_id))
                        .unwrap_or_else(|| depot.depot_id.clone());
                    let p = work_dir
                        .join("depots")
                        .join(&folder)
                        .to_string_lossy()
                        .to_string();
                    if !j.depot_dirs.contains(&p) {
                        j.depot_dirs.push(p);
                    }
                }
                (j.cancel_flag.clone(), j.pause_flag.clone())
            }
            None => (
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
                Arc::new(std::sync::atomic::AtomicBool::new(false)),
            ),
        }
    };

    for (idx, depot) in run_depots.iter().enumerate() {
        if check_cancelled(state, job_id).await {
            return Ok(results);
        }

        let depot_id_u: u32 = depot
            .depot_id
            .parse()
            .map_err(|_| format!("invalid depot id '{}'", depot.depot_id))?;
        let manifest_id_u: u64 = depot
            .manifest_id
            .parse()
            .map_err(|_| format!("invalid manifest id '{}'", depot.manifest_id))?;
        let info = depot_infos
            .iter()
            .find(|d| d.depot_id.to_string() == depot.depot_id)
            .ok_or_else(|| format!("depot {} missing DepotInfo / key", depot.depot_id))?;
        let key_hex = info
            .depot_key
            .as_ref()
            .ok_or_else(|| format!("depot {} has no key", depot.depot_id))?;
        let key_bytes = hex::decode(key_hex.trim())
            .map_err(|e| format!("depot {} key hex invalid: {}", depot.depot_id, e))?;
        if key_bytes.len() != 32 {
            return Err(format!("depot {} key length != 32 bytes", depot.depot_id));
        }
        let mut depot_key = [0u8; 32];
        depot_key.copy_from_slice(&key_bytes);

        let installed = install_state::load(work_dir, &depot.depot_id);
        if update_mode
            && installed
                .as_ref()
                .is_some_and(|i| i.manifest_id == depot.manifest_id)
        {
            let mut event = ProgressEvent::new("status", job_id);
            event.step = Some("depot_up_to_date".to_string());
            event.depot_id = Some(depot.depot_id.clone());
            event.manifest_id = Some(depot.manifest_id.clone());
            event.message = Some(format!(
                "Depot {} is already up to date (manifest {})",
                depot.depot_id, depot.manifest_id
            ));
            emit_progress(sink, &event);
            let mut done = ProgressEvent::new("depot_complete", job_id);
            done.depot_id = Some(depot.depot_id.clone());
            done.current = Some(idx + 1);
            done.total = Some(run_depots.len());
            emit_progress(sink, &done);
            results.push(serde_json::json!({
                "depotId": depot.depot_id,
                "success": true,
                "filesWritten": 0,
                "bytesWritten": 0,
                "upToDate": true,
                "sourcesTried": Vec::<&str>::new(),
            }));
            continue;
        }

        let mut event = ProgressEvent::new("status", job_id);
        event.step = Some("running_downloader".to_string());
        event.depot_id = Some(depot.depot_id.clone());
        event.current = Some(idx + 1);
        event.total = Some(run_depots.len());
        event.command = Some(format!(
            "[native] depot {} manifest {}",
            depot.depot_id, depot.manifest_id
        ));
        emit_progress(sink, &event);

        let sink_cb = sink.clone();
        let depot_id_str = depot.depot_id.clone();
        let job_id_owned = job_id.to_string();
        let progress_cb = move |p: NativeDownloadProgress| {
            sink_cb.emit(
                DOWNLOAD_PROGRESS,
                serde_json::json!({
                    "type": "output",
                    "jobId": job_id_owned,
                    "depotId": depot_id_str,
                    "output": format!("{:.2}% depots/{}/", p.percent, p.depot_id),
                    "stream": "stdout",
                    "completedBytes": p.completed_bytes,
                    "totalBytes": p.total_bytes,
                    "networkBytes": p.network_bytes,
                    "skippedChunks": p.skipped_chunks,
                    "skippedBytes": p.skipped_bytes,
                    "completedChunks": p.completed_chunks,
                    "totalChunks": p.total_chunks,
                    "percent": p.percent,
                }),
            );
        };

        let depot_folder_name = depot
            .display_name
            .as_deref()
            .filter(|n| !n.trim().is_empty())
            .map(|n| format!("{} - {}", sanitize_folder_segment(n), depot.depot_id))
            .unwrap_or_else(|| depot.depot_id.clone());
        let depot_out = work_dir.join("depots").join(&depot_folder_name);
        {
            let mut jobs = state.active_jobs.lock().await;
            if let Some(job) = jobs.get_mut(job_id) {
                let p = depot_out.to_string_lossy().to_string();
                if !job.depot_dirs.contains(&p) {
                    job.depot_dirs.push(p);
                }
            }
        }

        let manifest_already_on_disk = work_dir
            .join(format!("{}_{}.manifest", depot.depot_id, depot.manifest_id))
            .exists();

        let mut sources_tried: Vec<&'static str> = Vec::new();
        let mut no_fallback_left: Option<&'static str> = None;

        let attempt_outcome = if manifest_already_on_disk {
            sources_tried.push(diag::SourceKind::Cached.as_str());
            emit_manifest_source(
                sink,
                job_id,
                &depot.depot_id,
                "cached",
                "Using cached manifest already on disk",
            );
            let path = work_dir.join(format!("{}_{}.manifest", depot.depot_id, depot.manifest_id));
            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    download_depot_from_local_manifest(
                        state.http_client.clone(),
                        session.clone(),
                        app_id_u,
                        depot_id_u,
                        depot_key,
                        bytes,
                        depot_out.clone(),
                        cancel_flag.clone(),
                        pause_flag.clone(),
                        progress_cb,
                        chunk_concurrency,
                    )
                    .await
                }
                Err(e) => Err(format!("read cached manifest failed: {}", e)),
            }
        } else {
            sources_tried.push(diag::SourceKind::SteamDirect.as_str());
            emit_manifest_source(
                sink,
                job_id,
                &depot.depot_id,
                "steam",
                "Fetching manifest directly from Steam CDN",
            );
            let mut steam_result = download_depot_native(
                state.http_client.clone(),
                session.clone(),
                app_id_u,
                depot_id_u,
                manifest_id_u,
                depot_key,
                depot_out.clone(),
                cancel_flag.clone(),
                pause_flag.clone(),
                progress_cb,
                chunk_concurrency,
            )
            .await;

            if let Some(steam_err) = steam_result.as_ref().err().cloned() {
                let mut source_attempt: Option<Result<std::path::PathBuf, String>> = None;

                if is_hubcap {
                    sources_tried.push(diag::SourceKind::Hubcap.as_str());
                    emit_manifest_source(
                        sink,
                        job_id,
                        &depot.depot_id,
                        "hubcap_fallback",
                        &format!("Steam direct failed ({}). Trying Hubcap cache.", steam_err),
                    );
                    let path =
                        work_dir.join(format!("{}_{}.manifest", depot.depot_id, depot.manifest_id));
                    source_attempt = Some(
                        hubcap_api::copy_cached_manifest(
                            app_data_dir,
                            app_id,
                            &depot.depot_id,
                            &depot.manifest_id,
                            work_dir,
                        )
                        .await
                        .map(|_| path),
                    );
                } else if !depot_sources_list.is_empty() {
                    sources_tried.push(diag::SourceKind::DepotSource.as_str());
                    emit_manifest_source(
                        sink,
                        job_id,
                        &depot.depot_id,
                        "depot_source_fallback",
                        &format!(
                            "Steam direct failed ({}). Falling back to configured manifest source.",
                            steam_err
                        ),
                    );
                    source_attempt = Some(
                        depot_sources::download_manifest_file(
                            &state.http_client,
                            depot_sources_list,
                            app_id,
                            &depot.depot_id,
                            &depot.manifest_id,
                            work_dir,
                        )
                        .await,
                    );
                }

                if let Some(result) = source_attempt {
                    match result {
                        Ok(path) => match tokio::fs::read(&path).await {
                            Ok(bytes) => {
                                let sink_cb3 = sink.clone();
                                let depot_id_str3 = depot.depot_id.clone();
                                let job_id_owned3 = job_id.to_string();
                                let progress_cb3 = move |p: NativeDownloadProgress| {
                                    sink_cb3.emit(
                                        DOWNLOAD_PROGRESS,
                                        serde_json::json!({
                                            "type": "output",
                                            "jobId": job_id_owned3,
                                            "depotId": depot_id_str3,
                                            "output": format!("{:.2}% depots/{}/", p.percent, p.depot_id),
                                            "stream": "stdout",
                                            "completedBytes": p.completed_bytes,
                                            "totalBytes": p.total_bytes,
                                            "networkBytes": p.network_bytes,
                                            "skippedChunks": p.skipped_chunks,
                                            "skippedBytes": p.skipped_bytes,
                                            "completedChunks": p.completed_chunks,
                                            "totalChunks": p.total_chunks,
                                            "percent": p.percent,
                                        }),
                                    );
                                };
                                steam_result = download_depot_from_local_manifest(
                                    state.http_client.clone(),
                                    session.clone(),
                                    app_id_u,
                                    depot_id_u,
                                    depot_key,
                                    bytes,
                                    depot_out.clone(),
                                    cancel_flag.clone(),
                                    pause_flag.clone(),
                                    progress_cb3,
                                    chunk_concurrency,
                                )
                                .await;
                            }
                            Err(e) => {
                                steam_result =
                                    Err(format!("read manifest from disk failed: {}", e));
                            }
                        },
                        Err(e) => {
                            steam_result = Err(format!("configured source failed: {}", e));
                        }
                    }
                }
            }

            if let Err(ref steam_err) = steam_result {
                let key_opt = mh_api_key.filter(|k| !k.is_empty());
                if key_opt.is_none() {
                    emit_manifest_source(
                        sink,
                        job_id,
                        &depot.depot_id,
                        "manifesthub_unavailable",
                        &format!(
                            "All sources failed ({}). No ManifestHub API key set — add one and retry.",
                            steam_err
                        ),
                    );
                    no_fallback_left = Some(if depot_sources_list.is_empty() {
                        "no manifest sources configured and No ManifestHub API key"
                    } else {
                        "No ManifestHub API key"
                    });
                }
                if let Some(key) = key_opt {
                    sources_tried.push(diag::SourceKind::ManifestHub.as_str());
                    emit_manifest_source(
                        sink,
                        job_id,
                        &depot.depot_id,
                        "manifesthub_fallback",
                        &format!(
                            "Previous sources failed ({}). Falling back to ManifestHub.",
                            steam_err
                        ),
                    );
                    match manifest_hub_api::download_from_manifest_hub(
                        &state.http_client,
                        app_id,
                        &depot.depot_id,
                        &depot.manifest_id,
                        work_dir,
                        key,
                    )
                    .await
                    {
                        Ok(path) => match tokio::fs::read(&path).await {
                            Ok(bytes) => {
                                let sink_cb2 = sink.clone();
                                let depot_id_str2 = depot.depot_id.clone();
                                let job_id_owned2 = job_id.to_string();
                                let progress_cb2 = move |p: NativeDownloadProgress| {
                                    sink_cb2.emit(
                                        DOWNLOAD_PROGRESS,
                                        serde_json::json!({
                                            "type": "output",
                                            "jobId": job_id_owned2,
                                            "depotId": depot_id_str2,
                                            "output": format!("{:.2}% depots/{}/", p.percent, p.depot_id),
                                            "stream": "stdout",
                                            "completedBytes": p.completed_bytes,
                                            "totalBytes": p.total_bytes,
                                            "networkBytes": p.network_bytes,
                                            "skippedChunks": p.skipped_chunks,
                                            "skippedBytes": p.skipped_bytes,
                                            "completedChunks": p.completed_chunks,
                                            "totalChunks": p.total_chunks,
                                            "percent": p.percent,
                                        }),
                                    );
                                };
                                steam_result = download_depot_from_local_manifest(
                                    state.http_client.clone(),
                                    session.clone(),
                                    app_id_u,
                                    depot_id_u,
                                    depot_key,
                                    bytes,
                                    depot_out.clone(),
                                    cancel_flag.clone(),
                                    pause_flag.clone(),
                                    progress_cb2,
                                    chunk_concurrency,
                                )
                                .await;
                            }
                            Err(e) => {
                                steam_result =
                                    Err(format!("read MH manifest from disk failed: {}", e));
                            }
                        },
                        Err(e) => {
                            steam_result = Err(format!("ManifestHub fallback failed: {}", e));
                        }
                    }
                }
            }
            steam_result
        };

        match attempt_outcome {
            Ok(outcome) => {
                let removed = finish_depot_install(
                    work_dir,
                    &depot_out,
                    &depot.depot_id,
                    &depot.manifest_id,
                    &depot_key,
                    installed.as_ref(),
                    &outcome.manifest_files,
                    update_mode,
                );
                if removed > 0 {
                    let mut event = ProgressEvent::new("status", job_id);
                    event.step = Some("removed_stale_files".to_string());
                    event.depot_id = Some(depot.depot_id.clone());
                    event.message = Some(format!(
                        "Removed {} file(s) that are no longer part of depot {}",
                        removed, depot.depot_id
                    ));
                    emit_progress(sink, &event);
                }
                let mut done = ProgressEvent::new("depot_complete", job_id);
                done.depot_id = Some(depot.depot_id.clone());
                done.current = Some(idx + 1);
                done.total = Some(run_depots.len());
                emit_progress(sink, &done);
                results.push(serde_json::json!({
                    "depotId": depot.depot_id,
                    "success": true,
                    "filesWritten": outcome.files_written,
                    "bytesWritten": outcome.bytes_written,
                    "sourcesTried": sources_tried,
                }));
            }
            Err(e) => {
                let mut err_event = ProgressEvent::new("error", job_id);
                err_event.message = Some(format!(
                    "Native download failed for depot {}: {}",
                    depot.depot_id, e
                ));
                err_event.depot_id = Some(depot.depot_id.clone());
                emit_progress(sink, &err_event);
                let reported = match no_fallback_left {
                    Some(reason) => format!("{} ({})", reason, e),
                    None => e,
                };
                results.push(serde_json::json!({
                    "depotId": depot.depot_id,
                    "success": false,
                    "error": reported,
                    "sourcesTried": sources_tried,
                }));
            }
        }
    }

    Ok(results)
}

#[allow(clippy::too_many_arguments)]
fn finish_depot_install(
    work_dir: &Path,
    depot_out: &Path,
    depot_id: &str,
    manifest_id: &str,
    depot_key: &[u8; 32],
    installed: Option<&install_state::DepotInstall>,
    new_files: &[String],
    update_mode: bool,
) -> usize {
    let mut removed = 0;
    if update_mode {
        let old_manifests = install_state::other_manifest_files(work_dir, depot_id, manifest_id);
        let old_files: Vec<String> = match installed {
            Some(record) if record.manifest_id != manifest_id => record.files.clone(),
            Some(_) => Vec::new(),
            None => old_manifests
                .iter()
                .filter_map(|p| std::fs::read(p).ok())
                .filter_map(|bytes| crate::services::steam_manifest::decode_manifest(&bytes, depot_key).ok())
                .flat_map(|m| install_state::manifest_files(&m))
                .collect(),
        };
        let stale = install_state::stale_files(&old_files, new_files);
        removed = install_state::remove_stale(depot_out, &stale);
        for old in old_manifests {
            let _ = std::fs::remove_file(old);
        }
    }
    let record = install_state::DepotInstall {
        depot_id: depot_id.to_string(),
        manifest_id: manifest_id.to_string(),
        files: new_files.to_vec(),
    };
    if let Err(e) = install_state::save(work_dir, &record) {
        eprintln!("[Download] {}", e);
    }
    removed
}

fn is_safe_depot_cleanup_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let s = path.to_string_lossy();
    if s.trim().is_empty() || s.len() < 6 {
        return false;
    }
    if path.components().count() < 4 {
        return false;
    }
    let has_depot_segment = path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|seg| seg == "depots" || seg == ".DepotDownloader")
            .unwrap_or(false)
    });
    has_depot_segment
}

fn sanitize_folder_segment(input: &str) -> String {
    let forbidden = ['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0'];
    let cleaned: String = input
        .chars()
        .map(|c| {
            if forbidden.contains(&c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "depot".to_string()
    } else if trimmed.len() > 80 {
        trimmed
            .chars()
            .take(80)
            .collect::<String>()
            .trim()
            .to_string()
    } else {
        trimmed
    }
}

fn emit_manifest_source(sink: &Sink, job_id: &str, depot_id: &str, source: &str, note: &str) {
    sink.emit(
        DOWNLOAD_PROGRESS,
        serde_json::json!({
            "type": "manifest_source",
            "jobId": job_id,
            "depotId": depot_id,
            "source": source,
            "message": note,
        }),
    );
}

fn resolve_download_dir(dir_path: Option<&str>) -> Option<PathBuf> {
    let path_str = dir_path?.trim();
    if path_str.is_empty() {
        return None;
    }

    let resolved = PathBuf::from(path_str);
    if !resolved.is_absolute() {
        return None;
    }
    if resolved.to_string_lossy().len() < 3 {
        return None;
    }

    Some(resolved)
}

#[cfg(target_os = "windows")]
fn get_disk_space_info(path: &Path) -> Option<(f64, String)> {
    let path_str = path.to_string_lossy();
    if path_str.len() < 2 {
        return None;
    }

    let drive_letter = path_str.chars().next()?;
    let drive = format!("{}:", drive_letter);

    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-Command",
        &format!("(Get-PSDrive {}).Free", drive_letter),
    ]);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let output = cmd.output().ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let free_bytes: u64 = stdout.trim().parse().ok()?;
    let free_gb = (free_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
    let free_gb = (free_gb * 100.0).round() / 100.0;

    Some((free_gb, drive))
}

#[cfg(target_os = "linux")]
fn get_disk_space_info(path: &Path) -> Option<(f64, String)> {
    use std::ffi::CString;

    let path_str = path.to_string_lossy();
    let c_path = CString::new(path_str.as_ref()).ok()?;

    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        let result = libc::statvfs(c_path.as_ptr(), &mut stat);
        if result != 0 {
            return None;
        }

        let free = (stat.f_bavail as u64) * (stat.f_frsize as u64);
        let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
        let free_gb = (free_gb * 100.0).round() / 100.0;

        Some((free_gb, path_str.to_string()))
    }
}

#[cfg(test)]
mod update_install_tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smd-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("depots/481/sub")).unwrap();
        dir
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn update_removes_files_dropped_by_the_new_version() {
        let work = tmp("update");
        let depot = work.join("depots/481");
        for f in ["game.exe", "sub/old.pak", "save.dat"] {
            std::fs::write(depot.join(f), b"x").unwrap();
        }
        std::fs::write(work.join("481_100.manifest"), b"old").unwrap();
        let old = install_state::DepotInstall {
            depot_id: "481".into(),
            manifest_id: "100".into(),
            files: s(&["game.exe", "sub/old.pak", "gone.bin"]),
        };
        let removed = finish_depot_install(
            &work,
            &depot,
            "481",
            "200",
            &[0u8; 32],
            Some(&old),
            &s(&["game.exe", "new.pak"]),
            true,
        );
        assert_eq!(removed, 1);
        assert!(depot.join("game.exe").exists());
        assert!(depot.join("save.dat").exists());
        assert!(!depot.join("sub").exists());
        assert!(!work.join("481_100.manifest").exists());
        let rec = install_state::load(&work, "481").unwrap();
        assert_eq!(rec.manifest_id, "200");
        assert_eq!(rec.files, s(&["game.exe", "new.pak"]));
        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn a_normal_download_never_deletes_files() {
        let work = tmp("normal");
        let depot = work.join("depots/481");
        std::fs::write(depot.join("extra.txt"), b"x").unwrap();
        let old = install_state::DepotInstall {
            depot_id: "481".into(),
            manifest_id: "100".into(),
            files: s(&["extra.txt"]),
        };
        let removed = finish_depot_install(
            &work,
            &depot,
            "481",
            "200",
            &[0u8; 32],
            Some(&old),
            &s(&["game.exe"]),
            false,
        );
        assert_eq!(removed, 0);
        assert!(depot.join("extra.txt").exists());
        assert_eq!(install_state::load(&work, "481").unwrap().manifest_id, "200");
        let _ = std::fs::remove_dir_all(&work);
    }
}
