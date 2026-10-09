const GITHUB_REPO: &str = "MCbabel/Steam-Manifest-Downloader";
const USER_AGENT: &str = "SteamManifestDownloader";

pub fn detect_install_method() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "self"
    }
    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("FLATPAK_ID").is_some() {
            return "flatpak";
        }
        match std::env::current_exe() {
            Ok(path) => {
                let s = path.to_string_lossy();
                if s.starts_with("/app/") {
                    "flatpak"
                } else if s.starts_with("/snap/") {
                    "snap"
                } else if s.starts_with("/usr/") || s.starts_with("/opt/") {
                    "system"
                } else {
                    "self"
                }
            }
            Err(_) => "self",
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        "self"
    }
}

pub async fn check_for_updates(current_version: &str) -> Result<serde_json::Value, String> {
    let client = crate::services::net::client();
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let response = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", crate::services::net::describe(&e)))?;

    if !response.status().is_success() {
        return Ok(serde_json::json!({
            "available": false,
            "error": format!("GitHub API returned {}", response.status()),
        }));
    }

    let release: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let tag = release["tag_name"].as_str().unwrap_or("");
    let remote_version = tag.trim_start_matches('v');

    if !is_newer_version(current_version, remote_version) {
        return Ok(serde_json::json!({ "available": false }));
    }

    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let package = crate::services::telemetry::package_kind();
    let (update_kind, asset) = pick_update_asset(package, &assets);

    Ok(serde_json::json!({
        "available": true,
        "version": remote_version,
        "currentVersion": current_version,
        "date": release["published_at"].as_str(),
        "body": release["body"].as_str(),
        "installerUrl": asset.and_then(|a| a["browser_download_url"].as_str()),
        "assetDigest": asset.and_then(|a| a["digest"].as_str()),
        "assetSize": asset.and_then(|a| a["size"].as_u64()),
        "updateKind": update_kind,
        "releaseUrl": release["html_url"].as_str(),
        "installMethod": detect_install_method(),
    }))
}

fn appimage_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "amd64",
    }
}

pub fn pick_update_asset<'a>(package: &str, assets: &'a [serde_json::Value]) -> (&'static str, Option<&'a serde_json::Value>) {
    let find = |pred: &dyn Fn(&str) -> bool| assets.iter().find(|a| pred(a["name"].as_str().unwrap_or("")));
    match package {
        "installer" | "exe" => match find(&|n| n.ends_with("-setup.exe")) {
            Some(a) => ("installer", Some(a)),
            None => ("page", None),
        },
        "appimage" => {
            let suffix = format!("_{}.AppImage", appimage_arch());
            match find(&|n: &str| n.ends_with(&suffix)) {
                Some(a) => ("appimage", Some(a)),
                None => ("page", None),
            }
        }
        _ => ("page", None),
    }
}

pub fn running_appimage() -> Option<std::path::PathBuf> {
    if !cfg!(target_os = "linux") {
        return None;
    }
    let path = std::path::PathBuf::from(std::env::var_os("APPIMAGE")?);
    path.is_file().then_some(path)
}

pub async fn install_appimage(
    url: &str,
    digest: Option<&str>,
    progress: impl Fn(u64, Option<u64>),
) -> Result<std::path::PathBuf, String> {
    use sha2::Digest;
    use tokio::io::AsyncWriteExt;

    let target = running_appimage().ok_or_else(|| "This build is not running as an AppImage".to_string())?;
    let dir = target
        .parent()
        .ok_or_else(|| "AppImage has no parent folder".to_string())?
        .to_path_buf();
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app.AppImage".into());
    let tmp = dir.join(format!(".{}.update", name));
    let mut file = tokio::fs::File::create(&tmp)
        .await
        .map_err(|e| format!("No write access next to the AppImage in {}: {}", dir.display(), e))?;

    let result: Result<(), String> = async {
        let mut resp = crate::services::net::client()
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Failed to download update: {}", crate::services::net::describe(&e)))?;
        if !resp.status().is_success() {
            return Err(format!("Download failed with status {}", resp.status()));
        }
        let total = resp.content_length();
        let mut hasher = sha2::Sha256::new();
        let mut done: u64 = 0;
        let mut head: Vec<u8> = Vec::new();
        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| format!("Failed to download update: {}", crate::services::net::describe(&e)))?
        {
            if head.len() < 4 {
                head.extend(chunk.iter().take(4 - head.len()));
            }
            hasher.update(&chunk);
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to save update: {}", e))?;
            done += chunk.len() as u64;
            progress(done, total);
        }
        file.flush().await.map_err(|e| format!("Failed to save update: {}", e))?;
        if head != b"\x7fELF" {
            return Err("The downloaded update is not an AppImage".into());
        }
        if let Some(expected) = digest.and_then(|d| d.strip_prefix("sha256:")) {
            let actual = hex::encode(hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected.trim()) {
                return Err("The downloaded update is damaged (checksum mismatch)".into());
            }
        }
        Ok(())
    }
    .await;
    drop(file);
    if let Err(e) = result {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&target).map(|m| m.permissions().mode()).unwrap_or(0o755) | 0o111;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode));
    }
    if let Err(e) = tokio::fs::rename(&tmp, &target).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(format!("Failed to replace the AppImage: {}", e));
    }
    Ok(target)
}

pub fn relaunch_appimage(path: &std::path::Path) -> Result<(), String> {
    let appdir = std::env::var("APPDIR").unwrap_or_default();
    let mut cmd = std::process::Command::new(path);
    for (key, value) in std::env::vars_os() {
        let k = key.to_string_lossy();
        if matches!(k.as_ref(), "APPIMAGE" | "APPDIR" | "ARGV0" | "OWD") {
            cmd.env_remove(&key);
            continue;
        }
        match clean_env_value(&value.to_string_lossy(), &appdir) {
            Some(Some(clean)) => {
                cmd.env(&key, clean);
            }
            Some(None) => {
                cmd.env_remove(&key);
            }
            None => {}
        }
    }
    if let Some(owd) = std::env::var_os("OWD") {
        cmd.current_dir(owd);
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to start the updated AppImage: {}", e))
}

fn clean_env_value(value: &str, appdir: &str) -> Option<Option<String>> {
    if appdir.is_empty() || !value.contains(appdir) {
        return None;
    }
    let kept: Vec<&str> = value
        .split(':')
        .filter(|part| !part.is_empty() && !part.starts_with(appdir))
        .collect();
    Some((!kept.is_empty()).then(|| kept.join(":")))
}

pub fn is_newer_version(current: &str, remote: &str) -> bool {
    let parse = |v: &str| -> (u64, u64, u64) {
        let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    let c = parse(current);
    let r = parse(remote);
    r > c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_appimage_paths_from_the_environment() {
        let appdir = "/tmp/.mount_SteamAb12";
        assert_eq!(clean_env_value("/usr/bin:/bin", appdir), None);
        assert_eq!(
            clean_env_value("/tmp/.mount_SteamAb12/usr/bin:/usr/bin:/bin", appdir),
            Some(Some("/usr/bin:/bin".to_string()))
        );
        assert_eq!(clean_env_value("/tmp/.mount_SteamAb12/usr/lib", appdir), Some(None));
        assert_eq!(clean_env_value("/usr/bin", ""), None);
    }

    #[test]
    fn picks_the_asset_for_the_package() {
        let assets = vec![
            serde_json::json!({ "name": "Steam.Manifest.Downloader_1.6.0_x64-setup.exe", "browser_download_url": "https://x/setup.exe" }),
            serde_json::json!({ "name": "Steam.Manifest.Downloader_1.6.0_amd64.AppImage", "browser_download_url": "https://x/app.AppImage", "digest": "sha256:ab" }),
            serde_json::json!({ "name": "Steam-Manifest-Downloader_1.6.0_windows-standalone.zip" }),
        ];
        let (kind, asset) = pick_update_asset("installer", &assets);
        assert_eq!((kind, asset.unwrap()["browser_download_url"].as_str()), ("installer", Some("https://x/setup.exe")));
        if appimage_arch() == "amd64" {
            let (kind, asset) = pick_update_asset("appimage", &assets);
            assert_eq!((kind, asset.unwrap()["digest"].as_str()), ("appimage", Some("sha256:ab")));
        }
        assert_eq!(pick_update_asset("portable", &assets).0, "page");
        assert_eq!(pick_update_asset("binary", &assets).0, "page");
        assert_eq!(pick_update_asset("installer", &[]).0, "page");
        assert!(is_newer_version("1.5.0", "1.6.0"));
        assert!(!is_newer_version("1.6.0", "1.5.9"));
    }
}
