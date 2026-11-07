use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::WalkDir;

use crate::services::emulator::EmuSettings;

const SAVE_MARKER: &str = "smd-saves.json";
const DEFAULT_SAVES_FOLDER: &str = "GSE Saves";
const AVATAR_BASE: &str = "account_avatar";
const FONTS_DIR: &str = "fonts";
const SOUNDS_DIR: &str = "sounds";
const MODS_DIR: &str = "mods";
const MOD_IMAGES_DIR: &str = "mod_images";
const MODS_JSON: &str = "mods.json";
const BROADCASTS: &str = "custom_broadcasts.txt";
const MAX_IMAGE_MB: u64 = 10;
const MAX_FONT_MB: u64 = 30;
const MAX_SOUND_MB: u64 = 20;
const MAX_PREVIEW_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SaveFolder {
    pub path: String,
    pub exists: bool,
    pub files: usize,
    pub size_bytes: u64,
    pub modified: Option<u64>,
    pub place: String,
    pub portable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AchievementRow {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub hidden: bool,
    pub earned: bool,
    pub earned_time: u64,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementView {
    pub rows: Vec<AchievementRow>,
    pub stats_files: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OverlayAssets {
    pub avatar: Option<String>,
    pub font: Option<String>,
    pub achievement_sound: Option<String>,
    pub friend_sound: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FontEntry {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModEntry {
    pub id: String,
    pub title: String,
    pub files: usize,
    pub size_bytes: u64,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AddedMod {
    pub id: String,
    pub title: String,
    pub other_app: Option<String>,
}

pub fn settings_dir(target: &Path) -> Option<PathBuf> {
    target.parent().map(|p| p.join("steam_settings"))
}

fn settings_dirs(targets: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut dirs: Vec<PathBuf> = targets.iter().filter_map(|t| settings_dir(Path::new(t))).collect();
    dirs.sort();
    dirs.dedup();
    if dirs.is_empty() {
        return Err("No emulator files selected".into());
    }
    Ok(dirs)
}

fn app_id_for(target: &Path, app_id: &str) -> String {
    let given = app_id.trim();
    if !given.is_empty() {
        return given.to_string();
    }
    settings_dir(target)
        .and_then(|d| fs::read_to_string(d.join("steam_appid.txt")).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn dir_stats(dir: &Path) -> (usize, u64, Option<u64>) {
    let mut files = 0;
    let mut size = 0;
    let mut newest: Option<u64> = None;
    for entry in WalkDir::new(dir).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        files += 1;
        if let Ok(meta) = entry.metadata() {
            size += meta.len();
            let secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            newest = newest.max(secs);
        }
    }
    (files, size, newest)
}

fn folder(path: PathBuf, place: &str, portable: bool) -> SaveFolder {
    let exists = path.is_dir();
    let (files, size_bytes, modified) = if exists { dir_stats(&path) } else { (0, 0, None) };
    SaveFolder {
        path: path.to_string_lossy().to_string(),
        exists,
        files,
        size_bytes,
        modified,
        place: place.to_string(),
        portable,
    }
}

fn wine_candidates(folder_name: &str, app_id: &str) -> Vec<(PathBuf, &'static str)> {
    let mut out = Vec::new();
    let Some(home) = dirs::home_dir() else { return out };
    if let Ok(users) = fs::read_dir(home.join(".wine/drive_c/users")) {
        for u in users.flatten() {
            let p = u.path().join("AppData/Roaming").join(folder_name).join(app_id);
            out.push((p, "wine"));
        }
    }
    for root in [
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/data/Steam"),
    ] {
        let Ok(prefixes) = fs::read_dir(root.join("steamapps/compatdata")) else { continue };
        for pfx in prefixes.flatten() {
            let p = pfx
                .path()
                .join("pfx/drive_c/users/steamuser/AppData/Roaming")
                .join(folder_name)
                .join(app_id);
            out.push((p, "proton"));
        }
    }
    out
}

pub fn save_folders(target: &Path, app_id: &str) -> Vec<SaveFolder> {
    let app_id = app_id_for(target, app_id);
    if app_id.is_empty() {
        return Vec::new();
    }
    let settings = settings_dir(target).map(|d| EmuSettings::read_from_dir(&d)).unwrap_or_default();
    if let Some(local) = settings.local_save_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let base = Path::new(local);
        let base = if base.is_absolute() {
            base.to_path_buf()
        } else {
            target.parent().map(|p| p.join(base)).unwrap_or_else(|| base.to_path_buf())
        };
        return vec![folder(base.join(&app_id), "portable", true)];
    }
    let folder_name = settings
        .saves_folder_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_SAVES_FOLDER)
        .to_string();
    let windows_target = target.to_string_lossy().to_ascii_lowercase().ends_with(".dll");
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from).or_else(dirs::config_dir) {
            out.push(folder(appdata.join(&folder_name).join(&app_id), "windows", false));
        }
    } else if windows_target {
        let candidates = wine_candidates(&folder_name, &app_id);
        out.extend(candidates.iter().filter(|(p, _)| p.is_dir()).map(|(p, place)| folder(p.clone(), place, false)));
        if out.is_empty() {
            if let Some((p, place)) = candidates.into_iter().find(|(_, place)| *place == "wine") {
                out.push(folder(p, place, false));
            }
        }
    } else {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .or_else(|| dirs::home_dir().map(|h| h.join(".local/share")));
        if let Some(base) = base {
            out.push(folder(base.join(&folder_name).join(&app_id), "linux", false));
        }
    }
    out.sort_by(|a, b| b.exists.cmp(&a.exists).then(b.modified.cmp(&a.modified)));
    out
}

fn active_save_dir(target: &Path, app_id: &str) -> Option<PathBuf> {
    save_folders(target, app_id)
        .into_iter()
        .find(|f| f.exists)
        .map(|f| PathBuf::from(f.path))
}

pub fn backup_saves(folder: &Path, dest: &Path) -> Result<usize, String> {
    if !folder.is_dir() {
        return Err(format!("Save folder not found: {}", folder.display()));
    }
    let file = fs::File::create(dest).map_err(|e| format!("create {}: {}", dest.display(), e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let marker = serde_json::json!({ "folder": folder.file_name().map(|n| n.to_string_lossy().to_string()) });
    zip.start_file(SAVE_MARKER, opts).map_err(|e| e.to_string())?;
    zip.write_all(marker.to_string().as_bytes()).map_err(|e| e.to_string())?;
    let mut count = 0;
    for entry in WalkDir::new(folder).follow_links(false).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(folder) else { continue };
        let name = rel.to_string_lossy().replace('\\', "/");
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        fs::File::open(entry.path())
            .and_then(|mut f| f.read_to_end(&mut data))
            .map_err(|e| format!("read {}: {}", entry.path().display(), e))?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
        count += 1;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(count)
}

pub fn restore_saves(folder: &Path, archive: &Path) -> Result<usize, String> {
    let file = fs::File::open(archive).map_err(|e| format!("open {}: {}", archive.display(), e))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|_| format!("Not a save backup: {}", archive.display()))?;
    if zip.by_name(SAVE_MARKER).is_err() {
        return Err(format!("Not a save backup: {}", archive.display()));
    }
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.enclosed_name().is_none() {
            return Err(format!("Unsafe path in backup: {}", entry.name().map(|n| n.to_string()).unwrap_or_default()));
        }
    }
    let before = PathBuf::from(format!("{}.before-restore", folder.display()));
    if folder.exists() {
        let _ = fs::remove_dir_all(&before);
        fs::rename(folder, &before).map_err(|e| format!("move {}: {}", folder.display(), e))?;
    }
    fs::create_dir_all(folder).map_err(|e| format!("create {}: {}", folder.display(), e))?;
    let mut count = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        if rel == Path::new(SAVE_MARKER) {
            continue;
        }
        let out = folder.join(rel);
        if entry.is_dir() {
            let _ = fs::create_dir_all(&out);
            continue;
        }
        if let Some(parent) = out.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        fs::write(&out, data).map_err(|e| format!("write {}: {}", out.display(), e))?;
        count += 1;
    }
    Ok(count)
}

fn text_in(v: &Value, lang: &str) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(map) => map
            .get(lang)
            .or_else(|| map.get("english"))
            .or_else(|| map.values().next())
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        Some(Value::String(s)) => s.trim() == "1" || s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

fn read_json(path: &Path) -> Option<Value> {
    fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn achievements(target: &Path, app_id: &str, lang: &str) -> AchievementView {
    let Some(sdir) = settings_dir(target) else { return AchievementView::default() };
    let db = read_json(&sdir.join("achievements.json")).and_then(|v| v.as_array().cloned()).unwrap_or_default();
    let save = active_save_dir(target, app_id);
    let user = save
        .as_ref()
        .and_then(|d| read_json(&d.join("achievements.json")))
        .unwrap_or(Value::Null);
    let rows = db
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let state = user.get(&name);
            let icon = a
                .get("icon")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| sdir.join(s))
                .filter(|p| p.is_file())
                .map(|p| p.to_string_lossy().to_string());
            Some(AchievementRow {
                display_name: text_in(a.get("displayName").unwrap_or(&Value::Null), lang),
                description: text_in(a.get("description").unwrap_or(&Value::Null), lang),
                hidden: truthy(a.get("hidden")),
                earned: truthy(state.and_then(|s| s.get("earned"))),
                earned_time: state.and_then(|s| s.get("earned_time")).and_then(|v| v.as_u64()).unwrap_or(0),
                icon,
                name,
            })
        })
        .collect();
    let stats_files = save.map(|d| dir_stats(&d.join("stats")).0).unwrap_or(0);
    AchievementView { rows, stats_files }
}

pub fn reset_achievements(target: &Path, app_id: &str, names: Option<&[String]>) -> Result<usize, String> {
    let Some(save) = active_save_dir(target, app_id) else { return Ok(0) };
    let path = save.join("achievements.json");
    let Some(mut user) = read_json(&path) else { return Ok(0) };
    let Some(map) = user.as_object_mut() else { return Ok(0) };
    let mut changed = 0;
    for (name, state) in map.iter_mut() {
        if names.is_some_and(|n| !n.iter().any(|x| x == name)) {
            continue;
        }
        if truthy(state.get("earned")) {
            state["earned"] = Value::Bool(false);
            state["earned_time"] = Value::from(0);
            changed += 1;
        }
    }
    if changed > 0 {
        fs::write(&path, serde_json::to_string_pretty(&user).unwrap_or_default())
            .map_err(|e| format!("write {}: {}", path.display(), e))?;
    }
    Ok(changed)
}

pub fn reset_stats(target: &Path, app_id: &str) -> Result<usize, String> {
    let Some(save) = active_save_dir(target, app_id) else { return Ok(0) };
    let dir = save.join("stats");
    let mut count = 0;
    for entry in WalkDir::new(&dir).follow_links(false).into_iter().flatten() {
        if entry.file_type().is_file() && fs::remove_file(entry.path()).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

pub fn image_data_url(path: &Path) -> Result<String, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => return Err("Choose a PNG or JPG image".into()),
    };
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() as usize > MAX_PREVIEW_BYTES {
        return Err(format!("File is too large (max {} MB)", MAX_PREVIEW_BYTES / 1024 / 1024));
    }
    let data = fs::read(path).map_err(|e| e.to_string())?;
    Ok(format!("data:{};base64,{}", mime, base64::engine::general_purpose::STANDARD.encode(data)))
}

fn checked_source(src: &Path, exts: &[&str], wrong: &str, max_mb: u64) -> Result<String, String> {
    let ext = src
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !exts.contains(&ext.as_str()) {
        return Err(wrong.to_string());
    }
    let meta = fs::metadata(src).map_err(|e| format!("read {}: {}", src.display(), e))?;
    if !meta.is_file() {
        return Err(wrong.to_string());
    }
    if meta.len() > max_mb * 1024 * 1024 {
        return Err(format!("File is too large (max {} MB)", max_mb));
    }
    Ok(ext)
}

fn safe_name(path: &Path) -> String {
    let base = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let cleaned: String = base
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' })
        .collect();
    cleaned.trim_start_matches('.').to_string()
}

fn update_settings(dir: &Path, f: impl FnOnce(&mut EmuSettings)) -> Result<(), String> {
    let mut s = EmuSettings::read_from_dir(dir);
    f(&mut s);
    s.write_to_dir(dir)
}

fn avatar_in(dir: &Path) -> Option<PathBuf> {
    ["png", "jpg", "jpeg"]
        .iter()
        .map(|e| dir.join(format!("{}.{}", AVATAR_BASE, e)))
        .find(|p| p.is_file())
}

pub fn overlay_assets(target: &Path) -> OverlayAssets {
    let Some(dir) = settings_dir(target) else { return OverlayAssets::default() };
    let font = EmuSettings::read_from_dir(&dir)
        .font_override
        .filter(|f| dir.join(FONTS_DIR).join(f).is_file());
    let sound = |kind: &str| {
        let p = dir.join(SOUNDS_DIR).join(format!("overlay_{}_notification.wav", kind));
        p.is_file().then(|| p.to_string_lossy().to_string())
    };
    OverlayAssets {
        avatar: avatar_in(&dir).map(|p| p.to_string_lossy().to_string()),
        font,
        achievement_sound: sound("achievement"),
        friend_sound: sound("friend"),
    }
}

pub fn set_avatar(targets: &[String], src: &Path) -> Result<(), String> {
    let ext = checked_source(src, &["png", "jpg", "jpeg"], "Choose a PNG or JPG image", MAX_IMAGE_MB)?;
    let ext = if ext == "jpeg" { "jpg".to_string() } else { ext };
    for dir in settings_dirs(targets)? {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        while let Some(old) = avatar_in(&dir) {
            fs::remove_file(&old).map_err(|e| e.to_string())?;
        }
        fs::copy(src, dir.join(format!("{}.{}", AVATAR_BASE, ext))).map_err(|e| e.to_string())?;
        update_settings(&dir, |s| s.enable_account_avatar = Some(true))?;
    }
    Ok(())
}

pub fn remove_avatar(targets: &[String]) -> Result<(), String> {
    for dir in settings_dirs(targets)? {
        while let Some(old) = avatar_in(&dir) {
            fs::remove_file(&old).map_err(|e| e.to_string())?;
        }
        if dir.is_dir() {
            update_settings(&dir, |s| s.enable_account_avatar = None)?;
        }
    }
    Ok(())
}

pub fn set_font(targets: &[String], src: &Path) -> Result<(), String> {
    checked_source(src, &["ttf", "otf"], "Choose a TrueType or OpenType font (.ttf, .otf)", MAX_FONT_MB)?;
    let name = safe_name(src);
    for dir in settings_dirs(targets)? {
        let fonts = dir.join(FONTS_DIR);
        fs::create_dir_all(&fonts).map_err(|e| e.to_string())?;
        let old = EmuSettings::read_from_dir(&dir).font_override;
        if let Some(old) = old.filter(|o| *o != name) {
            let _ = fs::remove_file(fonts.join(old));
        }
        fs::copy(src, fonts.join(&name)).map_err(|e| e.to_string())?;
        update_settings(&dir, |s| s.font_override = Some(name.clone()))?;
    }
    Ok(())
}

pub fn remove_font(targets: &[String]) -> Result<(), String> {
    for dir in settings_dirs(targets)? {
        if !dir.is_dir() {
            continue;
        }
        if let Some(old) = EmuSettings::read_from_dir(&dir).font_override {
            let _ = fs::remove_file(dir.join(FONTS_DIR).join(old));
        }
        update_settings(&dir, |s| s.font_override = Some(String::new()))?;
    }
    Ok(())
}

fn sound_kind(kind: &str) -> Result<&'static str, String> {
    match kind {
        "achievement" => Ok("achievement"),
        "friend" => Ok("friend"),
        _ => Err(format!("unknown sound {}", kind)),
    }
}

pub fn set_sound(targets: &[String], kind: &str, src: &Path) -> Result<(), String> {
    let kind = sound_kind(kind)?;
    checked_source(src, &["wav"], "Choose a WAV sound file", MAX_SOUND_MB)?;
    for dir in settings_dirs(targets)? {
        let sounds = dir.join(SOUNDS_DIR);
        fs::create_dir_all(&sounds).map_err(|e| e.to_string())?;
        fs::copy(src, sounds.join(format!("overlay_{}_notification.wav", kind))).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove_sound(targets: &[String], kind: &str) -> Result<(), String> {
    let kind = sound_kind(kind)?;
    for dir in settings_dirs(targets)? {
        let _ = fs::remove_file(dir.join(SOUNDS_DIR).join(format!("overlay_{}_notification.wav", kind)));
    }
    Ok(())
}

fn font_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        if let Some(win) = std::env::var_os("WINDIR") {
            out.push(PathBuf::from(win).join("Fonts"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        out.push(PathBuf::from("/System/Library/Fonts"));
        out.push(PathBuf::from("/Library/Fonts"));
        if let Some(h) = dirs::home_dir() {
            out.push(h.join("Library/Fonts"));
        }
    } else {
        out.push(PathBuf::from("/usr/share/fonts"));
        out.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(h) = dirs::home_dir() {
            out.push(h.join(".local/share/fonts"));
            out.push(h.join(".fonts"));
        }
    }
    out
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    d.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn font_full_name(data: &[u8]) -> Option<String> {
    let tables = be16(data, 4)? as usize;
    let mut name_off = None;
    for i in 0..tables.min(64) {
        let rec = 12 + i * 16;
        if data.get(rec..rec + 4)? == b"name" {
            name_off = Some(be32(data, rec + 8)? as usize);
            break;
        }
    }
    let base = name_off?;
    let count = be16(data, base + 2)? as usize;
    let strings = base + be16(data, base + 4)? as usize;
    let mut best: Option<(u8, String)> = None;
    for i in 0..count.min(512) {
        let rec = base + 6 + i * 12;
        let platform = be16(data, rec)?;
        let lang = be16(data, rec + 4)?;
        let id = be16(data, rec + 6)?;
        if id != 4 && id != 1 {
            continue;
        }
        let len = be16(data, rec + 8)? as usize;
        let off = strings + be16(data, rec + 10)? as usize;
        let raw = data.get(off..off + len)?;
        let text = match platform {
            0 | 3 => {
                let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                String::from_utf16_lossy(&units)
            }
            1 => raw.iter().map(|&b| b as char).collect(),
            _ => continue,
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let rank = match (id, platform, lang) {
            (4, 3, 0x409) => 0,
            (4, 3, _) | (4, 0, _) => 1,
            (4, 1, _) => 2,
            _ => 3,
        };
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, t)| t)
}

pub fn installed_fonts() -> Vec<FontEntry> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for dir in font_dirs() {
        for entry in WalkDir::new(&dir).max_depth(4).follow_links(true).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            let mut head = Vec::new();
            let Ok(f) = fs::File::open(entry.path()) else { continue };
            if f.take(4 * 1024 * 1024).read_to_end(&mut head).is_err() {
                continue;
            }
            let name = font_full_name(&head).unwrap_or_else(|| {
                entry.path().file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
            });
            seen.entry(name).or_insert_with(|| entry.path().to_string_lossy().to_string());
        }
    }
    let mut list: Vec<FontEntry> = seen.into_iter().map(|(name, path)| FontEntry { name, path }).collect();
    list.sort_by_key(|f| f.name.to_lowercase());
    list
}

pub fn broadcasts(target: &Path) -> Vec<String> {
    settings_dir(target)
        .and_then(|d| fs::read_to_string(d.join(BROADCASTS)).ok())
        .map(|t| {
            t.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn is_address(s: &str) -> bool {
    if s.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    s.len() <= 253
        && s.contains(|c: char| c.is_ascii_alphabetic())
        && s.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

pub fn set_broadcasts(targets: &[String], list: &[String]) -> Result<Vec<String>, String> {
    let mut good: Vec<String> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for item in list.iter().flat_map(|l| l.split([',', ';', ' ', '\t', '\n', '\r'])) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        if is_address(item) {
            if !good.iter().any(|g| g == item) {
                good.push(item.to_string());
            }
        } else {
            bad.push(item.to_string());
        }
    }
    for dir in settings_dirs(targets)? {
        let path = dir.join(BROADCASTS);
        if good.is_empty() {
            let _ = fs::remove_file(&path);
            continue;
        }
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(&path, good.join("\n") + "\n").map_err(|e| format!("write {}: {}", BROADCASTS, e))?;
    }
    Ok(bad)
}

pub fn lobby_tool(platform_cache: &Path, windows: bool) -> Option<PathBuf> {
    let name = if windows { "lobby_connect_x64.exe" } else { "lobby_connect_x64" };
    WalkDir::new(platform_cache)
        .max_depth(6)
        .into_iter()
        .flatten()
        .find(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path().to_path_buf())
}

pub fn place_lobby_tool(tool: &Path, target: &Path) -> Result<PathBuf, String> {
    let dir = target.parent().ok_or("target has no parent dir")?;
    let dest = dir.join(tool.file_name().ok_or("bad tool path")?);
    fs::copy(tool, &dest).map_err(|e| format!("copy lobby_connect: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    }
    Ok(dest)
}

pub fn launch_console(path: &Path) -> Result<(), String> {
    let is_exe = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    let dir = path.parent().unwrap_or(Path::new("."));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = is_exe;
        std::process::Command::new(path)
            .current_dir(dir)
            .creation_flags(0x0000_0010)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        if is_exe {
            return Err("WINDOWS_TOOL_ON_LINUX".into());
        }
        let program = path.to_string_lossy().to_string();
        let terminals: [(&str, &[&str]); 7] = [
            ("x-terminal-emulator", &["-e"]),
            ("gnome-terminal", &["--"]),
            ("konsole", &["-e"]),
            ("xfce4-terminal", &["-x"]),
            ("kitty", &[]),
            ("alacritty", &["-e"]),
            ("xterm", &["-e"]),
        ];
        for (term, args) in terminals {
            let spawned = std::process::Command::new(term)
                .args(args)
                .arg(&program)
                .current_dir(dir)
                .spawn();
            if spawned.is_ok() {
                return Ok(());
            }
        }
        Err("No terminal program found to run lobby_connect in".into())
    }
}

pub fn parse_workshop_id(text: &str) -> Option<String> {
    let text = text.trim();
    if !text.is_empty() && text.chars().all(|c| c.is_ascii_digit()) {
        return Some(text.to_string());
    }
    let after = text.split("id=").nth(1)?;
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    (!digits.is_empty()).then_some(digits)
}

fn read_mods_json(dir: &Path) -> serde_json::Map<String, Value> {
    read_json(&dir.join(MODS_JSON))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

pub fn mods(target: &Path) -> Vec<ModEntry> {
    let Some(dir) = settings_dir(target) else { return Vec::new() };
    let meta = read_mods_json(&dir);
    let Ok(entries) = fs::read_dir(dir.join(MODS_DIR)) else { return Vec::new() };
    let mut out: Vec<ModEntry> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let id = e.file_name().to_string_lossy().to_string();
            let info = meta.get(&id);
            let (files, size_bytes, _) = dir_stats(&e.path());
            let preview = info
                .and_then(|i| i.get("preview_filename"))
                .and_then(|v| v.as_str())
                .map(|f| dir.join(MOD_IMAGES_DIR).join(&id).join(f))
                .filter(|p| p.is_file())
                .map(|p| p.to_string_lossy().to_string());
            ModEntry {
                title: info
                    .and_then(|i| i.get("title"))
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&id)
                    .to_string(),
                id,
                files,
                size_bytes,
                preview,
            }
        })
        .collect();
    out.sort_by_key(|m| m.title.to_lowercase());
    out
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(Option<String>, u64), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if src.is_file() {
        let name = src.file_name().ok_or("bad file")?;
        fs::copy(src, dest.join(name)).map_err(|e| e.to_string())?;
        let size = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        return Ok((Some(name.to_string_lossy().to_string()), size));
    }
    let mut first: Option<String> = None;
    let mut total = 0;
    for entry in WalkDir::new(src).follow_links(false).into_iter().flatten() {
        let Ok(rel) = entry.path().strip_prefix(src) else { continue };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(entry.path(), &out).map_err(|e| e.to_string())?;
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            if first.is_none() {
                first = Some(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    Ok((first, total))
}

async fn workshop_details(http: &reqwest::Client, id: &str) -> Option<Value> {
    let resp = http
        .post("https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(format!("itemcount=1&publishedfileids%5B0%5D={}", id))
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let body: Value = resp.json().await.ok()?;
    let d = body.pointer("/response/publishedfiledetails/0")?.clone();
    (d.get("result").and_then(|r| r.as_i64()) == Some(1)).then_some(d)
}

async fn download_preview(http: &reqwest::Client, url: &str) -> Option<(Vec<u8>, &'static str)> {
    if !url.starts_with("https://") {
        return None;
    }
    let resp = http.get(url).send().await.ok()?.error_for_status().ok()?;
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = if mime.contains("png") { "png" } else if mime.contains("gif") { "gif" } else { "jpg" };
    let bytes = resp.bytes().await.ok()?;
    (bytes.len() <= MAX_PREVIEW_BYTES).then(|| (bytes.to_vec(), ext))
}

pub async fn add_mod(
    http: &reqwest::Client,
    targets: &[String],
    app_id: &str,
    id_text: &str,
    src: &Path,
) -> Result<AddedMod, String> {
    let id = parse_workshop_id(id_text).ok_or("Enter the workshop ID or link of the mod")?;
    let dirs = settings_dirs(targets)?;
    let src_abs = src.canonicalize().map_err(|e| format!("read {}: {}", src.display(), e))?;
    for t in targets {
        let game = Path::new(t).parent().and_then(|p| p.canonicalize().ok());
        if let Some(game) = game {
            if src_abs.is_dir() && (game.starts_with(&src_abs) || src_abs.starts_with(&game)) {
                return Err("Choose the mod's own folder, not the game's".into());
            }
        }
    }
    let details = workshop_details(http, &id).await;
    let title = details
        .as_ref()
        .and_then(|d| d.get("title"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&id)
        .to_string();
    let other_app = details
        .as_ref()
        .and_then(|d| d.get("consumer_app_id").or_else(|| d.get("creator_app_id")))
        .and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .map(|n| n.to_string())
        .filter(|app| !app_id.trim().is_empty() && app != app_id.trim() && app != "0");
    let preview = match details.as_ref().and_then(|d| d.get("preview_url")).and_then(|v| v.as_str()) {
        Some(url) => download_preview(http, url).await,
        None => None,
    };
    for dir in &dirs {
        let dest = dir.join(MODS_DIR).join(&id);
        let _ = fs::remove_dir_all(&dest);
        let (primary, total) = copy_tree(&src_abs, &dest)?;
        let mut entry = serde_json::json!({
            "title": title,
            "primary_filename": primary.clone().unwrap_or_default(),
            "total_files_sizes": total,
            "workshop_item_url": format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id),
        });
        if let Some(d) = &details {
            for key in ["description", "time_created", "time_updated", "tags"] {
                if let Some(v) = d.get(key) {
                    if key == "tags" {
                        let tags: Vec<String> = v
                            .as_array()
                            .map(|a| a.iter().filter_map(|t| t.get("tag").and_then(|x| x.as_str()).map(str::to_string)).collect())
                            .unwrap_or_default();
                        entry["tags"] = Value::String(tags.join(","));
                    } else {
                        entry[key] = v.clone();
                    }
                }
            }
        }
        if let Some((bytes, ext)) = &preview {
            let img_dir = dir.join(MOD_IMAGES_DIR).join(&id);
            let _ = fs::remove_dir_all(&img_dir);
            fs::create_dir_all(&img_dir).map_err(|e| e.to_string())?;
            let name = format!("preview.{}", ext);
            fs::write(img_dir.join(&name), bytes).map_err(|e| e.to_string())?;
            entry["preview_filename"] = Value::String(name);
        }
        let mut all = read_mods_json(dir);
        all.insert(id.clone(), entry);
        fs::write(dir.join(MODS_JSON), serde_json::to_string_pretty(&Value::Object(all)).unwrap_or_default())
            .map_err(|e| format!("write {}: {}", MODS_JSON, e))?;
    }
    Ok(AddedMod { id, title, other_app })
}

pub fn remove_mod(targets: &[String], id: &str) -> Result<(), String> {
    let id = parse_workshop_id(id).ok_or("Enter the workshop ID or link of the mod")?;
    for dir in settings_dirs(targets)? {
        let _ = fs::remove_dir_all(dir.join(MODS_DIR).join(&id));
        let _ = fs::remove_dir_all(dir.join(MOD_IMAGES_DIR).join(&id));
        let mut all = read_mods_json(&dir);
        if all.remove(&id).is_some() {
            let path = dir.join(MODS_JSON);
            if all.is_empty() {
                let _ = fs::remove_file(&path);
            } else {
                fs::write(&path, serde_json::to_string_pretty(&Value::Object(all)).unwrap_or_default())
                    .map_err(|e| format!("write {}: {}", MODS_JSON, e))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(name: &str) -> (PathBuf, String) {
        let root = std::env::temp_dir().join(format!("smd-extras-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("game/steam_settings")).unwrap();
        fs::write(root.join("game/steam_settings/steam_appid.txt"), "480").unwrap();
        let target = root.join("game/libsteam_api.so").to_string_lossy().to_string();
        (root, target)
    }

    #[test]
    fn portable_saves_back_up_and_restore() {
        let (root, target) = game("saves");
        fs::write(
            root.join("game/steam_settings/configs.user.ini"),
            "[user::saves]\nlocal_save_path=./saves\n",
        )
        .unwrap();
        let folders = save_folders(Path::new(&target), "");
        assert_eq!(folders.len(), 1);
        assert!(folders[0].portable && !folders[0].exists);
        let save = PathBuf::from(&folders[0].path);
        fs::create_dir_all(save.join("remote")).unwrap();
        fs::write(save.join("remote/slot1.sav"), b"one").unwrap();
        let zip = root.join("backup.zip");
        assert_eq!(backup_saves(&save, &zip).unwrap(), 1);
        fs::write(save.join("remote/slot1.sav"), b"changed").unwrap();
        assert_eq!(restore_saves(&save, &zip).unwrap(), 1);
        assert_eq!(fs::read(save.join("remote/slot1.sav")).unwrap(), b"one");
        assert!(PathBuf::from(format!("{}.before-restore", save.display())).join("remote/slot1.sav").is_file());
        assert!(restore_saves(&save, &root.join("game/steam_settings/steam_appid.txt")).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn achievements_merge_and_lock_again() {
        let (root, target) = game("ach");
        fs::write(
            root.join("game/steam_settings/configs.user.ini"),
            "[user::saves]\nlocal_save_path=./saves\n",
        )
        .unwrap();
        fs::write(
            root.join("game/steam_settings/achievements.json"),
            r#"[{"name":"A","displayName":{"english":"First","german":"Erstes"},"description":"d","hidden":0},{"name":"B","displayName":"Second","hidden":"1"}]"#,
        )
        .unwrap();
        let save = root.join("game/saves/480");
        fs::create_dir_all(save.join("stats")).unwrap();
        fs::write(save.join("stats/kills"), b"1").unwrap();
        fs::write(save.join("achievements.json"), r#"{"A":{"earned":true,"earned_time":5},"B":{"earned":false,"earned_time":0}}"#).unwrap();
        let view = achievements(Path::new(&target), "", "german");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].display_name, "Erstes");
        assert!(view.rows[0].earned && view.rows[1].hidden);
        assert_eq!(view.stats_files, 1);
        assert_eq!(reset_achievements(Path::new(&target), "", None).unwrap(), 1);
        assert!(!achievements(Path::new(&target), "", "english").rows[0].earned);
        assert_eq!(reset_stats(Path::new(&target), "").unwrap(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn broadcasts_keep_addresses_and_report_the_rest() {
        let (root, target) = game("bc");
        let bad = set_broadcasts(&[target.clone()], &["100.64.0.2\n300.1.1.1 ip!".into(), "pc.lan, ::1".into()]).unwrap();
        assert_eq!(bad, vec!["300.1.1.1".to_string(), "ip!".to_string()]);
        assert_eq!(broadcasts(Path::new(&target)), vec!["100.64.0.2", "pc.lan", "::1"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workshop_ids_come_from_ids_and_links() {
        assert_eq!(parse_workshop_id("123456").as_deref(), Some("123456"));
        assert_eq!(
            parse_workshop_id("https://steamcommunity.com/sharedfiles/filedetails/?id=987&searchtext=").as_deref(),
            Some("987")
        );
        assert_eq!(parse_workshop_id("mod"), None);
    }

    #[test]
    fn sounds_and_fonts_land_in_steam_settings() {
        let (root, target) = game("assets");
        let wav = root.join("ding.wav");
        fs::write(&wav, b"RIFF").unwrap();
        set_sound(&[target.clone()], "achievement", &wav).unwrap();
        let font = root.join("My Font.ttf");
        fs::write(&font, b"\0\x01\0\0").unwrap();
        set_font(&[target.clone()], &font).unwrap();
        let assets = overlay_assets(Path::new(&target));
        assert!(assets.achievement_sound.is_some() && assets.friend_sound.is_none());
        assert_eq!(assets.font.as_deref(), Some("My_Font.ttf"));
        assert!(set_font(&[target.clone()], &wav).is_err());
        remove_font(&[target.clone()]).unwrap();
        assert_eq!(overlay_assets(Path::new(&target)).font, None);
        let _ = fs::remove_dir_all(&root);
    }
}
