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

pub async fn save(app_data_dir: &Path, followup: &PendingFollowup) -> Result<(), String> {
    tokio::fs::create_dir_all(app_data_dir)
        .await
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    let json = serde_json::to_vec_pretty(followup).map_err(|e| e.to_string())?;
    tokio::fs::write(followup_path(app_data_dir), json)
        .await
        .map_err(|e| format!("Failed to save the pending steps: {}", e))
}

pub async fn load(app_data_dir: &Path) -> Option<PendingFollowup> {
    let path = followup_path(app_data_dir);
    let bytes = tokio::fs::read(&path).await.ok()?;
    let followup: Option<PendingFollowup> = serde_json::from_slice(&bytes).ok();
    match followup {
        Some(f) if Path::new(&f.download_dir).is_dir() => Some(f),
        _ => {
            let _ = tokio::fs::remove_file(&path).await;
            None
        }
    }
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
}
