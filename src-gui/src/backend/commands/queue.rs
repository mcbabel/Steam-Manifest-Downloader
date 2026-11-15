use crate::backend::{parse, plain, reply, Cx};
use smd_core::services::download_queue::{self, QueuedDownload};


pub async fn queue_list(app: &Cx) -> Vec<QueuedDownload> {
    download_queue::load(&app.data_dir()).await
}

pub async fn queue_add(app: &Cx, item: QueuedDownload) -> Result<Vec<QueuedDownload>, String> {
    download_queue::add(&app.data_dir(), item).await
}

pub async fn queue_remove(app: &Cx, id: String) -> Result<Vec<QueuedDownload>, String> {
    download_queue::remove(&app.data_dir(), &id).await
}

pub async fn queue_move(app: &Cx, id: String, offset: i32) -> Result<Vec<QueuedDownload>, String> {
    download_queue::move_item(&app.data_dir(), &id, offset).await
}

pub async fn queue_take_next(app: &Cx) -> Result<Option<QueuedDownload>, String> {
    download_queue::take_next(&app.data_dir()).await
}

pub async fn queue_clear(app: &Cx) -> Result<(), String> {
    download_queue::clear(&app.data_dir()).await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "queue_list" => plain(queue_list(app).await),
        "queue_add" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { item: QueuedDownload, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(queue_add(app, a.item).await)
        }
        "queue_remove" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(queue_remove(app, a.id).await)
        }
        "queue_move" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { id: String, offset: i32, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(queue_move(app, a.id, a.offset).await)
        }
        "queue_take_next" => reply(queue_take_next(app).await),
        "queue_clear" => reply(queue_clear(app).await),
        _ => return None,
    })
}
