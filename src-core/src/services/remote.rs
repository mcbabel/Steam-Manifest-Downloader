use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use base64::Engine;
use dryoc::classic::crypto_sign::{
    crypto_sign_detached, crypto_sign_seed_keypair, crypto_sign_verify_detached,
};
use serde::{Deserialize, Serialize};

pub const REMOTE_URL: &str = "https://smd.mcbabel.de/remote/smd-remote.json";
pub const PUBLIC_KEY: &str = "VFVz1/acRhqCU7Cj00XWHQa7Di8xnoSsdNM5Ws50+WE=";
const CACHE_FILE: &str = "remote.json";
const CACHE_SIG: &str = "remote.json.sig";
const MAX_BYTES: usize = 64 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(8);
const CONNECTIVITY_URL: &str = "https://api.steampowered.com/ISteamWebAPIUtil/GetServerInfo/v1/";
const CONNECTIVITY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Notice {
    pub id: String,
    pub level: String,
    pub text: BTreeMap<String, String>,
    pub url: String,
    pub min_version: String,
    pub max_version: String,
    pub frontends: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct RemoteFile {
    pub version: u32,
    pub links: BTreeMap<String, String>,
    pub notices: Vec<Notice>,
    pub disabled_providers: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteView {
    pub source: String,
    pub links: BTreeMap<String, String>,
    pub notices: Vec<Notice>,
}

pub fn default_links() -> BTreeMap<String, String> {
    [
        ("website", "https://smd.mcbabel.de/"),
        ("docs_issues", "https://smd.mcbabel.de/documentation/issues"),
        ("issues", "https://github.com/MCbabel/Steam-Manifest-Downloader/issues"),
        ("releases", "https://github.com/MCbabel/Steam-Manifest-Downloader/releases"),
        ("achievement_watcher", "https://github.com/Shirowwww/Achievement-Watcher-Next"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

fn active() -> &'static RwLock<RemoteFile> {
    static ACTIVE: OnceLock<RwLock<RemoteFile>> = OnceLock::new();
    ACTIVE.get_or_init(|| RwLock::new(RemoteFile::default()))
}

pub fn provider_enabled(name: &str) -> bool {
    active()
        .read()
        .map(|r| !r.disabled_providers.iter().any(|p| p.eq_ignore_ascii_case(name)))
        .unwrap_or(true)
}

fn decode_b64(text: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD.decode(text.trim()).ok()
}

fn encode_b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn public_key() -> Result<[u8; 32], String> {
    decode_b64(PUBLIC_KEY)
        .and_then(|k| <[u8; 32]>::try_from(k.as_slice()).ok())
        .ok_or_else(|| "no valid public key".to_string())
}

pub fn verify_with(key: &[u8; 32], body: &[u8], sig_text: &str) -> Result<(), String> {
    let sig = decode_b64(sig_text)
        .and_then(|s| <[u8; 64]>::try_from(s.as_slice()).ok())
        .ok_or_else(|| "invalid signature".to_string())?;
    crypto_sign_verify_detached(&sig, body, key).map_err(|_| "invalid signature".to_string())
}

pub fn verify(body: &[u8], sig_text: &str) -> Result<RemoteFile, String> {
    if body.len() > MAX_BYTES {
        return Err("remote file too large".into());
    }
    verify_with(&public_key()?, body, sig_text)?;
    serde_json::from_slice(body).map_err(|e| format!("remote file: {}", e))
}

pub fn keygen() -> (String, String) {
    let seed: [u8; 32] = rand_seed();
    let (pk, _) = crypto_sign_seed_keypair(&seed);
    (encode_b64(&seed), encode_b64(&pk))
}

fn rand_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    dryoc::rng::copy_randombytes(&mut seed);
    seed
}

pub fn sign(secret_text: &str, body: &[u8]) -> Result<(String, String), String> {
    let seed = decode_b64(secret_text)
        .and_then(|s| <[u8; 32]>::try_from(s.as_slice()).ok())
        .ok_or_else(|| "not a secret key file".to_string())?;
    let (pk, sk) = crypto_sign_seed_keypair(&seed);
    let mut sig = [0u8; 64];
    crypto_sign_detached(&mut sig, body, &sk).map_err(|e| e.to_string())?;
    Ok((encode_b64(&sig), encode_b64(&pk)))
}

fn version_parts(v: &str) -> Vec<u64> {
    v.trim()
        .trim_start_matches('v')
        .split(['.', '-', '+'])
        .take(3)
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect()
}

fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (mut a, mut b) = (version_parts(a), version_parts(b));
    a.resize(3, 0);
    b.resize(3, 0);
    a.cmp(&b)
}

pub fn notices_for(file: &RemoteFile, app_version: &str, frontend: &str) -> Vec<Notice> {
    file.notices
        .iter()
        .filter(|n| !n.id.trim().is_empty() && !n.text.is_empty())
        .filter(|n| n.frontends.is_empty() || n.frontends.iter().any(|f| f.eq_ignore_ascii_case(frontend)))
        .filter(|n| n.min_version.is_empty() || version_cmp(app_version, &n.min_version).is_ge())
        .filter(|n| n.max_version.is_empty() || version_cmp(app_version, &n.max_version).is_le())
        .filter(|n| n.url.is_empty() || n.url.starts_with("https://"))
        .cloned()
        .collect()
}

fn view(file: &RemoteFile, source: &str, app_version: &str, frontend: &str) -> RemoteView {
    let mut links = default_links();
    for (k, v) in &file.links {
        if links.contains_key(k) && v.starts_with("https://") {
            links.insert(k.clone(), v.clone());
        }
    }
    RemoteView {
        source: source.to_string(),
        links,
        notices: notices_for(file, app_version, frontend),
    }
}

async fn fetch(http: &reqwest::Client) -> Result<(Vec<u8>, String), String> {
    let get = |url: String| async move {
        let resp = http
            .get(url)
            .timeout(FETCH_TIMEOUT)
            .send()
            .await
            .map_err(|e| e.without_url().to_string())?
            .error_for_status()
            .map_err(|e| e.without_url().to_string())?;
        if resp.content_length().is_some_and(|n| n as usize > MAX_BYTES) {
            return Err("remote file too large".to_string());
        }
        let bytes = resp.bytes().await.map_err(|e| e.without_url().to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("remote file too large".to_string());
        }
        Ok::<Vec<u8>, String>(bytes.to_vec())
    };
    let body = get(REMOTE_URL.to_string()).await?;
    let sig = get(format!("{}.sig", REMOTE_URL)).await?;
    Ok((body, String::from_utf8_lossy(&sig).into_owned()))
}

fn load_cache(app_data_dir: &Path) -> Option<RemoteFile> {
    let body = std::fs::read(app_data_dir.join(CACHE_FILE)).ok()?;
    let sig = std::fs::read_to_string(app_data_dir.join(CACHE_SIG)).ok()?;
    verify(&body, &sig).ok()
}

fn activate(file: &RemoteFile) {
    if let Ok(mut a) = active().write() {
        *a = file.clone();
    }
}

pub async fn refresh(
    http: &reqwest::Client,
    app_data_dir: &Path,
    enabled: bool,
    app_version: &str,
    frontend: &str,
) -> RemoteView {
    if !enabled {
        activate(&RemoteFile::default());
        return view(&RemoteFile::default(), "builtin", app_version, frontend);
    }
    if let Ok((body, sig)) = fetch(http).await {
        if let Ok(file) = verify(&body, &sig) {
            let _ = std::fs::create_dir_all(app_data_dir);
            let _ = std::fs::write(app_data_dir.join(CACHE_FILE), &body);
            let _ = std::fs::write(app_data_dir.join(CACHE_SIG), sig.trim());
            activate(&file);
            return view(&file, "website", app_version, frontend);
        }
        crate::dlog!("remote", "signature check failed, using the cache");
    }
    match load_cache(app_data_dir) {
        Some(file) => {
            activate(&file);
            view(&file, "cache", app_version, frontend)
        }
        None => view(&RemoteFile::default(), "builtin", app_version, frontend),
    }
}

pub async fn connectivity_check(http: &reqwest::Client) -> bool {
    http.get(CONNECTIVITY_URL)
        .timeout(CONNECTIVITY_TIMEOUT)
        .send()
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signed_file_verifies_and_a_changed_one_does_not() {
        let (secret, public) = keygen();
        let key: [u8; 32] = decode_b64(&public).unwrap().try_into().unwrap();
        let body = br#"{"version":1,"notices":[]}"#;
        let (sig, signer) = sign(&secret, body).unwrap();
        assert_eq!(signer, public);
        assert!(verify_with(&key, body, &sig).is_ok());
        assert!(verify_with(&key, br#"{"version":2,"notices":[]}"#, &sig).is_err());
        assert!(verify_with(&key, body, "nonsense").is_err());
    }

    #[test]
    fn the_shipped_file_matches_the_built_in_key() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/remote");
        let body = std::fs::read(root.join("smd-remote.json")).unwrap();
        let sig = std::fs::read_to_string(root.join("smd-remote.json.sig")).unwrap();
        let file = verify(&body, &sig).unwrap();
        assert!(file.version >= 1);
    }

    #[test]
    fn notices_follow_version_and_frontend() {
        let notice = |id: &str, min: &str, max: &str, fronts: &[&str]| Notice {
            id: id.into(),
            level: "info".into(),
            text: [("en".to_string(), "x".to_string())].into_iter().collect(),
            url: String::new(),
            min_version: min.into(),
            max_version: max.into(),
            frontends: fronts.iter().map(|s| s.to_string()).collect(),
        };
        let file = RemoteFile {
            version: 1,
            notices: vec![
                notice("all", "", "", &[]),
                notice("old", "", "1.4.9", &[]),
                notice("new", "1.5.0", "", &["tui"]),
            ],
            ..Default::default()
        };
        let ids = |v: Vec<Notice>| v.into_iter().map(|n| n.id).collect::<Vec<_>>();
        assert_eq!(ids(notices_for(&file, "1.5.1", "gui")), vec!["all"]);
        assert_eq!(ids(notices_for(&file, "1.5.1", "tui")), vec!["all", "new"]);
        assert_eq!(ids(notices_for(&file, "1.4.2", "gui")), vec!["all", "old"]);
    }

    #[test]
    fn only_known_https_links_replace_the_defaults() {
        let mut file = RemoteFile::default();
        file.links.insert("issues".into(), "https://example.org/issues".into());
        file.links.insert("releases".into(), "http://example.org".into());
        file.links.insert("other".into(), "https://example.org".into());
        let v = view(&file, "website", "1.5.0", "gui");
        assert_eq!(v.links["issues"], "https://example.org/issues");
        assert_eq!(v.links["releases"], default_links()["releases"]);
        assert!(!v.links.contains_key("other"));
    }
}
