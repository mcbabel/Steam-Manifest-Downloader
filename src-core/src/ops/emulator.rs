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
) -> Result<Vec<ReplaceResult>, String> {
    let info = emulator::fetch_release_info(&state.http_client, app_data_dir).await?;
    crate::dlog!(
        "emu",
        "apply_replacement targets={} variant={:?} release={} cache_root={}",
        targets.len(),
        variant,
        info.tag,
        info.cache_root
    );

    let need_windows = targets.iter().any(|t| t.platform == Platform::Windows);
    let need_linux = targets.iter().any(|t| t.platform == Platform::Linux);
    let mut missing = Vec::new();
    if need_windows && !emulator::platform_cached(&info, Platform::Windows) {
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
        results.push(emulator::apply_replacement(
            path,
            &platform_cache,
            variant,
            t.platform,
            x64,
            &app_id,
            &installed_app_ids,
            settings_ref,
            dlcs.as_deref(),
        ));
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

pub fn read_emu_settings(target_path: &str) -> Result<EmuSettings, String> {
    emulator::read_emu_settings_for_target(Path::new(target_path))
}

pub fn write_emu_settings(target_path: &str, settings: &EmuSettings) -> Result<(), String> {
    emulator::write_emu_settings_for_target(Path::new(target_path), settings)
}

pub async fn revert_replacement(targets: Vec<String>) -> Result<Vec<ReplaceResult>, String> {
    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let path = PathBuf::from(&target);
        let mut r = ReplaceResult {
            path: target.clone(),
            backup_path: None,
            success: false,
            error: None,
            fail_class: None,
            dlc_count: None,
        };
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
