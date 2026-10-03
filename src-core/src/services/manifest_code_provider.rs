use std::time::Duration;

use reqwest::Client;

const PROVIDER_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub enum ManifestCodeSource {
    SteamRun,
    WuDrm,
}

impl ManifestCodeSource {
    pub fn label(self) -> &'static str {
        match self {
            ManifestCodeSource::SteamRun => "steam.run",
            ManifestCodeSource::WuDrm => "wudrm",
        }
    }
}

pub struct ManifestCodeResolved {
    pub request_code: u64,
    pub source: ManifestCodeSource,
}

pub async fn fetch_manifest_request_code_external(
    http: &Client,
    manifest_gid: u64,
) -> Result<ManifestCodeResolved, String> {
    let mut errors: Vec<String> = Vec::new();

    match fetch_steamrun(http, manifest_gid).await {
        Ok(code) => {
            return Ok(ManifestCodeResolved {
                request_code: code,
                source: ManifestCodeSource::SteamRun,
            });
        }
        Err(e) => errors.push(format!("steam.run: {}", e)),
    }

    match fetch_wudrm(http, manifest_gid).await {
        Ok(code) => {
            return Ok(ManifestCodeResolved {
                request_code: code,
                source: ManifestCodeSource::WuDrm,
            });
        }
        Err(e) => errors.push(format!("wudrm: {}", e)),
    }

    Err(format!(
        "All external manifest-code providers failed for gid {}: {}",
        manifest_gid,
        errors.join(" | ")
    ))
}

async fn fetch_steamrun(http: &Client, gid: u64) -> Result<u64, String> {
    let url = format!("https://manifest.steam.run/api/manifest/{}", gid);
    let resp = http
        .get(&url)
        .timeout(PROVIDER_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("body read failed: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("invalid JSON: {}", e))?;
    let content = json
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("response missing 'content' field")?;
    content
        .trim()
        .parse::<u64>()
        .map_err(|e| format!("parse failed: {}", e))
}

async fn fetch_wudrm(http: &Client, gid: u64) -> Result<u64, String> {
    let url = format!("http://gmrc.wudrm.com/manifest/{}", gid);
    let resp = http
        .get(&url)
        .timeout(PROVIDER_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("body read failed: {}", e))?;
    body.trim()
        .parse::<u64>()
        .map_err(|e| format!("parse failed: {}", e))
}
