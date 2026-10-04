use crate::services::history::{self as history_service, HistoryEntry};
use std::path::{Path, PathBuf};

pub async fn remove_entry(
    app_data_dir: &Path,
    entry_id: &str,
    delete_files: bool,
) -> Result<(), String> {
    let mut depot_dirs: Vec<PathBuf> = Vec::new();
    if delete_files {
        let history = history_service::load_history(app_data_dir).await;
        if let Some(entry) = history.entries.iter().find(|e| e.id == entry_id) {
            if entry.status == "cancelled_resumable" && entry.resume_payload.is_some() {
                depot_dirs = collect_resumable_depot_dirs(entry);
            }
        }
    }

    history_service::remove_entry(app_data_dir, entry_id).await?;

    let failures = delete_dirs(&depot_dirs).await;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Entry removed, but {} depot folder(s) could not be deleted: {}",
            failures.len(),
            failures.join("; ")
        ))
    }
}

pub async fn clear(app_data_dir: &Path, delete_resumable_files: bool) -> Result<(), String> {
    let mut depot_dirs: Vec<PathBuf> = Vec::new();
    if delete_resumable_files {
        let history = history_service::load_history(app_data_dir).await;
        for entry in history.entries.iter() {
            if entry.status == "cancelled_resumable" && entry.resume_payload.is_some() {
                depot_dirs.extend(collect_resumable_depot_dirs(entry));
            }
        }
    }

    history_service::clear(app_data_dir).await?;

    let failures = delete_dirs(&depot_dirs).await;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "History cleared, but {} depot folder(s) could not be deleted: {}",
            failures.len(),
            failures.join("; ")
        ))
    }
}

fn collect_resumable_depot_dirs(entry: &HistoryEntry) -> Vec<PathBuf> {
    if entry.download_dir.trim().is_empty() {
        return Vec::new();
    }
    let work_dir = PathBuf::from(&entry.download_dir);
    if !is_safe_workdir(&work_dir) {
        return Vec::new();
    }
    let depots_root = work_dir.join("depots");
    if !depots_root.is_dir() {
        return Vec::new();
    }

    let mut wanted_ids: Vec<String> = entry.depot_ids.clone();
    if let Some(payload) = entry.resume_payload.as_ref() {
        if let Some(arr) = payload.get("selectedDepots").and_then(|v| v.as_array()) {
            for d in arr {
                if let Some(id) = d.get("depotId").and_then(|v| v.as_str()) {
                    if !wanted_ids.iter().any(|x| x == id) {
                        wanted_ids.push(id.to_string());
                    }
                }
            }
        }
    }

    let mut out: Vec<PathBuf> = Vec::new();
    let read = match std::fs::read_dir(&depots_root) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    for ent in read.flatten() {
        let path = ent.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            eprintln!("[History] Skipping symlink in depots dir: {:?}", path);
            continue;
        }
        if !meta.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let matches = wanted_ids.iter().any(|id| {
            name == *id
                || name.ends_with(&format!(" - {}", id))
                || name.ends_with(&format!("-{}", id))
        });
        if !matches {
            continue;
        }
        if !path_is_within(&path, &depots_root) {
            continue;
        }
        out.push(path);
    }
    out
}

async fn delete_dirs(dirs: &[PathBuf]) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if let Err(e) = tokio::fs::remove_dir_all(&dir).await {
            eprintln!("[History] Failed to delete {:?}: {}", dir, e);
            failures.push(format!("{:?}: {}", dir, e));
        } else {
            eprintln!("[History] Deleted depot folder: {:?}", dir);
        }
    }
    failures
}

fn is_safe_workdir(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let s = path.to_string_lossy();
    if s.trim().is_empty() || s.len() < 8 {
        return false;
    }
    path.components().count() >= 4
}

fn path_is_within(child: &Path, parent: &Path) -> bool {
    let c = match child.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let p = match parent.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    c.starts_with(&p) && c != p
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateCheck {
    pub entry_id: String,
    pub app_id: String,
    pub update_available: bool,
    pub depots: Vec<DepotUpdate>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DepotUpdate {
    pub depot_id: String,
    pub installed: String,
    pub latest: String,
}

pub fn compare_installed(
    entry: &HistoryEntry,
    installed: &[crate::services::install_state::DepotInstall],
    latest: &std::collections::HashMap<String, String>,
) -> Option<UpdateCheck> {
    let depots: Vec<DepotUpdate> = installed
        .iter()
        .filter_map(|d| {
            latest.get(&d.depot_id).map(|gid| DepotUpdate {
                depot_id: d.depot_id.clone(),
                installed: d.manifest_id.clone(),
                latest: gid.clone(),
            })
        })
        .collect();
    if depots.is_empty() {
        return None;
    }
    Some(UpdateCheck {
        entry_id: entry.id.clone(),
        app_id: entry.app_id.clone(),
        update_available: depots.iter().any(|d| d.installed != d.latest),
        depots,
    })
}

pub async fn check_updates(
    app_data_dir: &Path,
    session: std::sync::Arc<crate::services::steam_session::SteamSession>,
) -> Result<Vec<UpdateCheck>, String> {
    let history = history_service::load_history(app_data_dir).await;
    let candidates: Vec<(HistoryEntry, Vec<crate::services::install_state::DepotInstall>)> = history
        .entries
        .into_iter()
        .filter(|e| e.status == "complete" && !e.download_dir.is_empty())
        .filter_map(|e| {
            let installed = crate::services::install_state::installed(Path::new(&e.download_dir));
            (!installed.is_empty()).then_some((e, installed))
        })
        .collect();
    let mut app_ids: Vec<u32> = candidates
        .iter()
        .filter_map(|(e, _)| e.app_id.parse().ok())
        .collect();
    app_ids.sort_unstable();
    app_ids.dedup();
    if app_ids.is_empty() {
        return Ok(Vec::new());
    }
    let latest = crate::services::steam_pics::fetch_public_manifests(session, &app_ids).await?;
    Ok(candidates
        .iter()
        .filter_map(|(entry, installed)| {
            let app: u32 = entry.app_id.parse().ok()?;
            compare_installed(entry, installed, latest.get(&app)?)
        })
        .collect())
}

#[cfg(test)]
mod update_check_tests {
    use super::*;
    use crate::services::install_state::DepotInstall;

    fn entry() -> HistoryEntry {
        serde_json::from_value(serde_json::json!({
            "id": "e", "app_id": "70", "game_name": null, "header_image": null,
            "depot_count": 2, "depots_downloaded": 2, "status": "complete",
            "download_dir": "/x", "started_at": "", "completed_at": null, "source_repo": null
        }))
        .unwrap()
    }

    fn installed(id: &str, manifest: &str) -> DepotInstall {
        DepotInstall {
            depot_id: id.to_string(),
            manifest_id: manifest.to_string(),
            files: vec![],
        }
    }

    #[test]
    fn flags_a_depot_with_a_newer_public_manifest() {
        let latest = [("71".to_string(), "2".to_string()), ("72".to_string(), "5".to_string())]
            .into_iter()
            .collect();
        let up_to_date = compare_installed(&entry(), &[installed("71", "2"), installed("72", "5")], &latest).unwrap();
        assert!(!up_to_date.update_available);
        let outdated = compare_installed(&entry(), &[installed("71", "1"), installed("72", "5")], &latest).unwrap();
        assert!(outdated.update_available);
    }

    #[test]
    fn unknown_depots_give_no_answer() {
        let latest = std::collections::HashMap::new();
        assert!(compare_installed(&entry(), &[installed("71", "1")], &latest).is_none());
    }
}
