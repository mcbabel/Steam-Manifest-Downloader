use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

const MAX_HISTORY_ENTRIES: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub app_id: String,
    pub game_name: Option<String>,
    pub header_image: Option<String>,
    pub depot_count: usize,
    pub depots_downloaded: usize,
    pub status: String,
    pub download_dir: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub source_repo: Option<String>,
    #[serde(default)]
    pub depot_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct History {
    pub entries: Vec<HistoryEntry>,
}

fn history_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("history.json")
}

pub async fn load_history(app_data_dir: &Path) -> History {
    let path = history_path(app_data_dir);
    match fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => History::default(),
    }
}

async fn save_history(app_data_dir: &Path, history: &History) -> Result<(), String> {
    let path = history_path(app_data_dir);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create history directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write history file: {}", e))?;

    Ok(())
}

pub fn supersedes_resumable(entry: &HistoryEntry, old: &HistoryEntry) -> bool {
    if old.status != "cancelled_resumable" {
        return false;
    }
    if entry.status != "complete" && entry.status != "partial" {
        return false;
    }
    old.app_id == entry.app_id && old.download_dir == entry.download_dir
}

pub async fn add_entry(app_data_dir: &Path, entry: HistoryEntry) -> Result<(), String> {
    let mut history = load_history(app_data_dir).await;
    history
        .entries
        .retain(|old| !supersedes_resumable(&entry, old));
    history.entries.insert(0, entry);
    if history.entries.len() > MAX_HISTORY_ENTRIES {
        history.entries.truncate(MAX_HISTORY_ENTRIES);
    }

    save_history(app_data_dir, &history).await
}

pub async fn remove_entry(app_data_dir: &Path, entry_id: &str) -> Result<(), String> {
    let mut history = load_history(app_data_dir).await;
    history.entries.retain(|e| e.id != entry_id);
    save_history(app_data_dir, &history).await
}

pub async fn clear(app_data_dir: &Path) -> Result<(), String> {
    let history = History::default();
    save_history(app_data_dir, &history).await
}

#[cfg(test)]
mod supersede_tests {
    use super::*;

    fn entry(status: &str, app: &str, dir: &str) -> HistoryEntry {
        HistoryEntry {
            id: format!("{}-{}-{}", status, app, dir),
            app_id: app.to_string(),
            game_name: None,
            header_image: None,
            depot_count: 1,
            depots_downloaded: 1,
            status: status.to_string(),
            download_dir: dir.to_string(),
            started_at: String::new(),
            completed_at: None,
            source_repo: None,
            depot_ids: vec![],
            resume_payload: None,
        }
    }

    #[test]
    fn finishing_a_download_clears_its_stale_resumable_entry() {
        let done = entry("complete", "730", "/games/730");
        let stale = entry("cancelled_resumable", "730", "/games/730");
        assert!(supersedes_resumable(&done, &stale));
    }

    #[test]
    fn a_partial_run_also_supersedes_it() {
        let partial = entry("partial", "730", "/games/730");
        let stale = entry("cancelled_resumable", "730", "/games/730");
        assert!(supersedes_resumable(&partial, &stale));
    }

    #[test]
    fn a_resumable_entry_for_another_game_survives() {
        let done = entry("complete", "730", "/games/730");
        let other = entry("cancelled_resumable", "440", "/games/440");
        assert!(!supersedes_resumable(&done, &other));
    }

    #[test]
    fn the_same_game_in_a_different_folder_survives() {
        let done = entry("complete", "730", "/games/730");
        let elsewhere = entry("cancelled_resumable", "730", "/other/730");
        assert!(!supersedes_resumable(&done, &elsewhere));
    }

    #[test]
    fn a_failed_run_does_not_discard_a_resumable_entry() {
        let failed = entry("failed", "730", "/games/730");
        let stale = entry("cancelled_resumable", "730", "/games/730");
        assert!(!supersedes_resumable(&failed, &stale));
    }

    #[test]
    fn completed_entries_are_never_discarded() {
        let done = entry("complete", "730", "/games/730");
        let older_done = entry("complete", "730", "/games/730");
        assert!(!supersedes_resumable(&done, &older_done));
    }
}
