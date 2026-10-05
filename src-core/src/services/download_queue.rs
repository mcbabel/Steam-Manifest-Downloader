use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedDownload {
    pub id: String,
    pub app_id: String,
    #[serde(default)]
    pub game_name: Option<String>,
    #[serde(default)]
    pub header_image: Option<String>,
    #[serde(default)]
    pub depot_count: usize,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub depots: serde_json::Value,
    pub config: serde_json::Value,
    #[serde(default)]
    pub added_at: Option<String>,
}

fn queue_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("download_queue.json")
}

pub async fn load(app_data_dir: &Path) -> Vec<QueuedDownload> {
    tokio::fs::read(queue_path(app_data_dir))
        .await
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

async fn save(app_data_dir: &Path, queue: &[QueuedDownload]) -> Result<(), String> {
    let path = queue_path(app_data_dir);
    if queue.is_empty() {
        let _ = tokio::fs::remove_file(&path).await;
        return Ok(());
    }
    tokio::fs::create_dir_all(app_data_dir)
        .await
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;
    let json = serde_json::to_vec_pretty(queue).map_err(|e| e.to_string())?;
    tokio::fs::write(path, json)
        .await
        .map_err(|e| format!("Failed to save the download queue: {}", e))
}

pub async fn add(app_data_dir: &Path, mut item: QueuedDownload) -> Result<Vec<QueuedDownload>, String> {
    let mut queue = load(app_data_dir).await;
    if item.id.is_empty() {
        item.id = uuid::Uuid::new_v4().to_string();
    }
    queue.push(item);
    save(app_data_dir, &queue).await?;
    Ok(queue)
}

pub async fn remove(app_data_dir: &Path, id: &str) -> Result<Vec<QueuedDownload>, String> {
    let mut queue = load(app_data_dir).await;
    queue.retain(|q| q.id != id);
    save(app_data_dir, &queue).await?;
    Ok(queue)
}

pub async fn move_item(app_data_dir: &Path, id: &str, offset: i32) -> Result<Vec<QueuedDownload>, String> {
    let mut queue = load(app_data_dir).await;
    if let Some(pos) = queue.iter().position(|q| q.id == id) {
        let target = (pos as i64 + offset as i64).clamp(0, queue.len() as i64 - 1) as usize;
        let item = queue.remove(pos);
        queue.insert(target, item);
        save(app_data_dir, &queue).await?;
    }
    Ok(queue)
}

pub async fn take_next(app_data_dir: &Path) -> Result<Option<QueuedDownload>, String> {
    let mut queue = load(app_data_dir).await;
    if queue.is_empty() {
        return Ok(None);
    }
    let next = queue.remove(0);
    save(app_data_dir, &queue).await?;
    Ok(Some(next))
}

pub async fn clear(app_data_dir: &Path) -> Result<(), String> {
    save(app_data_dir, &[]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(app: &str) -> QueuedDownload {
        QueuedDownload {
            id: String::new(),
            app_id: app.to_string(),
            game_name: None,
            header_image: None,
            depot_count: 1,
            size_bytes: None,
            depots: serde_json::Value::Null,
            config: serde_json::json!({ "mainAppId": app }),
            added_at: None,
        }
    }

    #[tokio::test]
    async fn downloads_come_out_in_the_order_they_were_queued() {
        let dir = std::env::temp_dir().join(format!("smd-queue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        add(&dir, item("1")).await.unwrap();
        add(&dir, item("2")).await.unwrap();
        let queue = add(&dir, item("3")).await.unwrap();
        assert!(queue.iter().all(|q| !q.id.is_empty()));
        let queue = move_item(&dir, &queue[2].id, -1).await.unwrap();
        assert_eq!(queue.iter().map(|q| q.app_id.as_str()).collect::<Vec<_>>(), ["1", "3", "2"]);
        let queue = remove(&dir, &queue[0].id).await.unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(take_next(&dir).await.unwrap().map(|q| q.app_id), Some("3".into()));
        assert_eq!(take_next(&dir).await.unwrap().map(|q| q.app_id), Some("2".into()));
        assert_eq!(take_next(&dir).await.unwrap(), None);
        assert!(!dir.join("download_queue.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
