mod achievements;
mod cloud;
mod controller;
mod inventory;
mod leaderboards;
mod media;
mod watcher;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use achievements::{achievements_json, stats_json, Achievement, Stat};
pub use media::MediaLevel;

use crate::services::steam_pics;
use crate::services::steam_session::SteamSession;

const API: &str = "https://api.steampowered.com";
const COMMUNITY_IMAGES: &str = "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps";
const CONTROLLER_DIR: &str = "controller";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameDataResult {
    pub languages: usize,
    pub depots: usize,
    pub branches: usize,
    pub achievements: usize,
    pub achievement_languages: usize,
    pub stats: usize,
    pub icons: usize,
    pub leaderboards: usize,
    pub items: usize,
    pub controller_sets: usize,
    pub cloud_dirs: usize,
    pub watcher_schemas: usize,
    pub media_files: usize,
    pub source: String,
    pub notes: Vec<String>,
}

pub struct Options<'a> {
    pub language: &'a str,
    pub web_api_key: Option<&'a str>,
    pub media: MediaLevel,
}

pub fn settings_dirs(targets: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = targets
        .iter()
        .filter_map(|t| Path::new(t).parent().map(|p| p.join("steam_settings")))
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

pub fn find_game_root(targets: &[String]) -> Option<PathBuf> {
    for t in targets {
        let mut dir = Path::new(t).parent();
        for _ in 0..8 {
            let Some(d) = dir else { break };
            if !crate::services::install_state::installed(d).is_empty() {
                return Some(d.to_path_buf());
            }
            dir = d.parent();
        }
    }
    None
}

fn safe_file_name(url: &str) -> Option<String> {
    let name = url.rsplit('/').next()?.split('?').next()?;
    let ok = !name.is_empty()
        && name.len() <= 128
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    ok.then(|| name.to_string())
}

async fn get_json(http: &reqwest::Client, url: &str) -> Result<Value, String> {
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Steam Web API request failed: {}", e.without_url()))?;
    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Steam Web API key was rejected".into());
    }
    if !status.is_success() {
        return Err(format!("Steam Web API returned HTTP {}", status.as_u16()));
    }
    resp.json::<Value>()
        .await
        .map_err(|e| format!("Steam Web API body read failed: {}", e.without_url()))
}

async fn copy_dir_files(from: &Path, to: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(from).await else {
        return;
    };
    let _ = tokio::fs::create_dir_all(to).await;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let target = to.join(entry.file_name());
        if !target.exists() {
            let _ = tokio::fs::copy(entry.path(), target).await;
        }
    }
}

async fn write_text(path: PathBuf, text: String, label: &str) -> Result<(), String> {
    tokio::fs::write(&path, text)
        .await
        .map_err(|e| format!("write {}: {}", label, e))
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default()
}

pub async fn generate(
    http: &reqwest::Client,
    session: Arc<SteamSession>,
    targets: &[String],
    app_id: &str,
    opts: Options<'_>,
) -> Result<GameDataResult, String> {
    let dirs = settings_dirs(targets);
    if dirs.is_empty() {
        return Err("no patched library to write game data next to".into());
    }
    let app_id = app_id.trim();
    let app_num: u32 = app_id
        .parse()
        .map_err(|_| format!("Invalid app id '{}'", app_id))?;
    let key = opts.web_api_key.map(str::trim).filter(|k| !k.is_empty());
    let mut result = GameDataResult {
        source: "none".into(),
        ..Default::default()
    };

    let app = match steam_pics::fetch_app_info(session.clone(), app_num).await {
        Ok(app) => Some(app),
        Err(_) => {
            result.notes.push("gameData.appInfoFailed".into());
            None
        }
    };
    let languages = app.as_ref().map(steam_pics::supported_languages).unwrap_or_default();
    let root = find_game_root(targets);
    let mut depots: Vec<String> = root
        .as_ref()
        .map(|r| {
            crate::services::install_state::installed(r)
                .into_iter()
                .map(|d| d.depot_id)
                .collect()
        })
        .unwrap_or_default();
    if depots.is_empty() {
        depots = app.as_ref().map(steam_pics::depot_ids).unwrap_or_default();
    }
    let branches = app.as_ref().map(steam_pics::branches).unwrap_or_default();
    let (save_files, overrides) = app.as_ref().map(steam_pics::cloud_saves).unwrap_or_default();
    let win_dirs = cloud::dirs_for("Windows", &save_files, &overrides);
    let linux_dirs = cloud::dirs_for("Linux", &save_files, &overrides);

    let mut fetch_langs = languages.clone();
    if fetch_langs.is_empty() {
        fetch_langs.push("english".into());
    }
    if let Some(pos) = fetch_langs.iter().position(|l| l == opts.language) {
        let preferred = fetch_langs.remove(pos);
        fetch_langs.insert(0, preferred);
    } else if !opts.language.is_empty() && languages.is_empty() {
        fetch_langs.insert(0, opts.language.to_string());
    }
    let fetched = achievements::fetch(http, app_id, &fetch_langs, key, &mut result.notes).await;
    let mut achievement_list = fetched.achievements;
    achievements::add_percentages(http, app_id, &mut achievement_list).await;
    if achievement_list.is_empty() && key.is_none() {
        result.notes.push("gameData.needsKey".into());
    }
    let stats = fetched.stats;

    let boards = leaderboards::fetch(http, app_id, &mut result.notes).await;
    let inv = inventory::fetch(http, &session, app_num, key, &mut result.notes).await;
    let controller_files = match app.as_ref().map(steam_pics::controller_configs) {
        Some(configs) => controller::fetch(http, &session, &configs, &mut result.notes).await,
        None => Vec::new(),
    };

    let first = dirs[0].clone();
    for dir in &dirs {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| format!("create steam_settings: {}", e))?;
        if !languages.is_empty() {
            write_text(dir.join("supported_languages.txt"), languages.join("\n") + "\n", "supported_languages.txt").await?;
        }
        if !depots.is_empty() {
            write_text(dir.join("depots.txt"), depots.join("\n") + "\n", "depots.txt").await?;
        }
        if !branches.is_empty() {
            write_text(dir.join("branches.json"), pretty(&serde_json::to_value(&branches).unwrap_or_default()), "branches.json").await?;
        }
        if !achievement_list.is_empty() {
            write_text(dir.join("achievements.json"), pretty(&achievements_json(&achievement_list)), "achievements.json").await?;
        }
        if !stats.is_empty() {
            write_text(dir.join("stats.json"), pretty(&stats_json(&stats)), "stats.json").await?;
        }
        if !boards.is_empty() {
            write_text(dir.join("leaderboards.txt"), leaderboards::render(&boards), "leaderboards.txt").await?;
        }
        if let Some(inv) = &inv {
            write_text(dir.join("items.json"), pretty(&inv.items), "items.json").await?;
            write_text(dir.join("default_items.json"), pretty(&inv.defaults), "default_items.json").await?;
        }
        if !controller_files.is_empty() {
            let cdir = dir.join(CONTROLLER_DIR);
            tokio::fs::create_dir_all(&cdir)
                .await
                .map_err(|e| format!("create controller folder: {}", e))?;
            for (name, text) in &controller_files {
                write_text(cdir.join(name), text.clone(), name).await?;
            }
        }
        let ini_path = dir.join("configs.app.ini");
        let existing = tokio::fs::read_to_string(&ini_path).await.unwrap_or_default();
        let merged = cloud::merge_into_app_ini(&existing, &win_dirs, &linux_dirs);
        if merged != existing && !(merged.is_empty() && existing.is_empty()) {
            write_text(ini_path, merged, "configs.app.ini").await?;
        }
    }
    if !achievement_list.is_empty() {
        result.icons = achievements::download_icons(http, app_id, &achievement_list, &first).await;
        for dir in dirs.iter().skip(1) {
            copy_dir_files(&first.join(achievements::ICON_DIR), &dir.join(achievements::ICON_DIR)).await;
        }
        if let Some(schema_root) = watcher::schema_root() {
            let name = app.as_ref().and_then(|a| steam_pics::common_value(a, "name")).unwrap_or_default();
            let exe = app.as_ref().and_then(steam_pics::launch_exe).unwrap_or_default();
            let icon = app.as_ref().and_then(|a| steam_pics::common_value(a, "icon"));
            let meta = watcher::AppMeta { app_id, name: &name, exe: &exe, icon_hash: icon.as_deref() };
            result.watcher_schemas = watcher::write(&schema_root, &meta, &achievement_list).await;
        }
    }
    if opts.media != MediaLevel::Off {
        let media_root = root.clone().or_else(|| first.parent().map(Path::to_path_buf)).unwrap_or_else(|| first.clone());
        let common = |k: &str| app.as_ref().and_then(|a| steam_pics::common_value(a, k));
        let hashes = media::CommunityHashes {
            clienticon: common("clienticon"),
            icon: common("icon"),
            logo: common("logo"),
            logo_small: common("logo_small"),
        };
        let icons = inv.as_ref().map(|i| i.icons.clone()).unwrap_or_default();
        result.media_files = media::download(http, &media_root, app_id, opts.media, &hashes, &icons).await;
        if result.media_files == 0 {
            result.notes.push("gameData.mediaFailed".into());
        }
    }

    result.languages = languages.len();
    result.depots = depots.len();
    result.branches = branches.len();
    result.achievements = achievement_list.len();
    result.achievement_languages = if achievement_list.is_empty() { 0 } else { fetched.languages };
    result.stats = stats.len();
    result.leaderboards = boards.len();
    result.items = inv.as_ref().map(|i| i.count).unwrap_or(0);
    result.controller_sets = controller_files.len();
    result.cloud_dirs = win_dirs.len() + linux_dirs.len();
    result.source = fetched.source.into();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_cannot_leave_the_folder() {
        assert_eq!(safe_file_name("https://x/y/../../evil.exe"), Some("evil.exe".into()));
        assert_eq!(safe_file_name("https://x/y/a%2Fb.jpg"), None);
        assert_eq!(safe_file_name("https://x/y/"), None);
        assert_eq!(safe_file_name(".."), None);
        assert_eq!(safe_file_name("InGame.txt"), Some("InGame.txt".into()));
    }

    #[test]
    fn finds_the_game_root_from_a_nested_library() {
        let root = std::env::temp_dir().join(format!("smd-gamedata-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let bin = root.join("bin").join("win64");
        std::fs::create_dir_all(&bin).unwrap();
        crate::services::install_state::save(
            &root,
            &crate::services::install_state::DepotInstall { depot_id: "221".into(), manifest_id: "1".into(), files: vec![] },
        )
        .unwrap();
        let target = bin.join("steam_api64.dll").to_string_lossy().to_string();
        assert_eq!(find_game_root(&[target.clone()]), Some(root.clone()));
        assert_eq!(settings_dirs(&[target]), vec![bin.join("steam_settings")]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
