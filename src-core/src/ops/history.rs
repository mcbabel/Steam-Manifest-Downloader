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
