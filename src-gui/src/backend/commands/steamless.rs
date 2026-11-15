use crate::backend::{parse, reply, Cx};
use std::path::{Path, PathBuf};

use smd_core::services::steamless::{self, DrmScanEntry, UnpackResult};

pub async fn steamless_scan(game_dir: String) -> Result<Vec<DrmScanEntry>, String> {
    let path = PathBuf::from(&game_dir);
    if !path.exists() {
        return Err(format!("Path not found: {}", game_dir));
    }
    Ok(steamless::scan_game_dir_for_drm(&path))
}

pub async fn steamless_unpack(app: &Cx, targets: Vec<String>) -> Result<Vec<UnpackResult>, String> {
    let state = app.state();
    let dir = app.data_dir();
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let target = Path::new(t);
        let r = steamless::unpack_with_steamless(&state.http_client, &dir, target).await?;
        results.push(r);
    }
    Ok(results)
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "steamless_scan" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { game_dir: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steamless_scan(a.game_dir).await)
        }
        "steamless_unpack" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steamless_unpack(app, a.targets).await)
        }
        _ => return None,
    })
}
