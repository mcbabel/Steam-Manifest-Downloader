use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const STEAMID_BASE: u64 = 76_561_197_960_265_728;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamInstall {
    pub steam_dir: String,
    pub user_id3: String,
    pub user_id64: String,
    pub persona_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutAdded {
    pub shortcut_appid: u32,
    pub steam_dir: String,
    pub user_id3: String,
    pub grid_files: Vec<String>,
    #[serde(default)]
    pub steam_restarted: bool,
}

pub const STEAM_RUNNING_ERROR: &str = "STEAM_RUNNING";

#[derive(Debug, Clone)]
enum VdfNode {
    Object(Vec<(String, VdfNode)>),
    String(String),
    Int32(i32),
}

fn read_cstring(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let start = *pos;
    while *pos < bytes.len() && bytes[*pos] != 0 {
        *pos += 1;
    }
    if *pos >= bytes.len() {
        return Err("unexpected eof while reading cstring".into());
    }
    let s = std::str::from_utf8(&bytes[start..*pos])
        .map_err(|e| format!("invalid utf8 in cstring: {}", e))?
        .to_string();
    *pos += 1;
    Ok(s)
}

fn read_object(bytes: &[u8], pos: &mut usize) -> Result<Vec<(String, VdfNode)>, String> {
    let mut entries = Vec::new();
    while *pos < bytes.len() {
        let tag = bytes[*pos];
        *pos += 1;
        match tag {
            0x08 => return Ok(entries),
            0x00 => {
                let key = read_cstring(bytes, pos)?;
                let child = read_object(bytes, pos)?;
                entries.push((key, VdfNode::Object(child)));
            }
            0x01 => {
                let key = read_cstring(bytes, pos)?;
                let value = read_cstring(bytes, pos)?;
                entries.push((key, VdfNode::String(value)));
            }
            0x02 => {
                let key = read_cstring(bytes, pos)?;
                if *pos + 4 > bytes.len() {
                    return Err("eof while reading int32".into());
                }
                let v = i32::from_le_bytes([
                    bytes[*pos],
                    bytes[*pos + 1],
                    bytes[*pos + 2],
                    bytes[*pos + 3],
                ]);
                *pos += 4;
                entries.push((key, VdfNode::Int32(v)));
            }
            other => return Err(format!("unknown vdf tag {:#x} at {}", other, *pos - 1)),
        }
    }
    Ok(entries)
}

fn write_cstring(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

fn write_object(out: &mut Vec<u8>, entries: &[(String, VdfNode)]) {
    for (k, v) in entries {
        match v {
            VdfNode::Object(children) => {
                out.push(0x00);
                write_cstring(out, k);
                write_object(out, children);
                out.push(0x08);
            }
            VdfNode::String(s) => {
                out.push(0x01);
                write_cstring(out, k);
                write_cstring(out, s);
            }
            VdfNode::Int32(n) => {
                out.push(0x02);
                write_cstring(out, k);
                out.extend_from_slice(&n.to_le_bytes());
            }
        }
    }
}

fn parse_shortcuts(bytes: &[u8]) -> Result<Vec<(String, VdfNode)>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut pos = 0;
    if bytes[pos] != 0x00 {
        return Err(format!("expected 0x00 at start, got {:#x}", bytes[pos]));
    }
    pos += 1;
    let root_key = read_cstring(bytes, &mut pos)?;
    if root_key != "shortcuts" {
        return Err(format!("expected 'shortcuts' root, got '{}'", root_key));
    }
    read_object(bytes, &mut pos)
}

fn serialize_shortcuts(entries: &[(String, VdfNode)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0x00);
    write_cstring(&mut out, "shortcuts");
    write_object(&mut out, entries);
    out.push(0x08);
    out.push(0x08);
    out
}

fn parse_text_vdf_loginusers(text: &str) -> Vec<(String, std::collections::HashMap<String, String>)> {
    let mut out = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_map = std::collections::HashMap::new();
    let mut depth: u32 = 0;
    for raw in text.lines() {
        let line = raw.trim();
        if line == "{" {
            depth += 1;
            continue;
        }
        if line == "}" {
            if depth == 2 {
                if let Some(id) = current_id.take() {
                    out.push((id, std::mem::take(&mut current_map)));
                }
            }
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth == 1 && line.starts_with('"') {
            if let Some(end) = line[1..].find('"') {
                current_id = Some(line[1..=end].to_string());
            }
            continue;
        }
        if depth == 2 && line.starts_with('"') {
            let mut parts = line.splitn(2, "\"\t\t\"");
            if let Some(key_part) = parts.next() {
                if let Some(value_part) = parts.next() {
                    let key = key_part.trim_start_matches('"').to_string();
                    let value = value_part.trim_end_matches('"').to_string();
                    current_map.insert(key, value);
                    continue;
                }
            }
            let mut chunks = line.split('"').filter(|s| !s.trim().is_empty());
            let key = chunks.next().unwrap_or("").to_string();
            let value = chunks.next().unwrap_or("").to_string();
            if !key.is_empty() {
                current_map.insert(key, value);
            }
        }
    }
    out
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn linux_steam_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(home) = home_dir() {
        out.push(home.join(".steam").join("steam"));
        out.push(home.join(".local").join("share").join("Steam"));
        out.push(
            home.join(".var")
                .join("app")
                .join("com.valvesoftware.Steam")
                .join(".local")
                .join("share")
                .join("Steam"),
        );
    }
    out
}

#[cfg(target_os = "windows")]
fn registry_steam_path() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_reg_query_value(&String::from_utf8_lossy(&output.stdout), "SteamPath")
}

#[cfg(any(target_os = "windows", test))]
fn parse_reg_query_value(text: &str, name: &str) -> Option<PathBuf> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let (key, rest) = line.split_once(char::is_whitespace)?;
        if !key.eq_ignore_ascii_case(name) {
            return None;
        }
        let (kind, value) = rest.trim_start().split_once(char::is_whitespace)?;
        if !kind.starts_with("REG_") {
            return None;
        }
        let value = value.trim();
        (!value.is_empty()).then(|| PathBuf::from(value.replace('/', "\\")))
    })
}

#[cfg(target_os = "windows")]
fn windows_steam_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = registry_steam_path() {
        out.push(p);
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        out.push(PathBuf::from(pf86).join("Steam"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        out.push(PathBuf::from(pf).join("Steam"));
    }
    out
}

#[cfg(not(target_os = "windows"))]
fn windows_steam_candidates() -> Vec<PathBuf> {
    Vec::new()
}

fn steam_candidates() -> Vec<PathBuf> {
    let mut out = linux_steam_candidates();
    out.extend(windows_steam_candidates());
    out
}

pub fn detect_steam() -> Result<SteamInstall, String> {
    for candidate in steam_candidates() {
        let resolved = match std::fs::canonicalize(&candidate) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let loginusers = resolved.join("config").join("loginusers.vdf");
        if !loginusers.exists() {
            continue;
        }
        let text = match std::fs::read_to_string(&loginusers) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let users = parse_text_vdf_loginusers(&text);
        let chosen = users
            .iter()
            .find(|(_, m)| m.get("MostRecent").map(|v| v == "1").unwrap_or(false))
            .cloned()
            .or_else(|| users.first().cloned());
        let Some((id64_str, fields)) = chosen else {
            continue;
        };
        let id64: u64 = id64_str
            .parse()
            .map_err(|e| format!("invalid SteamID64 '{}': {}", id64_str, e))?;
        let id3 = (id64 - STEAMID_BASE).to_string();
        let userdata_dir = resolved.join("userdata").join(&id3);
        if !userdata_dir.exists() {
            continue;
        }
        return Ok(SteamInstall {
            steam_dir: resolved.to_string_lossy().to_string(),
            user_id3: id3,
            user_id64: id64_str,
            persona_name: fields.get("PersonaName").cloned(),
        });
    }
    Err("No Steam installation with an active user was found".into())
}

pub fn generate_shortcut_appid(exe: &str, app_name: &str) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(exe.as_bytes());
    hasher.update(app_name.as_bytes());
    hasher.finalize() | 0x8000_0000
}

fn build_shortcut_node(
    appid: u32,
    app_name: &str,
    exe_quoted: &str,
    start_dir: &str,
    icon: &str,
    launch_options: &str,
) -> VdfNode {
    VdfNode::Object(vec![
        ("appid".into(), VdfNode::Int32(appid as i32)),
        ("AppName".into(), VdfNode::String(app_name.into())),
        ("Exe".into(), VdfNode::String(exe_quoted.into())),
        ("StartDir".into(), VdfNode::String(start_dir.into())),
        ("icon".into(), VdfNode::String(icon.into())),
        ("ShortcutPath".into(), VdfNode::String(String::new())),
        (
            "LaunchOptions".into(),
            VdfNode::String(launch_options.into()),
        ),
        ("IsHidden".into(), VdfNode::Int32(0)),
        ("AllowDesktopConfig".into(), VdfNode::Int32(1)),
        ("AllowOverlay".into(), VdfNode::Int32(1)),
        ("OpenVR".into(), VdfNode::Int32(0)),
        ("Devkit".into(), VdfNode::Int32(0)),
        ("DevkitGameID".into(), VdfNode::String(String::new())),
        ("DevkitOverrideAppID".into(), VdfNode::Int32(0)),
        ("LastPlayTime".into(), VdfNode::Int32(0)),
        ("FlatpakAppID".into(), VdfNode::String(String::new())),
        ("sortas".into(), VdfNode::String(String::new())),
        ("tags".into(), VdfNode::Object(Vec::new())),
    ])
}

fn shortcut_index_entries(entries: &[(String, VdfNode)]) -> usize {
    entries.len()
}

fn shortcut_appid_in_entry(entry: &(String, VdfNode)) -> Option<u32> {
    let VdfNode::Object(fields) = &entry.1 else {
        return None;
    };
    for (k, v) in fields {
        if k.eq_ignore_ascii_case("appid") {
            if let VdfNode::Int32(n) = v {
                return Some(*n as u32);
            }
        }
    }
    None
}

fn shortcut_string_field<'a>(entry: &'a (String, VdfNode), name: &str) -> Option<&'a str> {
    let VdfNode::Object(fields) = &entry.1 else {
        return None;
    };
    fields.iter().find_map(|(k, v)| match v {
        VdfNode::String(s) if k.eq_ignore_ascii_case(name) => Some(s.as_str()),
        _ => None,
    })
}

fn same_exe(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.trim().trim_matches('"').replace('\\', "/");
    if cfg!(target_os = "windows") {
        norm(a).eq_ignore_ascii_case(&norm(b))
    } else {
        norm(a) == norm(b)
    }
}

pub fn find_shortcut_appid(exe_path: &str) -> Option<u32> {
    let install = detect_steam().ok()?;
    let path = PathBuf::from(&install.steam_dir)
        .join("userdata")
        .join(&install.user_id3)
        .join("config")
        .join("shortcuts.vdf");
    let entries = parse_shortcuts(&std::fs::read(path).ok()?).ok()?;
    entries
        .iter()
        .find(|e| shortcut_string_field(e, "Exe").is_some_and(|exe| same_exe(exe, exe_path)))
        .and_then(shortcut_appid_in_entry)
}

pub fn run_shortcut(appid: u32) -> Result<(), String> {
    let install = detect_steam()?;
    let game_id = (u64::from(appid) << 32) | 0x0200_0000;
    steam_command(Path::new(&install.steam_dir))
        .arg(format!("steam://rungameid/{}", game_id))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start Steam: {}", e))
}

pub fn quote_exe(path: &str) -> String {
    format!("\"{}\"", path)
}

pub fn add_or_replace_shortcut(
    steam_dir: &Path,
    user_id3: &str,
    app_name: &str,
    exe_path: &str,
    start_dir: &str,
    icon: &str,
    launch_options: &str,
) -> Result<u32, String> {
    let shortcuts_path = steam_dir
        .join("userdata")
        .join(user_id3)
        .join("config")
        .join("shortcuts.vdf");

    let existing_bytes = std::fs::read(&shortcuts_path).unwrap_or_default();
    let mut entries = if existing_bytes.is_empty() {
        Vec::new()
    } else {
        parse_shortcuts(&existing_bytes)
            .map_err(|e| format!("parse existing shortcuts.vdf: {}", e))?
    };

    let exe_quoted = quote_exe(exe_path);
    let appid = generate_shortcut_appid(&exe_quoted, app_name);
    let new_node = build_shortcut_node(appid, app_name, &exe_quoted, start_dir, icon, launch_options);

    let mut replaced = false;
    for entry in entries.iter_mut() {
        if shortcut_appid_in_entry(entry) == Some(appid) {
            entry.1 = new_node.clone();
            replaced = true;
            break;
        }
    }
    if !replaced {
        let index = shortcut_index_entries(&entries).to_string();
        entries.push((index, new_node));
    }

    let bytes = serialize_shortcuts(&entries);

    if let Some(parent) = shortcuts_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create config dir: {}", e))?;
    }
    if shortcuts_path.exists() {
        let backup = shortcuts_path.with_extension("vdf.bak");
        let _ = std::fs::copy(&shortcuts_path, &backup);
    }
    std::fs::write(&shortcuts_path, &bytes)
        .map_err(|e| format!("write shortcuts.vdf: {}", e))?;
    Ok(appid)
}

fn image_ext(url: &str) -> Option<&'static str> {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    if path.ends_with(".png") {
        Some("png")
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Some("jpg")
    } else {
        None
    }
}

async fn download_to(client: &Client, url: &str, dest: &Path) -> bool {
    let resp = match client
        .get(url)
        .timeout(Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return false,
    };
    let is_image = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|v| v.starts_with("image/"));
    if !resp.status().is_success() || !is_image {
        return false;
    }
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return false,
    };
    std::fs::write(dest, &bytes).is_ok()
}

pub async fn download_grid_art(
    client: &Client,
    steam_dir: &Path,
    user_id3: &str,
    shortcut_appid: u32,
    steam_app_id: &str,
) -> (Vec<String>, Option<String>) {
    let grid_dir = steam_dir
        .join("userdata")
        .join(user_id3)
        .join("config")
        .join("grid");
    if std::fs::create_dir_all(&grid_dir).is_err() {
        return (Vec::new(), None);
    }

    let resolved = crate::services::steam_assets::game_assets(client, steam_app_id).await;
    let legacy = crate::services::steam_assets::legacy(steam_app_id);
    let targets: [(&str, [Option<String>; 2], &str); 4] = [
        ("", [resolved.header.clone(), legacy.header.clone()], "jpg"),
        ("p", [resolved.capsule.clone(), legacy.capsule.clone()], "jpg"),
        ("_hero", [resolved.hero.clone(), legacy.hero.clone()], "jpg"),
        ("_logo", [resolved.logo.clone(), legacy.logo.clone()], "png"),
    ];

    let mut written = Vec::new();
    for (suffix, candidates, default_ext) in targets.iter() {
        for url in candidates.iter().flatten() {
            let ext = image_ext(url).unwrap_or(default_ext);
            let dest = grid_dir.join(format!("{}{}.{}", shortcut_appid, suffix, ext));
            if download_to(client, url, &dest).await {
                written.push(dest.to_string_lossy().to_string());
                break;
            }
        }
    }

    let icon_path = match resolved.icon {
        Some(url) => {
            let dest = grid_dir.join(format!("{}_icon.jpg", shortcut_appid));
            if download_to(client, &url, &dest).await {
                let p = dest.to_string_lossy().to_string();
                written.push(p.clone());
                Some(p)
            } else {
                None
            }
        }
        None => None,
    };

    (written, icon_path)
}

pub fn is_steam_running() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq steam.exe", "/FO", "CSV", "/NH"])
            .creation_flags(0x08000000)
            .output()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .to_ascii_lowercase()
                    .contains("\"steam.exe\"")
            })
            .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        std::fs::read_dir("/proc")
            .map(|entries| {
                entries.flatten().any(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .chars()
                        .all(|c| c.is_ascii_digit())
                        && std::fs::read_to_string(e.path().join("comm"))
                            .map(|c| c.trim() == "steam")
                            .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

fn steam_command(steam_dir: &Path) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new(steam_dir.join("steam.exe"));
        cmd.creation_flags(0x08000000);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let script = steam_dir.join("steam.sh");
        if script.exists() {
            std::process::Command::new(script)
        } else {
            std::process::Command::new("steam")
        }
    }
}

pub async fn shutdown_steam(steam_dir: &Path) -> Result<(), String> {
    steam_command(steam_dir)
        .arg("-shutdown")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not ask Steam to close: {}", e))?;
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if !is_steam_running() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            return Ok(());
        }
    }
    Err("Steam did not close within 30 seconds. Please close Steam yourself and try again.".into())
}

pub fn start_steam(steam_dir: &Path) -> Result<(), String> {
    steam_command(steam_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not start Steam: {}", e))
}

pub async fn add_to_steam_library(
    client: &Client,
    steam_app_id: &str,
    app_name: &str,
    exe_path: &str,
    start_dir: &str,
    launch_options: &str,
    close_steam: bool,
) -> Result<ShortcutAdded, String> {
    let install = detect_steam()?;
    let steam_dir = PathBuf::from(&install.steam_dir);

    let running = is_steam_running();
    if running && !close_steam {
        return Err(STEAM_RUNNING_ERROR.to_string());
    }

    let exe_quoted_for_id = quote_exe(exe_path);
    let appid = generate_shortcut_appid(&exe_quoted_for_id, app_name);

    let (grid_files, icon_path) =
        download_grid_art(client, &steam_dir, &install.user_id3, appid, steam_app_id).await;

    let icon = icon_path.unwrap_or_default();

    if running {
        shutdown_steam(&steam_dir).await?;
    }

    add_or_replace_shortcut(
        &steam_dir,
        &install.user_id3,
        app_name,
        exe_path,
        start_dir,
        &icon,
        launch_options,
    )?;

    let steam_restarted = running && start_steam(&steam_dir).is_ok();

    let _ = Utc::now();
    Ok(ShortcutAdded {
        shortcut_appid: appid,
        steam_dir: install.steam_dir,
        user_id3: install.user_id3,
        grid_files,
        steam_restarted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steam_path_from_reg_query_output() {
        let out = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    d:/games/steam\r\n\r\n";
        assert_eq!(
            parse_reg_query_value(out, "SteamPath"),
            Some(PathBuf::from("d:\\games\\steam"))
        );
    }

    #[test]
    fn reg_query_value_with_spaces_in_path() {
        let out = "    SteamPath    REG_SZ    c:/program files (x86)/steam\r\n";
        assert_eq!(
            parse_reg_query_value(out, "SteamPath"),
            Some(PathBuf::from("c:\\program files (x86)\\steam"))
        );
    }

    #[test]
    fn reg_query_value_missing() {
        assert_eq!(parse_reg_query_value("ERROR: not found", "SteamPath"), None);
    }
}
