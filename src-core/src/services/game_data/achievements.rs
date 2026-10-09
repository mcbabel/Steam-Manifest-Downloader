use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::{get_json, safe_file_name, API, COMMUNITY_IMAGES};

pub const ICON_DIR: &str = "img";
const ICON_CONCURRENCY: usize = 8;
const LANGUAGE_CONCURRENCY: usize = 4;
const MAX_LANGUAGES: usize = 32;
const ICON_MIRRORS: [&str; 2] = [
    "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps",
    "https://shared.akamai.steamstatic.com/community_assets/images/apps",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Achievement {
    pub name: String,
    pub display_name: BTreeMap<String, String>,
    pub description: BTreeMap<String, String>,
    pub hidden: bool,
    pub icon: String,
    pub icon_gray: String,
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Stat {
    pub name: String,
    pub default: f64,
}

pub struct Fetched {
    pub achievements: Vec<Achievement>,
    pub stats: Vec<Stat>,
    pub source: &'static str,
    pub languages: usize,
}

fn number(v: &Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(0.0)
}

fn icon_url(app_id: &str, file: &str) -> String {
    if file.is_empty() || file.starts_with("http") {
        file.to_string()
    } else {
        format!("{}/{}/{}", COMMUNITY_IMAGES, app_id, file)
    }
}

pub fn parse_schema(body: &Value, lang: &str) -> (Vec<Achievement>, Vec<Stat>) {
    let stats_root = &body["game"]["availableGameStats"];
    let achievements = stats_root["achievements"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    let name = a["name"].as_str()?.to_string();
                    let display = a["displayName"].as_str().unwrap_or(&name).to_string();
                    Some(Achievement {
                        display_name: BTreeMap::from([(lang.to_string(), display)]),
                        description: BTreeMap::from([(lang.to_string(), a["description"].as_str().unwrap_or_default().to_string())]),
                        hidden: number(&a["hidden"]) > 0.0,
                        icon: a["icon"].as_str().unwrap_or_default().to_string(),
                        icon_gray: a["icongray"].as_str().unwrap_or_default().to_string(),
                        percent: None,
                        name,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let stats = stats_root["stats"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|s| {
                    Some(Stat {
                        name: s["name"].as_str()?.to_string(),
                        default: number(&s["defaultvalue"]),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (achievements, stats)
}

pub fn parse_player_service(body: &Value, app_id: &str, lang: &str) -> Vec<Achievement> {
    body["response"]["achievements"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| {
                    let name = a["internal_name"].as_str()?.to_string();
                    let display = a["localized_name"].as_str().unwrap_or(&name).to_string();
                    Some(Achievement {
                        display_name: BTreeMap::from([(lang.to_string(), display)]),
                        description: BTreeMap::from([(lang.to_string(), a["localized_desc"].as_str().unwrap_or_default().to_string())]),
                        hidden: a["hidden"].as_bool().unwrap_or(false),
                        icon: icon_url(app_id, a["icon"].as_str().unwrap_or_default()),
                        icon_gray: icon_url(app_id, a["icon_gray"].as_str().unwrap_or_default()),
                        percent: a["player_percent_unlocked"]
                            .as_str()
                            .and_then(|p| p.parse().ok())
                            .or_else(|| a["player_percent_unlocked"].as_f64()),
                        name,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn parse_percentages(body: &Value) -> HashMap<String, f64> {
    body["achievementpercentages"]["achievements"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| Some((a["name"].as_str()?.to_string(), number(&a["percent"]))))
                .collect()
        })
        .unwrap_or_default()
}

pub fn merge_language(base: &mut [Achievement], other: Vec<Achievement>) -> bool {
    let mut by_name: HashMap<String, Achievement> = other.into_iter().map(|a| (a.name.clone(), a)).collect();
    let mut any = false;
    for a in base.iter_mut() {
        if let Some(o) = by_name.remove(&a.name) {
            a.display_name.extend(o.display_name);
            a.description.extend(o.description);
            any = true;
        }
    }
    any
}

fn localized(map: &BTreeMap<String, String>) -> Value {
    if map.len() == 1 {
        Value::String(map.values().next().cloned().unwrap_or_default())
    } else {
        Value::Object(map.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect())
    }
}

pub fn pick_language<'a>(map: &'a BTreeMap<String, String>, lang: &str) -> &'a str {
    map.get(lang)
        .or_else(|| map.iter().find(|(k, _)| k.eq_ignore_ascii_case(lang)).map(|(_, v)| v))
        .or_else(|| map.get("english"))
        .or_else(|| map.values().next())
        .map(|s| s.as_str())
        .unwrap_or("")
}

pub fn achievements_json(list: &[Achievement]) -> Value {
    let local = |url: &str| {
        safe_file_name(url)
            .map(|f| format!("{}/{}", ICON_DIR, f))
            .unwrap_or_default()
    };
    Value::Array(
        list.iter()
            .map(|a| {
                let mut entry = serde_json::json!({
                    "name": a.name,
                    "defaultvalue": 0,
                    "displayName": localized(&a.display_name),
                    "description": localized(&a.description),
                    "hidden": if a.hidden { 1 } else { 0 },
                    "icon": local(&a.icon),
                    "icongray": local(&a.icon_gray),
                    "icon_gray": local(&a.icon_gray),
                });
                if let Some(p) = a.percent {
                    entry["unlock_percentage"] = serde_json::json!((p.clamp(0.0, 100.0) * 10.0).round() / 10.0);
                }
                entry
            })
            .collect(),
    )
}

pub fn stats_json(list: &[Stat]) -> Value {
    Value::Array(
        list.iter()
            .map(|s| {
                let is_float = s.default.fract() != 0.0;
                let default = if is_float {
                    s.default.to_string()
                } else {
                    format!("{}", s.default as i64)
                };
                serde_json::json!({
                    "name": s.name,
                    "type": if is_float { "float" } else { "int" },
                    "default": default,
                    "global": "0",
                })
            })
            .collect(),
    )
}

fn schema_url(app_id: &str, lang: &str, key: &str) -> String {
    format!(
        "{}/ISteamUserStats/GetSchemaForGame/v2/?key={}&appid={}&l={}",
        API, key, app_id, lang
    )
}

fn keyless_url(app_id: &str, lang: &str) -> String {
    format!(
        "{}/IPlayerService/GetGameAchievements/v1/?appid={}&language={}",
        API, app_id, lang
    )
}

async fn other_languages<F, Fut>(langs: &[String], base: &mut [Achievement], fetch: F) -> usize
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Vec<Achievement>>,
{
    let results: Vec<Vec<Achievement>> = stream::iter(langs.iter().skip(1).cloned())
        .map(fetch)
        .buffer_unordered(LANGUAGE_CONCURRENCY)
        .collect()
        .await;
    1 + results
        .into_iter()
        .filter(|list| merge_language(base, list.clone()))
        .count()
}

pub async fn fetch(
    http: &reqwest::Client,
    app_id: &str,
    langs: &[String],
    key: Option<&str>,
    notes: &mut Vec<String>,
) -> Fetched {
    let langs: Vec<String> = langs.iter().take(MAX_LANGUAGES).cloned().collect();
    let empty = Fetched { achievements: Vec::new(), stats: Vec::new(), source: "none", languages: 0 };
    let Some(first) = langs.first().cloned() else {
        return empty;
    };
    if let Some(key) = key.map(str::trim).filter(|k| !k.is_empty()) {
        match get_json(http, &schema_url(app_id, &first, key)).await {
            Ok(body) => {
                let (mut achievements, stats) = parse_schema(&body, &first);
                let languages = if achievements.is_empty() {
                    0
                } else {
                    other_languages(&langs, &mut achievements, |lang| async move {
                        match get_json(http, &schema_url(app_id, &lang, key)).await {
                            Ok(body) => parse_schema(&body, &lang).0,
                            Err(_) => Vec::new(),
                        }
                    })
                    .await
                };
                return Fetched { achievements, stats, source: "web_api", languages };
            }
            Err(e) if e.contains("rejected") => notes.push("gameData.keyRejected".into()),
            Err(_) => notes.push("gameData.webApiFailed".into()),
        }
    }
    let Ok(body) = get_json(http, &keyless_url(app_id, &first)).await else {
        return empty;
    };
    let mut achievements = parse_player_service(&body, app_id, &first);
    if achievements.is_empty() {
        return empty;
    }
    let languages = other_languages(&langs, &mut achievements, |lang| async move {
        match get_json(http, &keyless_url(app_id, &lang)).await {
            Ok(body) => parse_player_service(&body, app_id, &lang),
            Err(_) => Vec::new(),
        }
    })
    .await;
    Fetched { achievements, stats: Vec::new(), source: "keyless", languages }
}

pub async fn add_percentages(http: &reqwest::Client, app_id: &str, list: &mut [Achievement]) {
    if list.is_empty() || list.iter().any(|a| a.percent.is_some()) {
        return;
    }
    let url = format!(
        "{}/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={}",
        API, app_id
    );
    if let Ok(body) = get_json(http, &url).await {
        let percents = parse_percentages(&body);
        for a in list.iter_mut() {
            a.percent = percents.get(&a.name).copied();
        }
    }
}

async fn download_icon(http: &reqwest::Client, app_id: &str, url: &str, path: &Path) -> Option<()> {
    let file = safe_file_name(url)?;
    let mut candidates = vec![url.to_string()];
    candidates.extend(ICON_MIRRORS.iter().map(|m| format!("{}/{}/{}", m, app_id, file)));
    candidates.dedup();
    for candidate in candidates {
        let Ok(resp) = http.get(&candidate).send().await else { continue };
        let Ok(resp) = resp.error_for_status() else { continue };
        let Ok(bytes) = resp.bytes().await else { continue };
        if tokio::fs::write(path, &bytes).await.is_ok() {
            return Some(());
        }
    }
    None
}

pub async fn download_icons(http: &reqwest::Client, app_id: &str, list: &[Achievement], dir: &Path) -> usize {
    let img = dir.join(ICON_DIR);
    if tokio::fs::create_dir_all(&img).await.is_err() {
        return 0;
    }
    let mut urls: Vec<String> = list
        .iter()
        .flat_map(|a| [a.icon.clone(), a.icon_gray.clone()])
        .filter(|u| u.starts_with("https://"))
        .collect();
    urls.sort();
    urls.dedup();
    stream::iter(urls)
        .map(|url| {
            let img = img.clone();
            async move {
                let path = img.join(safe_file_name(&url)?);
                if path.is_file() {
                    return Some(());
                }
                download_icon(http, app_id, &url, &path).await
            }
        })
        .buffer_unordered(ICON_CONCURRENCY)
        .filter(|r| std::future::ready(r.is_some()))
        .count()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_web_api_schema() {
        let body = serde_json::json!({ "game": { "availableGameStats": {
            "achievements": [
                { "name": "ACH_WIN", "defaultvalue": 0, "displayName": "Winner", "hidden": 1, "description": "Win once",
                  "icon": "https://cdn.example/apps/1/abc123.jpg", "icongray": "https://cdn.example/apps/1/def456.jpg" }
            ],
            "stats": [ { "name": "kills", "defaultvalue": 0 }, { "name": "accuracy", "defaultvalue": "0.5" } ]
        } } });
        let (a, s) = parse_schema(&body, "english");
        assert_eq!(a.len(), 1);
        assert!(a[0].hidden);
        let json = achievements_json(&a);
        assert_eq!(json[0]["displayName"], "Winner");
        assert_eq!(json[0]["icon"], "img/abc123.jpg");
        assert_eq!(json[0]["icongray"], "img/def456.jpg");
        assert_eq!(json[0]["hidden"], 1);
        let stats = stats_json(&s);
        assert_eq!(stats[0]["type"], "int");
        assert_eq!(stats[0]["default"], "0");
        assert_eq!(stats[1]["type"], "float");
    }

    #[test]
    fn merges_languages_into_one_object() {
        let en = serde_json::json!({ "response": { "achievements": [
            { "internal_name": "A1", "localized_name": "First", "localized_desc": "Do it", "hidden": false,
              "icon": "aa11.jpg", "icon_gray": "bb22.jpg", "player_percent_unlocked": "12.345" }
        ] } });
        let de = serde_json::json!({ "response": { "achievements": [
            { "internal_name": "A1", "localized_name": "Erster", "localized_desc": "Mach es" }
        ] } });
        let mut list = parse_player_service(&en, "620", "english");
        assert_eq!(list[0].icon, format!("{}/620/aa11.jpg", COMMUNITY_IMAGES));
        assert!(merge_language(&mut list, parse_player_service(&de, "620", "german")));
        let json = achievements_json(&list);
        assert_eq!(json[0]["displayName"]["english"], "First");
        assert_eq!(json[0]["displayName"]["german"], "Erster");
        assert_eq!(json[0]["description"]["german"], "Mach es");
        assert_eq!(json[0]["unlock_percentage"], 12.3);
        assert_eq!(pick_language(&list[0].display_name, "German"), "Erster");
        assert_eq!(pick_language(&list[0].display_name, "french"), "First");
        let p = parse_percentages(&serde_json::json!({ "achievementpercentages": { "achievements": [ { "name": "A1", "percent": "40.5" } ] } }));
        assert_eq!(p.get("A1"), Some(&40.5));
    }
}
