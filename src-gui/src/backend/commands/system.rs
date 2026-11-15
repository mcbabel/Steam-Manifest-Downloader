use crate::backend::{parse, plain, reply, Cx};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
pub fn get_debug_log_path() -> Option<String> {
    smd_core::services::debug_log::log_path().map(|p| p.to_string_lossy().to_string())
}

pub async fn get_build_info(app: &Cx) -> serde_json::Value {
    let settings = Some(smd_core::services::settings::load_settings(&app.data_dir()).await);
    let diagnostic_id = settings
        .as_ref()
        .filter(|s| s.telemetry_consent == smd_core::services::settings::TelemetryConsent::Accepted)
        .map(|s| smd_core::services::telemetry::diagnostic_id(&s.installation_id))
        .filter(|id| !id.is_empty());
    let engine = settings.as_ref().map(|s| if s.use_native_downloader { "native" } else { "ddm" });
    let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local");
    let git_sha = option_env!("SMD_GIT_SHA").unwrap_or("unknown");
    let build_date = option_env!("SMD_BUILD_DATE").unwrap_or("unknown");

    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };

    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "channel": channel,
        "gitSha": git_sha,
        "buildDate": build_date,
        "profile": profile,
        "targetOs": std::env::consts::OS,
        "targetArch": std::env::consts::ARCH,
        "package": smd_core::services::telemetry::package_kind(),
        "engine": engine,
        "ddmAvailable": smd_core::services::embedded_tools::AVAILABLE,
        "diagnosticId": diagnostic_id,
    })
}

pub async fn power_off_system(app: &Cx) -> Result<(), String> {
    app.end_session("shutdown", 3).await;
    tokio::task::spawn_blocking(smd_core::ops::system::power_off)
        .await
        .map_err(|e| e.to_string())?
}

pub fn check_free_space(download_dir: Option<String>, needed: u64) -> serde_json::Value {
    let base = smd_core::ops::download::base_download_dir(download_dir.as_deref());
    serde_json::to_value(smd_core::services::disk_space::check(&base, needed))
        .unwrap_or(serde_json::Value::Null)
}

pub async fn check_dotnet() -> Result<serde_json::Value, String> {
    serde_json::to_value(smd_core::ops::system::check_dotnet()).map_err(|e| e.to_string())
}

pub async fn get_disk_space(path: String) -> Result<serde_json::Value, String> {
    #[cfg(target_os = "windows")]
    {
        if path.len() < 2 {
            return Err("Invalid path".to_string());
        }

        let drive_letter = path.chars().next().ok_or("Empty path")?;

        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
                "-NoProfile",
                "-Command",
                &format!(
                    "$d = Get-PSDrive {}; @{{ Free = $d.Free; Used = $d.Used }} | ConvertTo-Json",
                    drive_letter
                ),
            ]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let data: serde_json::Value = serde_json::from_str(stdout.trim())
                    .unwrap_or_else(|_| serde_json::json!({}));

                let free = data["Free"].as_u64().unwrap_or(0);
                let used = data["Used"].as_u64().unwrap_or(0);
                let total = free + used;
                let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
                let free_gb = (free_gb * 100.0).round() / 100.0;

                Ok(serde_json::json!({
                    "free": free,
                    "total": total,
                    "freeGB": free_gb,
                    "drive": format!("{}:", drive_letter),
                    "path": path,
                }))
            }
            Err(e) => Err(format!("Failed to check disk space: {}", e)),
        }
    }

    #[cfg(unix)]
    {
        use std::ffi::CString;

        let c_path = CString::new(path.clone())
            .map_err(|_| "Invalid path".to_string())?;

        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            let result = libc::statvfs(c_path.as_ptr(), &mut stat);
            if result != 0 {
                return Err("Failed to get filesystem stats".to_string());
            }

            let free = (stat.f_bavail as u64) * (stat.f_frsize as u64);
            let total = (stat.f_blocks as u64) * (stat.f_frsize as u64);
            let free_gb = (free as f64) / (1024.0 * 1024.0 * 1024.0);
            let free_gb = (free_gb * 100.0).round() / 100.0;

            Ok(serde_json::json!({
                "free": free,
                "total": total,
                "freeGB": free_gb,
                "drive": path.clone(),
                "path": path,
            }))
        }
    }
}

pub async fn remote_refresh(app: &Cx) -> Result<smd_core::services::remote::RemoteView, String> {
    let state = app.state();
    let dir = app.data_dir();
    Ok(smd_core::ops::system::remote_refresh(&state, &dir, env!("CARGO_PKG_VERSION"), "gui").await)
}

pub async fn connectivity_check(app: &Cx) -> Result<bool, String> {
    let state = app.state();
    Ok(smd_core::ops::system::connectivity_check(&state).await)
}

pub(crate) async fn dispatch(app: &Cx, name: &str, args: serde_json::Value) -> Option<Result<serde_json::Value, String>> {
    Some(match name {
        "get_debug_log_path" => plain(get_debug_log_path()),
        "get_build_info" => plain(get_build_info(app).await),
        "power_off_system" => reply(power_off_system(app).await),
        "check_free_space" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { download_dir: Option<String>, needed: u64, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            plain(check_free_space(a.download_dir, a.needed))
        }
        "check_dotnet" => reply(check_dotnet().await),
        "get_disk_space" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { path: String, }
            let a: Args = match parse(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            reply(get_disk_space(a.path).await)
        }
        "remote_refresh" => reply(remote_refresh(app).await),
        "connectivity_check" => reply(connectivity_check(app).await),
        _ => return None,
    })
}
