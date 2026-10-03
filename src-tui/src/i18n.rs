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
        s = s.replace(&format!("{{{}}}", name), &value.to_string());
    }
    s
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
}
