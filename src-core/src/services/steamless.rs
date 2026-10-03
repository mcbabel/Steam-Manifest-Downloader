use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use walkdir::WalkDir;

const STEAMLESS_VERSION: &str = "v3.1.0.5";
const STEAMLESS_ZIP_URL: &str =
    "https://github.com/atom0s/Steamless/releases/download/v3.1.0.5/Steamless.v3.1.0.5.-.by.atom0s.zip";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrmScanEntry {
    pub path: String,
    pub has_drm: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnpackResult {
    pub path: String,
    pub success: bool,
    pub unpacked_path: Option<String>,
    pub backup_path: Option<String>,
    pub error: Option<String>,
}

pub fn has_steamstub_drm(exe_path: &Path) -> Result<bool, String> {
    let bytes = std::fs::read(exe_path)
        .map_err(|e| format!("read {}: {}", exe_path.display(), e))?;
    if bytes.len() < 0x40 {
        return Ok(false);
    }
    if bytes[0] != b'M' || bytes[1] != b'Z' {
        return Ok(false);
    }
    let e_lfanew = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;
    if bytes.len() < e_lfanew + 24 {
        return Ok(false);
    }
    if &bytes[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return Ok(false);
    }
    let coff = e_lfanew + 4;
    let num_sections =
        u16::from_le_bytes([bytes[coff + 2], bytes[coff + 3]]) as usize;
    let size_opt =
        u16::from_le_bytes([bytes[coff + 16], bytes[coff + 17]]) as usize;
    let sections_start = coff + 20 + size_opt;
    for i in 0..num_sections {
        let so = sections_start + i * 40;
        if so + 8 > bytes.len() {
            break;
        }
        let mut name = [0u8; 8];
        name.copy_from_slice(&bytes[so..so + 8]);
        let trimmed = trim_nul(&name);
        if trimmed == b".bind" {
            return Ok(true);
        }
    }
    Ok(false)
}

fn trim_nul(s: &[u8]) -> &[u8] {
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    &s[..end]
}

pub fn scan_game_dir_for_drm(game_dir: &Path) -> Vec<DrmScanEntry> {
    let mut out = Vec::new();
    if !game_dir.exists() {
        return out;
    }
    for entry in WalkDir::new(game_dir).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if !name.ends_with(".exe") {
            continue;
        }
        let path = entry.path().to_path_buf();
        let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let has_drm = has_steamstub_drm(&path).unwrap_or(false);
        if !has_drm {
            continue;
        }
        out.push(DrmScanEntry {
            path: path.to_string_lossy().to_string(),
            has_drm,
            size_bytes,
        });
    }
    out
}

fn cache_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("steamless_cache").join(STEAMLESS_VERSION)
}

pub async fn ensure_steamless_cached(
    client: &Client,
    app_data_dir: &Path,
) -> Result<PathBuf, String> {
    let root = cache_root(app_data_dir);
    let cli = root.join("Steamless.CLI.exe");
    if cli.exists() {
        return Ok(cli);
    }
    tokio::fs::create_dir_all(&root)
        .await
        .map_err(|e| format!("create steamless cache: {}", e))?;

    let resp = client
        .get(STEAMLESS_ZIP_URL)
        .header("User-Agent", "SteamManifestDownloader")
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("steamless download failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("steamless download HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("steamless read body: {}", e))?;

    let reader = std::io::Cursor::new(bytes.to_vec());
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| format!("steamless zip invalid: {}", e))?;
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| format!("zip entry {}: {}", i, e))?;
        let raw_name = match file.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let out_path = root.join(&raw_name);
        if file.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("mkdir {}: {}", out_path.display(), e))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
        }
        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| format!("create {}: {}", out_path.display(), e))?;
        std::io::copy(&mut file, &mut out_file)
            .map_err(|e| format!("write {}: {}", out_path.display(), e))?;
    }

    if !cli.exists() {
        return Err("Steamless.CLI.exe missing from zip".into());
    }
    Ok(cli)
}

#[cfg(target_os = "windows")]
fn build_steamless_command(cli: &Path, target: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new(cli);
    cmd.arg(target);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn build_steamless_command(cli: &Path, target: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new("mono");
    cmd.arg(cli).arg(target);
    if let Some(parent) = cli.parent() {
        let plugins = parent.join("Plugins");
        cmd.env("MONO_PATH", plugins);
    }
    cmd
}

pub async fn unpack_with_steamless(
    client: &Client,
    app_data_dir: &Path,
    target: &Path,
) -> Result<UnpackResult, String> {
    let mut result = UnpackResult {
        path: target.to_string_lossy().to_string(),
        success: false,
        unpacked_path: None,
        backup_path: None,
        error: None,
    };

    let cli = match ensure_steamless_cached(client, app_data_dir).await {
        Ok(p) => p,
        Err(e) => {
            result.error = Some(e);
            return Ok(result);
        }
    };

    let mut cmd = build_steamless_command(&cli, target);
    let workdir = cli.parent().unwrap_or(Path::new("."));
    cmd.current_dir(workdir);

    let output = match tokio::task::spawn_blocking(move || cmd.output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            result.error = Some(format!("spawn steamless: {}. On Linux/macOS install `mono`.", e));
            return Ok(result);
        }
        Err(e) => {
            result.error = Some(format!("join error: {}", e));
            return Ok(result);
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        result.error = Some(format!(
            "steamless exit {}: {} {}",
            output.status, stdout, stderr
        ));
        return Ok(result);
    }

    let mut unpacked = target.as_os_str().to_owned();
    unpacked.push(".unpacked.exe");
    let unpacked_path = PathBuf::from(unpacked);
    if !unpacked_path.exists() {
        result.error = Some(format!(
            "steamless completed but {} not found",
            unpacked_path.display()
        ));
        return Ok(result);
    }

    let backup = backup_path_for(target);
    if !backup.exists() {
        if let Err(e) = std::fs::copy(target, &backup) {
            result.error = Some(format!("backup original: {}", e));
            return Ok(result);
        }
    }
    result.backup_path = Some(backup.to_string_lossy().to_string());

    if let Err(e) = std::fs::rename(&unpacked_path, target) {
        result.error = Some(format!("replace target: {}", e));
        return Ok(result);
    }
    result.unpacked_path = Some(target.to_string_lossy().to_string());
    result.success = true;
    Ok(result)
}

fn backup_path_for(target: &Path) -> PathBuf {
    let mut p = target.as_os_str().to_owned();
    p.push(".drm.bak");
    PathBuf::from(p)
}
