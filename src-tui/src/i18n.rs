use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use serde_json::Value;

const GUI_LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../../public/locales/en.json")),
    ("de", include_str!("../../public/locales/de.json")),
];
const TUI_LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../locales/en.json")),
    ("de", include_str!("../locales/de.json")),
];

pub const FALLBACK: &str = "en";
pub const LANGUAGES: &[(&str, &str)] = &[("en", "English"), ("de", "Deutsch")];

struct Tables {
    gui: HashMap<&'static str, Value>,
    tui: HashMap<&'static str, Value>,
}

static TABLES: OnceLock<Tables> = OnceLock::new();
static CURRENT: RwLock<&'static str> = RwLock::new(FALLBACK);

fn tables() -> &'static Tables {
    TABLES.get_or_init(|| {
        let parse = |list: &[(&'static str, &str)]| {
            list.iter()
                .map(|(code, raw)| (*code, serde_json::from_str(raw).unwrap_or(Value::Null)))
                .collect()
        };
        Tables {
            gui: parse(GUI_LOCALES),
            tui: parse(TUI_LOCALES),
        }
    })
}

pub fn normalize(code: &str) -> Option<&'static str> {
    let lower = code.trim().to_ascii_lowercase();
    let base = lower.split(['_', '-', '.']).next().unwrap_or("");
    LANGUAGES.iter().map(|(c, _)| *c).find(|c| *c == base)
}

pub fn detect_system_language() -> &'static str {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(v) = std::env::var(var) {
            if let Some(code) = normalize(&v) {
                return code;
            }
        }
    }
    FALLBACK
}

pub fn init_from_data_dir(dir: &std::path::Path) {
    let stored = std::fs::read_to_string(dir.join("settings.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("language").and_then(|l| l.as_str()).map(str::to_string))
        .unwrap_or_default();
    let lang = if stored.is_empty() {
        detect_system_language()
    } else {
        normalize(&stored).unwrap_or(FALLBACK)
    };
    set_language(lang);
}

pub fn set_language(code: &str) {
    let code = normalize(code).unwrap_or(FALLBACK);
    if let Ok(mut cur) = CURRENT.write() {
        *cur = code;
    }
}

pub fn language() -> &'static str {
    CURRENT.read().map(|c| *c).unwrap_or(FALLBACK)
}

fn lookup<'a>(root: &'a Value, key: &str) -> Option<&'a str> {
    let mut node = root;
    for part in key.split('.') {
        node = node.get(part)?;
    }
    node.as_str()
}

fn raw(key: &str) -> Option<&'static str> {
    let t = tables();
    let lang = language();
    let order = [
        t.tui.get(lang),
        t.gui.get(lang),
        t.tui.get(FALLBACK),
        t.gui.get(FALLBACK),
    ];
    order
        .into_iter()
        .flatten()
        .find_map(|root| lookup(root, key))
}

pub fn t(key: &str) -> String {
    match raw(key) {
        Some(s) => strip_html(s),
        None => key.to_string(),
    }
}

pub fn tf(key: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut s = t(key);
    for (name, value) in args {
        let value = value.to_string();
        let value = if matches!(*name, "message" | "error") {
            localize_error(&value)
        } else {
            value
        };
        s = s.replace(&format!("{{{}}}", name), &value);
    }
    s
}

struct Template {
    key: String,
    literals: Vec<String>,
}

static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();

fn templates() -> &'static [Template] {
    TEMPLATES.get_or_init(|| {
        let mut list: Vec<Template> = tables()
            .gui
            .get(FALLBACK)
            .and_then(|v| v.get("backend"))
            .and_then(|v| v.as_object())
            .map(|map| {
                map.iter()
                    .filter_map(|(key, text)| {
                        let text = text.as_str()?;
                        let mut literals = Vec::new();
                        let mut rest = text;
                        while let Some(start) = rest.find('{') {
                            let Some(len) = rest[start + 1..].find('}') else {
                                break;
                            };
                            literals.push(rest[..start].to_string());
                            rest = &rest[start + len + 2..];
                        }
                        literals.push(rest.to_string());
                        Some(Template {
                            key: key.clone(),
                            literals,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        list.sort_by_key(|t| std::cmp::Reverse(t.literals.iter().map(|l| l.len()).sum::<usize>()));
        list
    })
}

fn capture<'a>(literals: &[String], s: &'a str, caps: &mut Vec<&'a str>) -> bool {
    let Some((next, more)) = literals.split_first() else {
        return s.is_empty();
    };
    if more.is_empty() {
        if let Some(head) = s.strip_suffix(next.as_str()) {
            caps.push(head);
            return true;
        }
        return false;
    }
    for (idx, _) in s.match_indices(next.as_str()) {
        caps.push(&s[..idx]);
        if capture(more, &s[idx + next.len()..], caps) {
            return true;
        }
        caps.pop();
    }
    false
}

fn localize_depth(lang: &str, text: &str, depth: usize) -> String {
    if depth > 4 || text.is_empty() || lang == FALLBACK {
        return text.to_string();
    }
    for tpl in templates() {
        let Some(rest) = text.strip_prefix(tpl.literals[0].as_str()) else {
            continue;
        };
        let mut caps = Vec::new();
        if !capture(&tpl.literals[1..], rest, &mut caps) {
            continue;
        }
        let Some(target) = tables()
            .gui
            .get(lang)
            .and_then(|root| lookup(root, &format!("backend.{}", tpl.key)))
        else {
            return text.to_string();
        };
        let mut out = target.to_string();
        for (i, cap) in caps.iter().enumerate() {
            out = out.replace(&format!("{{{}}}", i), &localize_depth(lang, cap, depth + 1));
        }
        return out;
    }
    text.to_string()
}

pub fn localize_error(text: &str) -> String {
    localize_depth(language(), text, 0)
}

pub fn event_text(msg: &Value) -> String {
    if let Some(key) = msg.get("key").and_then(|k| k.as_str()) {
        if let Some(template) = raw(key) {
            let mut out = strip_html(template);
            if let Some(params) = msg.get("params").and_then(|p| p.as_object()) {
                for (name, value) in params {
                    let value = match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    let value = if matches!(name.as_str(), "error" | "reason") {
                        localize_error(&value)
                    } else {
                        value
                    };
                    out = out.replace(&format!("{{{}}}", name), &value);
                }
            }
            return out;
        }
    }
    localize_error(msg.get("message").and_then(|m| m.as_str()).unwrap_or(""))
}

pub fn strip_html(s: &str) -> String {
    if !s.contains('<') && !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for n in chars.by_ref() {
                if n == '>' {
                    break;
                }
                tag.push(n);
            }
            let name = tag.trim_start_matches('/').to_ascii_lowercase();
            if name.starts_with("br") {
                out.push('\n');
            }
        } else {
            out.push(c);
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markup() {
        assert_eq!(
            strip_html("Drop your <strong>.lua</strong> file<br>now &amp; here"),
            "Drop your .lua file\nnow & here"
        );
    }

    #[test]
    fn normalizes_locale_names() {
        assert_eq!(normalize("de_DE.UTF-8"), Some("de"));
        assert_eq!(normalize("en-US"), Some("en"));
        assert_eq!(normalize("fr_FR"), None);
    }

    #[test]
    fn every_tui_key_exists_in_both_languages() {
        fn keys(prefix: &str, v: &Value, out: &mut Vec<String>) {
            if let Some(map) = v.as_object() {
                for (k, x) in map {
                    let p = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    keys(&p, x, out);
                }
            } else {
                out.push(prefix.to_string());
            }
        }
        let t = tables();
        let mut en = Vec::new();
        let mut de = Vec::new();
        keys("", &t.tui["en"], &mut en);
        keys("", &t.tui["de"], &mut de);
        en.sort();
        de.sort();
        assert_eq!(en, de);
        assert!(!en.is_empty());
    }

    #[test]
    fn falls_back_to_gui_strings() {
        set_language("en");
        assert_eq!(t("common.next"), "Next");
        assert_eq!(
            tf("emulator.applySuccess", &[("count", &2), ("total", &3)]),
            "Patch applied successfully (2/3 files)."
        );
        assert_eq!(t("no.such.key"), "no.such.key");
    }

    #[test]
    fn translates_backend_errors() {
        assert_eq!(
            localize_depth("de", "Unsupported file type: .txt. Expected .lua or .st", 0),
            "Nicht unterstützter Dateityp: .txt. Erwartet wird .lua oder .st"
        );
        assert_eq!(
            localize_depth("de", "Failed to start DepotDownloaderMod for depot 7: Path not found: x", 0),
            "DepotDownloaderMod konnte für Depot 7 nicht gestartet werden: Pfad nicht gefunden: x"
        );
        assert_eq!(localize_depth("de", "something else", 0), "something else");
        assert_eq!(localize_depth("en", "Invalid path", 0), "Invalid path");
    }

    #[test]
    fn backend_templates_keep_their_placeholders() {
        let t = tables();
        let en = t.gui["en"]["backend"].as_object().unwrap();
        let de = t.gui["de"]["backend"].as_object().unwrap();
        let holes = |s: &str| {
            let mut v: Vec<String> = s
                .match_indices('{')
                .map(|(i, _)| s[i..].split('}').next().unwrap_or("").to_string())
                .collect();
            v.sort();
            v
        };
        for (key, text) in en {
            let other = de.get(key).and_then(|v| v.as_str()).unwrap_or_else(|| panic!("de misses backend.{}", key));
            assert_eq!(holes(text.as_str().unwrap()), holes(other), "backend.{}", key);
        }
    }
}
