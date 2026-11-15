use crate::backend::{parse, plain, reply, Cx};
use smd_core::services::settings as settings_service;


const USER_AGENT: &str = "SteamManifestDownloader";

pub async fn check_for_updates(app: &Cx) -> Result<serde_json::Value, String> {
    smd_core::ops::updater::check_for_updates(app.version()).await
}

pub async fn get_whats_new(app: &Cx, first_start: bool) -> Option<serde_json::Value> {
    let dir = app.data_dir();
    smd_core::ops::updater::whats_new(&dir, app.version(), first_start).await
}

pub async fn install_update(app: &Cx, installer_url: String, update_kind: Option<String>, asset_digest: Option<String>) -> Result<(), String> {
    if update_kind.as_deref() == Some("appimage") {
        let progress = app.clone();
        let path = smd_core::ops::updater::install_appimage(&installer_url, asset_digest.as_deref(), move |done, total| {
            progress.emit("update-progress", serde_json::json!({ "done": done, "total": total }));
        })
        .await?;
        smd_core::ops::updater::relaunch_appimage(&path)?;
        app.request_exit();
        return Ok(());
    }
    if !cfg!(target_os = "windows") {
        return Err("Updates for this build are installed from the release page".into());
    }

    let client = smd_core::services::net::client();

    let temp_dir = std::env::temp_dir().join("SteamManifestDownloader");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let installer_path = temp_dir.join("update-installer.exe");

    let response = client
        .get(&installer_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", smd_core::services::net::describe(&e)))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status {}", response.status()));
    }

    let bytes = response.bytes().await
        .map_err(|e| format!("Failed to read download: {}", e))?;

    std::fs::write(&installer_path, &bytes)
        .map_err(|e| format!("Failed to save installer: {}", e))?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(&installer_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| format!("Failed to launch installer: {}", e))?;
    }

    app.request_exit();
    Ok(())
}

pub async fn get_auto_update_enabled(app: &Cx) -> Result<bool, String> {
    let settings = settings_service::load_settings(&app.data_dir()).await;
    Ok(settings.auto_update)
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "check_for_updates" => reply(check_for_updates(app).await),
        "get_whats_new" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { first_start: bool, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            plain(get_whats_new(app, a.first_start).await)
        }
        "install_update" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { installer_url: String, update_kind: Option<String>, asset_digest: Option<String>, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(install_update(app, a.installer_url, a.update_kind, a.asset_digest).await)
        }
        "get_auto_update_enabled" => reply(get_auto_update_enabled(app).await),
        _ => return None,
    })
}
