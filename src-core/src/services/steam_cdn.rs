use std::time::Duration;

use reqwest::Client;

use crate::services::steam_manifest::{decode_manifest, DecodedManifest};

const CDN_HTTP_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn fetch_decoded_manifest(
    client: &Client,
    host: &str,
    depot_id: u32,
    manifest_id: u64,
    request_code: u64,
    auth_token: Option<&str>,
    depot_key: &[u8; 32],
    use_https: bool,
) -> Result<DecodedManifest, String> {
    let scheme = if use_https { "https" } else { "http" };
    let mut url = format!(
        "{}://{}/depot/{}/manifest/{}/5/{}",
        scheme, host, depot_id, manifest_id, request_code
    );
    if let Some(token) = auth_token {
        if !token.is_empty() {
            url.push_str(token);
        }
    }

    let bytes = client
        .get(&url)
        .timeout(CDN_HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("manifest CDN request failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("manifest CDN returned HTTP error: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("manifest CDN body read failed: {}", e))?;

    decode_manifest(&bytes, depot_key)
}

pub async fn fetch_encrypted_chunk(
    client: &Client,
    host: &str,
    depot_id: u32,
    chunk_sha1_hex: &str,
    auth_token: Option<&str>,
    use_https: bool,
) -> Result<Vec<u8>, String> {
    let scheme = if use_https { "https" } else { "http" };
    let mut url = format!("{}://{}/depot/{}/chunk/{}", scheme, host, depot_id, chunk_sha1_hex);
    if let Some(token) = auth_token {
        if !token.is_empty() {
            url.push_str(token);
        }
    }
    let bytes = client
        .get(&url)
        .timeout(CDN_HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("chunk CDN request failed: {}", e))?
        .error_for_status()
        .map_err(|e| format!("chunk CDN returned HTTP error: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("chunk CDN body read failed: {}", e))?;
    Ok(bytes.to_vec())
}
