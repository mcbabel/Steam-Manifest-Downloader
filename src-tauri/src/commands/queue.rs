use smd_core::services::download_queue::{self, QueuedDownload};
use tauri::{command, AppHandle};

use super::app_data_dir;

#[command]
pub async fn queue_list(app: AppHandle) -> Vec<QueuedDownload> {
    download_queue::load(&app_data_dir(&app)).await
}

#[command]
pub async fn queue_add(app: AppHandle, item: QueuedDownload) -> Result<Vec<QueuedDownload>, String> {
    download_queue::add(&app_data_dir(&app), item).await
}

#[command]
pub async fn queue_remove(app: AppHandle, id: String) -> Result<Vec<QueuedDownload>, String> {
    download_queue::remove(&app_data_dir(&app), &id).await
}

#[command]
pub async fn queue_move(app: AppHandle, id: String, offset: i32) -> Result<Vec<QueuedDownload>, String> {
    download_queue::move_item(&app_data_dir(&app), &id, offset).await
}

#[command]
pub async fn queue_take_next(app: AppHandle) -> Result<Option<QueuedDownload>, String> {
    download_queue::take_next(&app_data_dir(&app)).await
}

#[command]
pub async fn queue_clear(app: AppHandle) -> Result<(), String> {
    download_queue::clear(&app_data_dir(&app)).await
}
