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

    // NSIS installer, not the portable .exe.
    let mut installer_url = None;
    if let Some(assets) = release["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name.ends_with("-setup.exe") || name.contains("x64-setup") {
                installer_url = asset["browser_download_url"].as_str().map(String::from);
                break;
            }
        }
    }

    Ok(serde_json::json!({
        "available": true,
        "version": remote_version,
        "currentVersion": current_version,
        "date": release["published_at"].as_str(),
        "body": release["body"].as_str(),
        "installerUrl": installer_url,
        "releaseUrl": release["html_url"].as_str(),
        "installMethod": detect_install_method(),
    }))
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
