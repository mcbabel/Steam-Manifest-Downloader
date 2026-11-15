use crate::backend::{parse, reply, Cx};
use std::path::PathBuf;
use std::sync::Arc;


use smd_core::services::steam_downloader::{
    download_depot_from_local_manifest, download_depot_native, NativeDownloadProgress,
};


#[derive(Debug, serde::Deserialize)]
pub struct NativeDownloadRequest {
    #[serde(rename = "appId")]
    pub app_id: u32,
    #[serde(rename = "depotId")]
    pub depot_id: u32,
    #[serde(rename = "manifestId")]
    pub manifest_id: String,
    #[serde(rename = "depotKey")]
    pub depot_key_hex: String,
    #[serde(rename = "outputDir")]
    pub output_dir: String,
    #[serde(rename = "manifestPath", default)]
    pub manifest_path: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct NativeDownloadResult {
    pub success: bool,
    #[serde(rename = "filesWritten")]
    pub files_written: usize,
    #[serde(rename = "bytesWritten")]
    pub bytes_written: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn native_download_depot(app: &Cx, request: NativeDownloadRequest) -> Result<NativeDownloadResult, String> {
    let state = app.state();
    let manifest_id: u64 = request
        .manifest_id
        .parse()
        .map_err(|_| format!("invalid manifest id '{}'", request.manifest_id))?;
    let depot_key = hex::decode(request.depot_key_hex.trim())
        .map_err(|e| format!("invalid depot key hex: {}", e))?;
    if depot_key.len() != 32 {
        return Err(format!(
            "depot key must be 32 bytes (got {})",
            depot_key.len()
        ));
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&depot_key);

    let out_dir = PathBuf::from(&request.output_dir);
    tokio::fs::create_dir_all(&out_dir)
        .await
        .map_err(|e| format!("create output dir failed: {}", e))?;

    let session = state.steam_session.clone();
    let http = state.http_client.clone();
    let events = app.clone();

    let progress_cb = move |p: NativeDownloadProgress| {
        events.emit(
            "download-progress",
            serde_json::json!({
                "type": "output",
                "jobId": "native",
                "depotId": p.depot_id.to_string(),
                "output": format!("{:.2}% depots/{}/", p.percent, p.depot_id),
                "stream": "stdout",
                "completedBytes": p.completed_bytes,
                "totalBytes": p.total_bytes,
                "networkBytes": p.network_bytes,
                "skippedChunks": p.skipped_chunks,
                "skippedBytes": p.skipped_bytes,
                "completedChunks": p.completed_chunks,
                "totalChunks": p.total_chunks,
                "percent": p.percent,
            }),
        );
    };

    let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pause = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let chunk_concurrency = smd_core::services::settings::load_settings(&app.data_dir())
        .await
        .native_chunk_concurrency;
    let outcome = if let Some(path) = request.manifest_path.as_deref() {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| format!("read manifest file '{}': {}", path, e))?;
        download_depot_from_local_manifest(
            http,
            session,
            request.app_id,
            request.depot_id,
            key_arr,
            bytes,
            out_dir,
            cancel,
            pause,
            progress_cb,
            chunk_concurrency,
            Default::default(),
        )
        .await
    } else {
        download_depot_native(
            http,
            session,
            request.app_id,
            request.depot_id,
            manifest_id,
            key_arr,
            out_dir,
            cancel,
            pause,
            progress_cb,
            chunk_concurrency,
            Default::default(),
        )
        .await
    };

    match outcome {
        Ok(o) => Ok(NativeDownloadResult {
            success: true,
            files_written: o.files_written,
            bytes_written: o.bytes_written,
            error: None,
        }),
        Err(e) => Ok(NativeDownloadResult {
            success: false,
            files_written: 0,
            bytes_written: 0,
            error: Some(e),
        }),
    }
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "native_download_depot" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { request: NativeDownloadRequest, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(native_download_depot(app, a.request).await)
        }
        _ => return None,
    })
}
