use serde::{Deserialize, Serialize};

use crate::services::settings::Settings;
use crate::services::steam_pics::{DepotMetadata, DepotRole};

pub const STEAM_LANGUAGES: &[&str] = &[
    "english",
    "german",
    "french",
    "italian",
    "spanish",
    "latam",
    "schinese",
    "tchinese",
    "japanese",
    "koreana",
    "russian",
    "polish",
    "brazilian",
    "portuguese",
    "turkish",
    "ukrainian",
    "czech",
    "dutch",
    "danish",
    "finnish",
    "norwegian",
    "swedish",
    "hungarian",
    "romanian",
    "thai",
    "vietnamese",
    "greek",
    "bulgarian",
    "arabic",
    "indonesian",
];

pub const PLATFORMS: &[&str] = &["windows", "linux", "macos"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefs {
    pub platform: String,
    pub language: String,
    pub include_dlc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    OtherPlatform,
    Arch32,
    OtherLanguage,
    LowViolence,
    Dlc,
    OptionalDlc,
    Redistributable,
    NoKey,
}

impl SkipReason {
    pub fn key(self) -> &'static str {
        match self {
            SkipReason::OtherPlatform => "other_platform",
            SkipReason::Arch32 => "arch32",
            SkipReason::OtherLanguage => "other_language",
            SkipReason::LowViolence => "low_violence",
            SkipReason::Dlc => "dlc",
            SkipReason::OptionalDlc => "optional_dlc",
            SkipReason::Redistributable => "redistributable",
            SkipReason::NoKey => "no_key",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skipped {
    #[serde(rename = "depotId")]
    pub depot_id: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub selected: Vec<String>,
    pub skipped: Vec<Skipped>,
    pub platform: String,
    pub language: String,
    pub known: bool,
}

pub fn host_platform() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "windows"
    }
}

pub fn steam_language(code: &str) -> &'static str {
    let lower = code.trim().to_ascii_lowercase();
    if let Some(found) = STEAM_LANGUAGES.iter().find(|l| **l == lower) {
        return found;
    }
    match lower.split(['_', '-', '.']).next().unwrap_or("") {
        "de" => "german",
        "fr" => "french",
        "it" => "italian",
        "es" => "spanish",
        "ja" => "japanese",
        "ko" => "koreana",
        "ru" => "russian",
        "pl" => "polish",
        "pt" => "portuguese",
        "tr" => "turkish",
        "uk" => "ukrainian",
        "cs" => "czech",
        "nl" => "dutch",
        "zh" => "schinese",
        _ => "english",
    }
}

pub fn prefs_from_settings(settings: &Settings, ui_language: &str) -> Prefs {
    let platform = match settings.target_platform.as_str() {
        p if PLATFORMS.contains(&p) => p.to_string(),
        _ => host_platform().to_string(),
    };
    let language = if settings.game_language.trim().is_empty() {
        let ui = if settings.language.is_empty() {
            ui_language
        } else {
            &settings.language
        };
        steam_language(ui).to_string()
    } else {
        steam_language(&settings.game_language).to_string()
    };
    Prefs {
        platform,
        language,
        include_dlc: settings.include_dlc,
    }
}

fn os_list(meta: &DepotMetadata) -> Vec<String> {
    meta.oslist
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .map(|s| {
            if s == "mac" || s == "osx" {
                "macos".to_string()
            } else {
                s
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn runs_on(meta: &DepotMetadata, platform: &str) -> bool {
    let list = os_list(meta);
    list.is_empty() || list.iter().any(|o| o == platform)
}

fn language_of(meta: &DepotMetadata) -> Option<String> {
    meta.language
        .as_deref()
        .map(|l| l.trim().to_ascii_lowercase())
        .filter(|l| !l.is_empty())
}

pub fn recommend(meta: &[DepotMetadata], candidates: &[String], prefs: &Prefs) -> Selection {
    let find = |id: &str| meta.iter().find(|m| m.depot_id == id);
    let known: Vec<(&String, &DepotMetadata)> = candidates
        .iter()
        .filter_map(|id| find(id).map(|m| (id, m)))
        .collect();

    let platform = if known
        .iter()
        .any(|(_, m)| os_list(m).contains(&prefs.platform))
    {
        prefs.platform.clone()
    } else if known
        .iter()
        .any(|(_, m)| os_list(m).iter().any(|o| o == "windows"))
    {
        "windows".to_string()
    } else {
        known
            .iter()
            .find_map(|(_, m)| os_list(m).into_iter().next())
            .unwrap_or_else(|| prefs.platform.clone())
    };

    let on_platform = |m: &DepotMetadata| runs_on(m, &platform);
    let has_64 = known
        .iter()
        .any(|(_, m)| on_platform(m) && m.osarch.as_deref() == Some("64"));
    let languages: Vec<String> = known
        .iter()
        .filter(|(_, m)| on_platform(m))
        .filter_map(|(_, m)| language_of(m))
        .collect();
    let language = if languages.contains(&prefs.language) {
        prefs.language.clone()
    } else if languages.iter().any(|l| l == "english") || languages.is_empty() {
        "english".to_string()
    } else {
        languages[0].clone()
    };

    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    for id in candidates {
        let Some(m) = find(id) else {
            selected.push(id.clone());
            continue;
        };
        let reason = if m.shared_install {
            Some(SkipReason::Redistributable)
        } else if !on_platform(m) {
            Some(SkipReason::OtherPlatform)
        } else if has_64 && m.osarch.as_deref() == Some("32") {
            Some(SkipReason::Arch32)
        } else if language_of(m).is_some_and(|l| l != language) {
            Some(SkipReason::OtherLanguage)
        } else if m.low_violence {
            Some(SkipReason::LowViolence)
        } else if m.role == DepotRole::Dlc && m.optional_dlc {
            Some(SkipReason::OptionalDlc)
        } else if m.role == DepotRole::Dlc && !prefs.include_dlc {
            Some(SkipReason::Dlc)
        } else {
            None
        };
        match reason {
            Some(reason) => skipped.push(Skipped {
                depot_id: id.clone(),
                reason,
            }),
            None => selected.push(id.clone()),
        }
    }

    if selected.is_empty() && !candidates.is_empty() {
        return Selection {
            selected: candidates.to_vec(),
            skipped: Vec::new(),
            platform,
            language,
            known: false,
        };
    }

    Selection {
        selected,
        skipped,
        platform,
        language,
        known: !known.is_empty(),
    }
}

pub fn recommend_keyed(
    meta: &[DepotMetadata],
    candidates: &[String],
    has_key: impl Fn(&str) -> bool,
    prefs: &Prefs,
) -> Selection {
    let (keyed, keyless): (Vec<String>, Vec<String>) =
        candidates.iter().cloned().partition(|id| has_key(id));
    if keyed.is_empty() || keyless.is_empty() {
        return recommend(meta, candidates, prefs);
    }
    let mut selection = recommend(meta, &keyed, prefs);
    selection
        .skipped
        .extend(keyless.into_iter().map(|depot_id| Skipped {
            depot_id,
            reason: SkipReason::NoKey,
        }));
    selection
}

pub fn chosen_dlcs(
    meta: &[DepotMetadata],
    main_app_id: &str,
    all_app_ids: &[String],
    selected_depots: &[String],
    include_dlc: Option<bool>,
) -> Vec<String> {
    let mut chosen: Vec<String> = meta
        .iter()
        .filter(|m| selected_depots.contains(&m.depot_id))
        .filter_map(|m| m.dlc_app_id)
        .map(|id| id.to_string())
        .filter(|id| id != main_app_id)
        .collect();
    if include_dlc.unwrap_or(!chosen.is_empty()) {
        let with_depots: Vec<String> = meta
            .iter()
            .filter_map(|m| m.dlc_app_id)
            .map(|id| id.to_string())
            .collect();
        chosen.extend(
            all_app_ids
                .iter()
                .filter(|id| *id != main_app_id)
                .filter(|id| !with_depots.contains(id))
                .filter(|id| !selected_depots.contains(id))
                .cloned(),
        );
    }
    chosen.sort_by_key(|id| id.parse::<u64>().unwrap_or(u64::MAX));
    chosen.dedup();
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn depot(id: &str, role: DepotRole) -> DepotMetadata {
        DepotMetadata {
            depot_id: id.to_string(),
            name: None,
            dlc_app_id: None,
            oslist: None,
            osarch: None,
            language: None,
            manifest_gid: None,
            role,
            low_violence: false,
            optional_dlc: false,
            shared_install: false,
        }
    }

    fn platform(id: &str, os: &str, arch: Option<&str>) -> DepotMetadata {
        let mut d = depot(id, DepotRole::Platform);
        d.oslist = Some(os.to_string());
        d.osarch = arch.map(str::to_string);
        d
    }

    fn language(id: &str, lang: &str) -> DepotMetadata {
        let mut d = depot(id, DepotRole::Language);
        d.language = Some(lang.to_string());
        d
    }

    fn prefs(platform: &str, language: &str) -> Prefs {
        Prefs {
            platform: platform.to_string(),
            language: language.to_string(),
            include_dlc: true,
        }
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn reason(sel: &Selection, id: &str) -> Option<SkipReason> {
        sel.skipped
            .iter()
            .find(|s| s.depot_id == id)
            .map(|s| s.reason)
    }

    #[test]
    fn picks_host_platform_and_language_like_steam() {
        let meta = vec![
            depot("11", DepotRole::SharedContent),
            platform("12", "windows", Some("64")),
            platform("13", "windows", Some("32")),
            platform("14", "linux", None),
            language("15", "german"),
            language("16", "french"),
        ];
        let sel = recommend(
            &meta,
            &ids(&["11", "12", "13", "14", "15", "16"]),
            &prefs("windows", "german"),
        );
        assert_eq!(sel.selected, ids(&["11", "12", "15"]));
        assert_eq!(reason(&sel, "13"), Some(SkipReason::Arch32));
        assert_eq!(reason(&sel, "14"), Some(SkipReason::OtherPlatform));
        assert_eq!(reason(&sel, "16"), Some(SkipReason::OtherLanguage));
        assert_eq!(sel.platform, "windows");
        assert_eq!(sel.language, "german");
    }

    #[test]
    fn skips_depots_without_a_key() {
        let meta = vec![
            depot("1", DepotRole::SharedContent),
            platform("2", "windows", Some("64")),
            platform("3", "windows", Some("64")),
        ];
        let sel = recommend_keyed(
            &meta,
            &ids(&["1", "2", "3"]),
            |id| id != "3",
            &prefs("windows", "english"),
        );
        assert_eq!(sel.selected, ids(&["1", "2"]));
        assert_eq!(reason(&sel, "3"), Some(SkipReason::NoKey));
    }

    #[test]
    fn keeps_all_depots_when_none_has_a_key() {
        let meta = vec![
            depot("1", DepotRole::SharedContent),
            platform("2", "windows", None),
        ];
        let sel = recommend_keyed(&meta, &ids(&["1", "2"]), |_| false, &prefs("windows", "english"));
        assert_eq!(sel.selected, ids(&["1", "2"]));
        assert!(sel.skipped.is_empty());
    }

    #[test]
    fn linux_without_native_build_falls_back_to_windows() {
        let meta = vec![
            depot("1", DepotRole::SharedContent),
            platform("2", "windows", None),
            platform("3", "macos", None),
        ];
        let sel = recommend(&meta, &ids(&["1", "2", "3"]), &prefs("linux", "english"));
        assert_eq!(sel.platform, "windows");
        assert_eq!(sel.selected, ids(&["1", "2"]));
    }

    #[test]
    fn linux_build_is_preferred_on_linux() {
        let meta = vec![platform("2", "windows", None), platform("3", "linux", None)];
        let sel = recommend(&meta, &ids(&["2", "3"]), &prefs("linux", "english"));
        assert_eq!(sel.selected, ids(&["3"]));
    }

    #[test]
    fn missing_language_falls_back_to_english() {
        let meta = vec![language("5", "english"), language("6", "french")];
        let sel = recommend(&meta, &ids(&["5", "6"]), &prefs("windows", "german"));
        assert_eq!(sel.language, "english");
        assert_eq!(sel.selected, ids(&["5"]));
    }

    #[test]
    fn dlc_rules_and_special_depots() {
        let mut dlc = depot("20", DepotRole::Dlc);
        dlc.dlc_app_id = Some(99);
        let mut optional = dlc.clone();
        optional.depot_id = "21".to_string();
        optional.optional_dlc = true;
        let mut gore = depot("22", DepotRole::SharedContent);
        gore.low_violence = true;
        let mut redist = depot("23", DepotRole::SharedContent);
        redist.shared_install = true;
        let meta = vec![
            depot("1", DepotRole::SharedContent),
            dlc,
            optional,
            gore,
            redist,
        ];
        let all = ids(&["1", "20", "21", "22", "23", "77"]);
        let sel = recommend(&meta, &all, &prefs("windows", "english"));
        assert_eq!(sel.selected, ids(&["1", "20", "77"]));
        assert_eq!(reason(&sel, "21"), Some(SkipReason::OptionalDlc));
        assert_eq!(reason(&sel, "22"), Some(SkipReason::LowViolence));
        assert_eq!(reason(&sel, "23"), Some(SkipReason::Redistributable));
        let mut no_dlc = prefs("windows", "english");
        no_dlc.include_dlc = false;
        let sel = recommend(&meta, &all, &no_dlc);
        assert_eq!(reason(&sel, "20"), Some(SkipReason::Dlc));
    }

    #[test]
    fn unknown_metadata_keeps_everything() {
        let sel = recommend(&[], &ids(&["1", "2"]), &prefs("windows", "english"));
        assert_eq!(sel.selected, ids(&["1", "2"]));
        assert!(!sel.known);
    }

    #[test]
    fn maps_ui_languages() {
        assert_eq!(steam_language("de"), "german");
        assert_eq!(steam_language("en-US"), "english");
        assert_eq!(steam_language("schinese"), "schinese");
        assert_eq!(steam_language(""), "english");
    }

    #[test]
    fn dlcs_follow_the_selection() {
        let mut dlc_a = depot("31", DepotRole::Dlc);
        dlc_a.dlc_app_id = Some(300);
        let mut dlc_b = depot("41", DepotRole::Dlc);
        dlc_b.dlc_app_id = Some(400);
        let meta = vec![depot("11", DepotRole::SharedContent), dlc_a, dlc_b];
        let all = ids(&["10", "300", "400", "500", "600"]);
        assert!(chosen_dlcs(&meta, "10", &all, &ids(&["11"]), None).is_empty());
        assert!(chosen_dlcs(&meta, "10", &all, &ids(&["11"]), Some(false)).is_empty());
        assert_eq!(
            chosen_dlcs(&meta, "10", &all, &ids(&["11", "31"]), None),
            ids(&["300", "500", "600"])
        );
        assert_eq!(
            chosen_dlcs(&meta, "10", &all, &ids(&["11", "31", "41"]), Some(true)),
            ids(&["300", "400", "500", "600"])
        );
        assert_eq!(chosen_dlcs(&meta, "10", &all, &ids(&["11", "31"]), Some(false)), ids(&["300"]));
    }
}
