use crate::backend::{parse, reply, Cx};
use smd_core::ops::download::{self, DownloadConfig};


pub async fn start_download(app: &Cx, config: DownloadConfig) -> Result<serde_json::Value, String> {
    let state = app.state();
    download::start_download(app.sink(), &state, &app.data_dir(), config).await
}

pub async fn cancel_download(app: &Cx, job_id: String) -> Result<(), String> {
    let state = app.state();
    download::cancel_download(&app.sink(), &state, &app.data_dir(), job_id).await
}

pub async fn pause_download(app: &Cx, job_id: String, paused: bool) -> Result<(), String> {
    let state = app.state();
    download::pause_download(&app.sink(), &state, job_id, paused).await
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "start_download" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { config: DownloadConfig, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(start_download(app, a.config).await)
        }
        "cancel_download" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { job_id: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(cancel_download(app, a.job_id).await)
        }
        "pause_download" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { job_id: String, paused: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(pause_download(app, a.job_id, a.paused).await)
        }
        _ => return None,
    })
}
