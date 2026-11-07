use std::path::{Path, PathBuf};

use crate::services::emulator::{
    self, EmuSettings, Platform, ReleaseInfo, ReplaceResult, ScannedFile, Variant,
};
use crate::services::events::{Sink, EMU_DOWNLOAD_PROGRESS};
use crate::services::AppState;

pub const DOWNLOAD_CONFIRM_PREFIX: &str = "EMU_DOWNLOAD_CONFIRM_REQUIRED:";

pub async fn ensure_cached(
    state: &AppState,
    app_data_dir: &Path,
    platform: Platform,
) -> Result<ReleaseInfo, String> {
    let info = emulator::fetch_release_info(&state.http_client, app_data_dir).await?;
    emulator::ensure_cached(&state.http_client, &info, platform, |_, _| {}).await?;
    let refreshed = emulator::fetch_release_info(&state.http_client, app_data_dir).await?;
    Ok(refreshed)
}

pub async fn scan_game_dir(game_dir: &str) -> Result<Vec<ScannedFile>, String> {
    let path = PathBuf::from(game_dir);
    if !path.exists() {
        return Err(format!("Path not found: {}", game_dir));
    }
    Ok(emulator::scan_game_dir(&path))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MergeCandidate {
    #[serde(rename = "depotId")]
    pub depot_id: String,
    pub path: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DlcMergePlan {
    #[serde(rename = "mainDepotDir")]
    pub main_depot_dir: String,
    #[serde(rename = "mainDepotId")]
    pub main_depot_id: String,
    #[serde(rename = "mainLabel", skip_serializing_if = "Option::is_none")]
    pub main_label: Option<String>,
    #[serde(rename = "toMerge")]
    pub to_merge: Vec<MergeCandidate>,
    pub skipped: Vec<MergeCandidate>,
    #[serde(rename = "dlcDepotDirs")]
    pub dlc_depot_dirs: Vec<String>,
}

pub async fn scan_for_dlc_merge(
    state: &AppState,
    game_dir: String,
    app_id: Option<String>,
) -> Result<Option<DlcMergePlan>, String> {
    let work = PathBuf::from(&game_dir);
    let depots_root = work.join("depots");
    if !depots_root.exists() {
        return Ok(None);
    }
    let mut entries = tokio::fs::read_dir(&depots_root)
        .await
        .map_err(|e| format!("read depots dir: {}", e))?;
    let mut depot_dirs: Vec<PathBuf> = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("walk depots dir: {}", e))?
    {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.') {
                continue;
            }
        }
        depot_dirs.push(p);
    }
    if depot_dirs.len() < 2 {
        return Ok(None);
    }

    let mut candidates: Vec<(PathBuf, crate::services::emulator::Platform)> = Vec::new();
    for d in &depot_dirs {
        if let Some(first) = emulator::scan_game_dir(d).into_iter().next() {
            candidates.push((d.clone(), first.platform));
        }
    }
    if candidates.is_empty() {
        return Ok(None);
    }
    let host_os = std::env::consts::OS;
    let host_preferred = candidates
        .iter()
        .find(|(_, p)| match (p, host_os) {
            (crate::services::emulator::Platform::Windows, "windows") => true,
            (crate::services::emulator::Platform::Linux, "linux") => true,
            _ => false,
        })
        .cloned();
    let (main_dir, main_platform) = host_preferred.unwrap_or_else(|| candidates[0].clone());
    let main_os = match main_platform {
        crate::services::emulator::Platform::Windows => "windows",
        crate::services::emulator::Platform::Linux => "linux",
    };

    let pics_map = if let Some(app_str) = app_id.as_deref() {
        if let Ok(app_id_u) = app_str.parse::<u32>() {
            match crate::services::steam_pics::fetch_depots_with_names(
                state.steam_session.clone(),
                app_id_u,
            )
            .await
            {
                Ok(list) => list
                    .into_iter()
                    .map(|d| (d.depot_id.clone(), d))
                    .collect::<std::collections::HashMap<_, _>>(),
                Err(e) => {
                    eprintln!("[merge-scan] PICS lookup failed: {}", e);
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        }
    } else {
        std::collections::HashMap::new()
    };

    let main_depot_id = extract_depot_id_from_folder(&main_dir).unwrap_or_default();
    let main_label = pics_map.get(&main_depot_id).and_then(|d| d.name.clone());

    let mut to_merge: Vec<MergeCandidate> = Vec::new();
    let mut skipped: Vec<MergeCandidate> = Vec::new();
    for d in &depot_dirs {
        if d == &main_dir {
            continue;
        }
        let depot_id = extract_depot_id_from_folder(d).unwrap_or_default();
        let info = pics_map.get(&depot_id);
        let path_str = d.to_string_lossy().to_string();
        let label = info.and_then(|m| m.name.clone());
        let has_own_exe = depot_dir_has_executable(d);

        if let Some(meta) = info {
            let role_str = match meta.role {
                crate::services::steam_pics::DepotRole::Platform => "platform",
                crate::services::steam_pics::DepotRole::SharedContent => "shared_content",
                crate::services::steam_pics::DepotRole::Dlc => "dlc",
                crate::services::steam_pics::DepotRole::Language => "language",
                crate::services::steam_pics::DepotRole::Redistributable => "redistributable",
                crate::services::steam_pics::DepotRole::Other => "other",
            }
            .to_string();

            let same_platform_as_main = meta
                .oslist
                .as_deref()
                .map(|os| os.to_ascii_lowercase().contains(main_os))
                .unwrap_or(false);

            let should_merge = if has_own_exe {
                false
            } else {
                match meta.role {
                    crate::services::steam_pics::DepotRole::SharedContent => true,
                    crate::services::steam_pics::DepotRole::Dlc => meta
                        .oslist
                        .as_deref()
                        .map(|os| same_platform(os, main_os))
                        .unwrap_or(true),
                    crate::services::steam_pics::DepotRole::Language => true,
                    crate::services::steam_pics::DepotRole::Platform => same_platform_as_main,
                    crate::services::steam_pics::DepotRole::Redistributable => false,
                    crate::services::steam_pics::DepotRole::Other => true,
                }
            };

            let cand = MergeCandidate {
                depot_id: depot_id.clone(),
                path: path_str,
                role: role_str,
                label,
            };
            if should_merge {
                to_merge.push(cand);
            } else {
                skipped.push(cand);
            }
        } else if has_own_exe {
            skipped.push(MergeCandidate {
                depot_id: depot_id.clone(),
                path: path_str,
                role: "standalone".to_string(),
                label,
            });
        } else {
            to_merge.push(MergeCandidate {
                depot_id: depot_id.clone(),
                path: path_str,
                role: "unknown".to_string(),
                label,
            });
        }
    }

    if to_merge.is_empty() && skipped.is_empty() {
        return Ok(None);
    }

    let dlc_depot_dirs: Vec<String> = to_merge.iter().map(|c| c.path.clone()).collect();

    Ok(Some(DlcMergePlan {
        main_depot_dir: main_dir.to_string_lossy().to_string(),
        main_depot_id,
        main_label,
        to_merge,
        skipped,
        dlc_depot_dirs,
    }))
}

fn depot_dir_has_executable(dir: &Path) -> bool {
    depot_dir_has_executable_recursive(dir, 0)
}

fn depot_dir_has_executable_recursive(dir: &Path, depth: usize) -> bool {
    if depth > 5 {
        return false;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return false,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            if depot_dir_has_executable_recursive(&path, depth + 1) {
                return true;
            }
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            if lower.ends_with(".exe") {
                return true;
            }
            if lower.ends_with(".dll")
                || lower.ends_with(".so")
                || lower.ends_with(".dylib")
                || lower.ends_with(".pak")
                || lower.ends_with(".dat")
                || lower.ends_with(".txt")
                || lower.ends_with(".json")
                || lower.ends_with(".ini")
                || lower.ends_with(".vdf")
                || lower.ends_with(".manifest")
                || lower.ends_with(".png")
                || lower.ends_with(".jpg")
                || lower.ends_with(".pdb")
                || lower.ends_with(".lua")
            {
                continue;
            }
            if is_elf_executable_sync(&path) {
                return true;
            }
        }
    }
    false
}

fn is_elf_executable_sync(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 20];
    if file.read_exact(&mut header).is_err() {
        return false;
    }
    if &header[..4] != b"\x7fELF" {
        return false;
    }
    let e_type = u16::from_le_bytes([header[16], header[17]]);
    e_type == 2 || e_type == 3
}

fn extract_depot_id_from_folder(p: &Path) -> Option<String> {
    let name = p.file_name()?.to_str()?;
    let last_token = name.rsplit(" - ").next()?.trim();
    if last_token.chars().all(|c| c.is_ascii_digit()) && !last_token.is_empty() {
        Some(last_token.to_string())
    } else if name.chars().all(|c| c.is_ascii_digit()) {
        Some(name.to_string())
    } else {
        None
    }
}

fn same_platform(depot_os: &str, main_os: &str) -> bool {
    if main_os.is_empty() {
        return true;
    }
    depot_os.to_ascii_lowercase().contains(main_os)
}

pub async fn merge_dlc_depots(
    main_depot_dir: String,
    dlc_depot_dirs: Vec<String>,
) -> Result<u64, String> {
    let main = PathBuf::from(&main_depot_dir);
    if !main.is_dir() {
        return Err(format!(
            "Main depot dir not a directory: {}",
            main_depot_dir
        ));
    }
    if !is_safe_depot_path(&main) {
        return Err(format!(
            "Refusing unsafe main depot path: {}",
            main_depot_dir
        ));
    }
    let mut moved: u64 = 0;
    for src in dlc_depot_dirs {
        let src_path = PathBuf::from(&src);
        if !src_path.is_dir() {
            continue;
        }
        if !is_safe_depot_path(&src_path) {
            eprintln!("[Merge] Refusing unsafe DLC depot path: {:?}", src_path);
            continue;
        }
        moved += merge_dir_into(&src_path, &main).await?;
        if let Err(e) = tokio::fs::remove_dir_all(&src_path).await {
            eprintln!(
                "[Merge] Failed to remove drained DLC dir {:?}: {}",
                src_path, e
            );
        }
    }
    Ok(moved)
}

fn is_safe_depot_path(path: &Path) -> bool {
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
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .map(|seg| seg == "depots")
            .unwrap_or(false)
    })
}

fn merge_dir_into<'a>(
    src: &'a Path,
    dst: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64, String>> + Send + 'a>> {
    Box::pin(async move {
        tokio::fs::create_dir_all(dst)
            .await
            .map_err(|e| format!("create dst {}: {}", dst.display(), e))?;
        let mut files_moved: u64 = 0;
        let mut entries = tokio::fs::read_dir(src)
            .await
            .map_err(|e| format!("read {}: {}", src.display(), e))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("walk {}: {}", src.display(), e))?
        {
            let path = entry.path();
            let name = entry.file_name();
            let target = dst.join(&name);
            if path.is_dir() {
                files_moved += merge_dir_into(&path, &target).await?;
            } else {
                if target.exists() {
                    continue;
                }
                if let Err(e) = tokio::fs::rename(&path, &target).await {
                    tokio::fs::copy(&path, &target).await.map_err(|copy_err| {
                        format!(
                            "rename + copy failed ({} / {}): {} / {}",
                            path.display(),
                            target.display(),
                            e,
                            copy_err
                        )
                    })?;
                    if let Err(rm_err) = tokio::fs::remove_file(&path).await {
                        eprintln!(
                            "[Merge] Copied {} -> {} but failed to remove source: {}",
                            path.display(),
                            target.display(),
                            rm_err
                        );
                    }
                }
                files_moved += 1;
            }
        }
        Ok(files_moved)
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_replacement(
    sink: &Sink,
    state: &AppState,
    app_data_dir: &Path,
    targets: Vec<ScannedFile>,
    variant: Variant,
    app_id: String,
    installed_app_ids: Vec<String>,
    emu_settings: Option<EmuSettings>,
    allow_download: bool,
    prefer_local: bool,
    coldclient_exe: Option<String>,
) -> Result<Vec<ReplaceResult>, String> {
    let mut info = emulator::fetch_release_info(&state.http_client, app_data_dir).await?;
    if prefer_local {
        info = emulator::local_info(app_data_dir, &info).ok_or("No gbe_fork release on this PC yet")?;
    }
    let cold = coldclient_exe.filter(|e| !e.trim().is_empty());
    crate::dlog!(
        "emu",
        "apply_replacement targets={} variant={:?} release={} cache_root={}",
        targets.len(),
        variant,
        info.tag,
        info.cache_root
    );

    let need_windows = cold.is_some() || targets.iter().any(|t| t.platform == Platform::Windows);
    let need_linux = targets.iter().any(|t| t.platform == Platform::Linux);
    let cold_missing = cold.is_some()
        && emulator::platform_cached(&info, Platform::Windows)
        && !emulator::tools_present(&PathBuf::from(&info.cache_root).join(Platform::Windows.cache_subdir()), Platform::Windows)
        && !info.bundled;
    let mut missing = Vec::new();
    if need_windows && (!emulator::platform_cached(&info, Platform::Windows) || cold_missing) {
        missing.push(("windows", info.windows_size));
    }
    if need_linux && !emulator::platform_cached(&info, Platform::Linux) {
        missing.push(("linux", info.linux_size));
    }
    if !missing.is_empty() && !allow_download && !info.bundled {
        let platforms: Vec<serde_json::Value> = missing
            .iter()
            .map(|(platform, size)| serde_json::json!({ "platform": platform, "size": size }))
            .collect();
        crate::dlog!(
            "emu",
            "apply_replacement needs download consent for {}",
            platforms
                .iter()
                .map(|p| p.get("platform").and_then(|v| v.as_str()).unwrap_or("?"))
                .collect::<Vec<_>>()
                .join(",")
        );
        return Err(format!(
            "{}{}",
            DOWNLOAD_CONFIRM_PREFIX,
            serde_json::json!({ "tag": info.tag, "platforms": platforms })
        ));
    }
    if cold_missing {
        emulator::forget_extracted(&info, Platform::Windows);
    }
    if need_windows {
        let sink = sink.clone();
        emulator::ensure_cached(
            &state.http_client,
            &info,
            Platform::Windows,
            move |downloaded, total| {
                sink.emit(
                    EMU_DOWNLOAD_PROGRESS,
                    serde_json::json!({
                        "platform": "windows",
                        "downloaded": downloaded,
                        "total": total,
                    }),
                );
            },
        )
        .await?;
    }
    if need_linux {
        let sink = sink.clone();
        emulator::ensure_cached(
            &state.http_client,
            &info,
            Platform::Linux,
            move |downloaded, total| {
                sink.emit(
                    EMU_DOWNLOAD_PROGRESS,
                    serde_json::json!({
                        "platform": "linux",
                        "downloaded": downloaded,
                        "total": total,
                    }),
                );
            },
        )
        .await?;
    }
    let cache_root = PathBuf::from(&info.cache_root);

    let settings_ref = emu_settings.as_ref();
    let dlcs = resolve_dlcs(state, &targets, &app_id).await;
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = Path::new(&t.path);
        let platform_cache = cache_root.join(t.platform.cache_subdir());
        let x64 = t.arch == "x64";
        let r = emulator::apply_replacement(
            path,
            &platform_cache,
            variant,
            t.platform,
            x64,
            &app_id,
            &installed_app_ids,
            settings_ref,
            dlcs.as_deref(),
        );
        if r.success {
            emulator::write_patch_info(path, &info.tag, variant);
        }
        results.push(r);
    }
    if let Some(exe) = cold {
        let windows_cache = cache_root.join(Platform::Windows.cache_subdir());
        let r = install_coldclient(&windows_cache, &exe, &app_id, &installed_app_ids, settings_ref, dlcs.as_deref(), &info.tag).await;
        results.push(r);
    }
    let succeeded = results.iter().filter(|r| r.success).count();
    crate::dlog!(
        "emu",
        "apply_replacement done: {}/{} succeeded",
        succeeded,
        results.len()
    );
    Ok(results)
}

async fn resolve_dlcs(
    state: &AppState,
    targets: &[ScannedFile],
    app_id: &str,
) -> Option<Vec<(String, String)>> {
    let root = targets
        .iter()
        .find_map(|t| crate::services::install_state::find_game_root(Path::new(&t.path)))?;
    let app_id_u: u32 = app_id.trim().parse().ok()?;
    let ids = match crate::services::install_state::load_dlc_choice(&root) {
        Some(ids) => ids,
        None => {
            let installed: Vec<String> = crate::services::install_state::installed(&root)
                .into_iter()
                .map(|d| d.depot_id)
                .collect();
            let meta = crate::services::steam_pics::fetch_depots_with_names(state.steam_session.clone(), app_id_u)
                .await
                .ok()?;
            let ids = crate::services::depot_select::chosen_dlcs(&meta, app_id.trim(), &[], &installed, Some(false));
            if ids.is_empty() {
                return None;
            }
            ids
        }
    };
    let numeric: Vec<u32> = ids.iter().filter_map(|id| id.parse().ok()).collect();
    let info = crate::services::steam_pics::fetch_dlc_info(state.steam_session.clone(), app_id_u, &numeric)
        .await
        .unwrap_or_default();
    Some(dlc_entries(&ids, &info))
}

fn dlc_entries(ids: &[String], info: &crate::services::steam_pics::DlcInfo) -> Vec<(String, String)> {
    ids.iter()
        .filter_map(|id| {
            let n: u32 = id.parse().ok()?;
            if !info.listed.is_empty() && !info.listed.contains(&n) {
                return None;
            }
            let name = info.names.get(&n).cloned().unwrap_or_else(|| format!("DLC {}", n));
            Some((id.clone(), name))
        })
        .collect()
}

pub fn patch_info(target_path: &str) -> emulator::PatchInfo {
    emulator::read_patch_info(Path::new(target_path))
}

pub async fn drm_backups(game_dir: String) -> Vec<crate::services::steamless::DrmBackup> {
    tokio::task::spawn_blocking(move || crate::services::steamless::drm_backups(Path::new(&game_dir)))
        .await
        .unwrap_or_default()
}

pub fn restore_drm(exe: &str) -> Result<(), String> {
    crate::services::steamless::restore_drm(Path::new(exe))
}

pub fn read_emu_settings(target_path: &str) -> Result<EmuSettings, String> {
    emulator::read_emu_settings_for_target(Path::new(target_path))
}

pub fn write_emu_settings(target_path: &str, settings: &EmuSettings) -> Result<(), String> {
    emulator::write_emu_settings_for_target(Path::new(target_path), settings)
}

pub async fn generate_game_data(
    state: &AppState,
    app_data_dir: &Path,
    targets: Vec<String>,
    app_id: String,
    language: Option<String>,
) -> Result<crate::services::game_data::GameDataResult, String> {
    let settings = crate::services::settings::load_settings(app_data_dir).await;
    let language = language
        .filter(|l| !l.trim().is_empty())
        .or_else(|| Some(settings.game_language.clone()).filter(|l| !l.trim().is_empty()))
        .unwrap_or_else(|| settings.language.clone());
    let language = crate::services::depot_select::steam_language(&language);
    let key = Some(settings.steam_web_api_key.trim().to_string()).filter(|k| !k.is_empty());
    let opts = crate::services::game_data::Options {
        language,
        web_api_key: key.as_deref(),
        media: crate::services::game_data::MediaLevel::parse(&settings.game_data_media),
    };
    crate::services::game_data::generate(
        &state.http_client,
        state.steam_session.clone(),
        &targets,
        &app_id,
        opts,
    )
    .await
}

pub async fn controller_info(
    state: &AppState,
    app_id: String,
) -> Result<crate::services::game_data::controller::ControllerInfo, String> {
    let app_num: u32 = app_id
        .trim()
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id.trim()))?;
    crate::services::game_data::controller::info(state.steam_session.clone(), app_num).await
}

pub async fn write_controller(
    state: &AppState,
    app_data_dir: &Path,
    targets: Vec<String>,
    app_id: String,
    layout_file: Option<String>,
) -> Result<crate::services::game_data::controller::ControllerResult, String> {
    crate::services::game_data::controller::write(
        &state.http_client,
        state.steam_session.clone(),
        &targets,
        &app_id,
        layout_file.as_deref(),
        &emulator::cache_root(app_data_dir),
    )
    .await
}

async fn install_coldclient(
    windows_cache: &Path,
    exe: &str,
    app_id: &str,
    installed_app_ids: &[String],
    settings: Option<&EmuSettings>,
    dlcs: Option<&[(String, String)]>,
    tag: &str,
) -> ReplaceResult {
    use crate::services::coldclient;
    let mut r = ReplaceResult::new(exe);
    let Some(src) = coldclient::loader_source(windows_cache) else {
        r.fail_class = Some("coldclient_missing".into());
        r.error = Some("This gbe_fork release has no ColdClientLoader files".into());
        return r;
    };
    let installed = match coldclient::install(&src, Path::new(exe), app_id) {
        Ok(i) => i,
        Err(e) => {
            r.fail_class = Some(emulator::fail::COPY_FAILED.into());
            r.error = Some(e);
            return r;
        }
    };
    r.path = installed.loader.clone();
    let loader = Path::new(&installed.loader);
    let settings_dir = loader.parent().map(|p| p.join("steam_settings")).unwrap_or_default();
    match emulator::write_settings_dir(&settings_dir, app_id, installed_app_ids, settings, dlcs) {
        Ok(n) => r.dlc_count = n,
        Err((class, e)) => {
            r.fail_class = Some(class.into());
            r.error = Some(e);
            return r;
        }
    }
    emulator::write_patch_info(loader, tag, Variant::Experimental);
    r.success = true;
    r.shortcut = retarget_steam_entry(exe, &installed.loader, false).await;
    let (from, to) = (exe.to_string(), installed.loader.clone());
    r.links = blocking(move || coldclient::retarget_links(&from, &to)).await.unwrap_or(0);
    r
}

async fn retarget_steam_entry(from: &str, to: &str, close_steam: bool) -> Option<String> {
    use crate::services::steam_library;
    if !steam_library::has_shortcut_for(from) {
        return None;
    }
    let path = steam_library::shortcuts_path()?;
    let running = steam_library::is_steam_running();
    if running && !close_steam {
        return Some("steam_running".into());
    }
    let steam_dir = steam_library::detect_steam().ok().map(|s| PathBuf::from(s.steam_dir));
    if running {
        let dir = steam_dir.clone()?;
        if steam_library::shutdown_steam(&dir).await.is_err() {
            return Some("steam_running".into());
        }
    }
    let state = match steam_library::retarget_shortcuts_in(&path, from, to) {
        Ok(n) if n > 0 => "updated",
        Ok(_) => "none",
        Err(_) => "failed",
    };
    if running {
        if let Some(dir) = steam_dir {
            let _ = steam_library::start_steam(&dir);
        }
    }
    Some(state.into())
}

pub async fn coldclient_shortcut(game_dir: String, back: bool, close_steam: bool) -> Vec<String> {
    let installs = blocking(move || crate::services::coldclient::installs_in(Path::new(&game_dir)))
        .await
        .unwrap_or_default();
    let mut states = Vec::new();
    for i in installs {
        let (from, to) = if back { (&i.loader, &i.exe) } else { (&i.exe, &i.loader) };
        states.push(retarget_steam_entry(from, to, close_steam).await.unwrap_or_else(|| "none".into()));
    }
    states
}

pub async fn revert_replacement(targets: Vec<String>, close_steam: bool) -> Result<Vec<ReplaceResult>, String> {
    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let path = PathBuf::from(&target);
        let mut r = ReplaceResult::new(&target);
        if let Some(cold) = crate::services::coldclient::install_for_loader(&path) {
            let state = retarget_steam_entry(&cold.loader, &cold.exe, close_steam).await;
            if state.as_deref() == Some("steam_running") {
                r.fail_class = Some("steam_running".into());
                r.error = Some("Steam is running".into());
                results.push(r);
                continue;
            }
            r.shortcut = state;
            let (from, to) = (cold.loader.clone(), cold.exe.clone());
            r.links = blocking(move || crate::services::coldclient::retarget_links(&from, &to)).await.unwrap_or(0);
            match crate::services::coldclient::remove(&path) {
                Ok(_) => r.success = true,
                Err(e) => {
                    r.fail_class = Some(emulator::fail::RESTORE_FAILED.into());
                    r.error = Some(e);
                }
            }
            results.push(r);
            continue;
        }
        match emulator::revert_replacement(&path) {
            Ok(()) => r.success = true,
            Err((class, e)) => {
                crate::dlog!("emu", "revert FAILED class={} reason={}", class, e);
                r.fail_class = Some(class.to_string());
                r.error = Some(e);
            }
        }
        results.push(r);
    }
    let succeeded = results.iter().filter(|r| r.success).count();
    crate::dlog!(
        "emu",
        "revert done: {}/{} succeeded",
        succeeded,
        results.len()
    );
    Ok(results)
}

#[cfg(test)]
mod dlc_tests {
    use super::*;

    #[test]
    fn lists_only_real_dlcs_with_their_names() {
        let info = crate::services::steam_pics::DlcInfo {
            listed: vec![300, 400],
            names: [(300, "Soundtrack".to_string())].into_iter().collect(),
        };
        let ids = vec!["228980".to_string(), "300".to_string(), "400".to_string()];
        assert_eq!(
            dlc_entries(&ids, &info),
            vec![
                ("300".to_string(), "Soundtrack".to_string()),
                ("400".to_string(), "DLC 400".to_string())
            ]
        );
        let ini = emulator::render_app_ini(&dlc_entries(&ids, &info));
        assert!(ini.contains("[app::dlcs]\nunlock_all=0\n300=Soundtrack\n400=DLC 400\n"));
    }
}

pub async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())
}

pub async fn lobby_connect(
    sink: &Sink,
    state: &AppState,
    app_data_dir: &Path,
    target: String,
    allow_download: bool,
) -> Result<String, String> {
    let windows = target.to_ascii_lowercase().ends_with(".dll");
    let platform = if windows { Platform::Windows } else { Platform::Linux };
    let info = emulator::fetch_release_info(&state.http_client, app_data_dir).await?;
    let cache_dir = PathBuf::from(&info.cache_root).join(platform.cache_subdir());
    let pruned = emulator::platform_cached(&info, platform)
        && crate::services::emu_extras::lobby_tool(&cache_dir, windows).is_none()
        && !info.bundled;
    if pruned && allow_download {
        emulator::forget_extracted(&info, platform);
    }
    if (!emulator::platform_cached(&info, platform) || pruned) && !info.bundled {
        if !allow_download {
            let size = if windows { info.windows_size } else { info.linux_size };
            let name = if windows { "windows" } else { "linux" };
            return Err(format!(
                "{}{}",
                DOWNLOAD_CONFIRM_PREFIX,
                serde_json::json!({ "tag": info.tag, "platforms": [{ "platform": name, "size": size }] })
            ));
        }
        let sink = sink.clone();
        let name = if windows { "windows" } else { "linux" };
        emulator::ensure_cached(&state.http_client, &info, platform, move |downloaded, total| {
            sink.emit(
                EMU_DOWNLOAD_PROGRESS,
                serde_json::json!({ "platform": name, "downloaded": downloaded, "total": total }),
            );
        })
        .await?;
    }
    let cache = PathBuf::from(&info.cache_root).join(platform.cache_subdir());
    blocking(move || {
        let tool = crate::services::emu_extras::lobby_tool(&cache, windows).ok_or("GBE_TOOLS_MISSING")?;
        crate::services::emu_extras::place_lobby_tool(&tool, Path::new(&target)).map(|p| p.to_string_lossy().to_string())
    })
    .await?
}
