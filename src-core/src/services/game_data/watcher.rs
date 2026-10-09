use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::achievements::{pick_language, Achievement};
use super::COMMUNITY_IMAGES;

const STORE_IMAGES: &str = "https://cdn.akamai.steamstatic.com/steam/apps";

pub struct AppMeta<'a> {
    pub app_id: &'a str,
    pub name: &'a str,
    pub exe: &'a str,
    pub icon_hash: Option<&'a str>,
}

pub fn schema_root() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let dir = dirs::config_dir()?.join("Achievement Watcher");
    dir.is_dir().then(|| dir.join("steam_cache").join("schema"))
}

pub fn languages(list: &[Achievement]) -> Vec<String> {
    let mut langs: BTreeSet<String> = list
        .iter()
        .flat_map(|a| a.display_name.keys().chain(a.description.keys()).cloned())
        .filter(|l| !l.eq_ignore_ascii_case("token"))
        .collect();
    if langs.is_empty() {
        langs.insert("english".into());
    }
    langs.into_iter().collect()
}

pub fn schema(meta: &AppMeta, lang: &str, list: &[Achievement]) -> Value {
    let full = |u: &str| {
        if u.is_empty() || u.starts_with("http") {
            u.to_string()
        } else {
            format!("{}/{}/{}", COMMUNITY_IMAGES, meta.app_id, u)
        }
    };
    let achievements: Vec<Value> = list
        .iter()
        .map(|a| {
            serde_json::json!({
                "displayName": pick_language(&a.display_name, lang),
                "description": pick_language(&a.description, lang),
                "name": a.name,
                "hidden": if a.hidden { 1 } else { 0 },
                "icon": full(&a.icon),
                "icongray": full(&a.icon_gray),
            })
        })
        .collect();
    let icon = meta
        .icon_hash
        .map(|h| format!("{}/{}/{}.jpg", COMMUNITY_IMAGES, meta.app_id, h))
        .unwrap_or_default();
    serde_json::json!({
        "appid": meta.app_id.parse::<u64>().unwrap_or(0),
        "name": meta.name,
        "binary": meta.exe,
        "achievement": { "total": list.len(), "list": achievements },
        "img": {
            "header": format!("{}/{}/header.jpg", STORE_IMAGES, meta.app_id),
            "background": format!("{}/{}/page_bg_generated_v6b.jpg", STORE_IMAGES, meta.app_id),
            "portrait": format!("{}/{}/library_600x900.jpg", STORE_IMAGES, meta.app_id),
            "hero": format!("{}/{}/library_hero.jpg", STORE_IMAGES, meta.app_id),
            "icon": icon,
        },
        "apiVersion": 1,
    })
}

pub async fn write(root: &Path, meta: &AppMeta<'_>, list: &[Achievement]) -> usize {
    let mut written = 0;
    for lang in languages(list) {
        if !lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        let dir = root.join(&lang);
        if tokio::fs::create_dir_all(&dir).await.is_err() {
            continue;
        }
        let text = serde_json::to_string_pretty(&schema(meta, &lang, list)).unwrap_or_default();
        if tokio::fs::write(dir.join(format!("{}.db", meta.app_id)), text).await.is_ok() {
            written += 1;
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn writes_one_schema_per_language() {
        let a = Achievement {
            name: "A1".into(),
            display_name: BTreeMap::from([("english".into(), "First".into()), ("german".into(), "Erster".into())]),
            description: BTreeMap::from([("english".into(), "Do it".into())]),
            hidden: true,
            icon: "https://cdn/x/aa.jpg".into(),
            icon_gray: "bb.jpg".into(),
            percent: None,
        };
        assert_eq!(languages(std::slice::from_ref(&a)), vec!["english", "german"]);
        let meta = AppMeta { app_id: "620", name: "Portal 2", exe: "portal2.exe", icon_hash: Some("ic") };
        let s = schema(&meta, "german", std::slice::from_ref(&a));
        assert_eq!(s["appid"], 620);
        assert_eq!(s["achievement"]["list"][0]["displayName"], "Erster");
        assert_eq!(s["achievement"]["list"][0]["description"], "Do it");
        assert_eq!(s["achievement"]["list"][0]["icongray"], format!("{}/620/bb.jpg", COMMUNITY_IMAGES));
        assert_eq!(s["img"]["icon"], format!("{}/620/ic.jpg", COMMUNITY_IMAGES));
    }
}
