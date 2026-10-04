use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingFollowup {
    pub app_id: String,
    #[serde(default)]
    pub game_name: Option<String>,
    #[serde(default)]
    pub header_image: Option<String>,
    pub download_dir: String,
    #[serde(default)]
    pub created_at: Option<String>,
}

fn followup_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("pending_followup.json")
}

async fn read_all(app_data_dir: &Path) -> Vec<PendingFollowup> {
    let Ok(bytes) = tokio::fs::read(followup_path(app_data_dir)).await else {
        return Vec::new();
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(serde_json::Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect(),
        Ok(value) => serde_json::from_value(value).ok().into_iter().collect(),
        Err(_) => Vec::new(),
    }
}

async fn write_all(app_data_dir: &Path, list: &[PendingFollowup]) -> Result<(), String> {
    let path = followup_path(app_data_dir);
    if list.is_empty() {
        let _ = tokio::fs::remove_file(&path).await;
        return Ok(());
    }
    tokio::fs::create_dir_all(app_data_dir)
        .await
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    let json = serde_json::to_vec_pretty(list).map_err(|e| e.to_string())?;
    tokio::fs::write(path, json)
        .await
        .map_err(|e| format!("Failed to save the pending steps: {}", e))
}

pub async fn save(app_data_dir: &Path, followup: &PendingFollowup) -> Result<(), String> {
    let mut list = read_all(app_data_dir).await;
    list.retain(|f| f.download_dir != followup.download_dir);
    list.push(followup.clone());
    write_all(app_data_dir, &list).await
}

pub async fn load(app_data_dir: &Path) -> Option<PendingFollowup> {
    let list = read_all(app_data_dir).await;
    let kept: Vec<PendingFollowup> = list
        .iter()
        .filter(|f| Path::new(&f.download_dir).is_dir())
        .cloned()
        .collect();
    if kept.len() != list.len() {
        let _ = write_all(app_data_dir, &kept).await;
    }
    kept.into_iter().next()
}

pub async fn remove(app_data_dir: &Path, download_dir: &str) {
    let mut list = read_all(app_data_dir).await;
    list.retain(|f| f.download_dir != download_dir);
    let _ = write_all(app_data_dir, &list).await;
}

pub async fn clear(app_data_dir: &Path) {
    let _ = tokio::fs::remove_file(followup_path(app_data_dir)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("smd-followup-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn a_saved_followup_is_loaded_until_cleared() {
        let data = temp_dir("roundtrip");
        let game = data.join("220 - Half-Life 2");
        std::fs::create_dir_all(&game).unwrap();
        let f = PendingFollowup {
            app_id: "220".into(),
            game_name: Some("Half-Life 2".into()),
            header_image: None,
            download_dir: game.to_string_lossy().into_owned(),
            created_at: None,
        };
        save(&data, &f).await.unwrap();
        assert_eq!(load(&data).await, Some(f));
        clear(&data).await;
        assert_eq!(load(&data).await, None);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    async fn a_followup_for_a_missing_folder_is_dropped() {
        let data = temp_dir("missing");
        let f = PendingFollowup {
            app_id: "220".into(),
            game_name: None,
            header_image: None,
            download_dir: data.join("gone").to_string_lossy().into_owned(),
            created_at: None,
        };
        save(&data, &f).await.unwrap();
        assert_eq!(load(&data).await, None);
        assert!(!data.join("pending_followup.json").exists());
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    async fn several_followups_are_offered_one_after_another() {
        let data = temp_dir("list");
        let mut dirs = Vec::new();
        for name in ["a", "b"] {
            let game = data.join(name);
            std::fs::create_dir_all(&game).unwrap();
            let dir = game.to_string_lossy().into_owned();
            save(
                &data,
                &PendingFollowup {
                    app_id: name.into(),
                    game_name: None,
                    header_image: None,
                    download_dir: dir.clone(),
                    created_at: None,
                },
            )
            .await
            .unwrap();
            dirs.push(dir);
        }
        assert_eq!(load(&data).await.map(|f| f.app_id), Some("a".into()));
        remove(&data, &dirs[0]).await;
        assert_eq!(load(&data).await.map(|f| f.app_id), Some("b".into()));
        remove(&data, &dirs[1]).await;
        assert_eq!(load(&data).await, None);
        let _ = std::fs::remove_dir_all(&data);
    }

    #[tokio::test]
    async fn the_old_single_note_format_is_still_read() {
        let data = temp_dir("legacy");
        let game = data.join("g");
        std::fs::create_dir_all(&game).unwrap();
        let note = serde_json::json!({ "app_id": "1", "download_dir": game.to_string_lossy() });
        std::fs::write(data.join("pending_followup.json"), note.to_string()).unwrap();
        assert_eq!(load(&data).await.map(|f| f.app_id), Some("1".into()));
        let _ = std::fs::remove_dir_all(&data);
    }
}
