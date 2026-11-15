use crate::backend::{parse, reply, Cx};
use std::path::{Path, PathBuf};

use smd_core::services::emulator::ScannedFile;
use smd_core::services::steam_api_bypass::{self, BypassApplyResult};

pub async fn steam_api_bypass_apply(app: &Cx, targets: Vec<ScannedFile>) -> Result<Vec<BypassApplyResult>, String> {
    let state = app.state();
    let dir = app.data_dir();
    let mut results = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = Path::new(&t.path);
        let x64 = t.arch == "x64";
        let r = steam_api_bypass::apply_to_target(&state.http_client, &dir, path, x64).await;
        results.push(r);
    }
    Ok(results)
}

pub async fn steam_api_bypass_revert(targets: Vec<String>) -> Result<Vec<bool>, String> {
    let mut out = Vec::with_capacity(targets.len());
    for t in &targets {
        let path = PathBuf::from(t);
        out.push(steam_api_bypass::revert_for_target(&path).is_ok());
    }
    Ok(out)
}

pub async fn steam_api_bypass_status(targets: Vec<String>) -> Result<bool, String> {
    for t in &targets {
        let path = PathBuf::from(t);
        if steam_api_bypass::is_installed_for_target(&path) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "steam_api_bypass_apply" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<ScannedFile>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steam_api_bypass_apply(app, a.targets).await)
        }
        "steam_api_bypass_revert" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steam_api_bypass_revert(a.targets).await)
        }
        "steam_api_bypass_status" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { targets: Vec<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(steam_api_bypass_status(a.targets).await)
        }
        _ => return None,
    })
}
