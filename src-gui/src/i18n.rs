use std::collections::HashMap;
use std::fmt::Display;

const EN: &str = include_str!("../../locales/en.json");
const DE: &str = include_str!("../../locales/de.json");

pub const FALLBACK: &str = "en";
pub const AVAILABLE: [(&str, &str); 2] = [("en", "English"), ("de", "Deutsch")];

fn flatten(prefix: &str, value: &serde_json::Value, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                flatten(&key, v, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        _ => {}
    }
}

fn load(source: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(source) {
        flatten("", &value, &mut out);
    }
    out
}

fn source_of(code: &str) -> &'static str {
    match code {
        "de" => DE,
        _ => EN,
    }
}

struct Pattern {
    key: String,
    matcher: regex::Regex,
}

fn backend_patterns() -> &'static [Pattern] {
    static PATTERNS: std::sync::OnceLock<Vec<Pattern>> = std::sync::OnceLock::new();
    PATTERNS.get_or_init(|| {
        let placeholder = regex::Regex::new(r"\{\d+\}").expect("placeholder pattern");
        let mut out: Vec<(usize, Pattern)> = load(EN)
            .into_iter()
            .filter_map(|(key, text)| {
                let short = key.strip_prefix("backend.")?;
                if short.contains('.') {
                    return None;
                }
                let parts: Vec<&str> = placeholder.split(&text).collect();
                let body = parts.iter().map(|part| regex::escape(part)).collect::<Vec<_>>().join("([\\s\\S]*?)");
                let weight = parts.iter().map(|part| part.chars().count()).sum();
                let matcher = regex::Regex::new(&format!("^{}$", body)).ok()?;
                Some((weight, Pattern { key: short.to_string(), matcher }))
            })
            .collect();
        out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.key.cmp(&b.1.key)));
        out.into_iter().map(|(_, pattern)| pattern).collect()
    })
}

pub fn error_key(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    backend_patterns().iter().find(|p| p.matcher.is_match(text)).map(|p| format!("backend.{}", p.key))
}

pub fn is_available(code: &str) -> bool {
    AVAILABLE.iter().any(|(c, _)| *c == code)
}

#[derive(Clone, Debug)]
pub struct I18n {
    code: String,
    current: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18n {
    pub fn new(code: &str) -> Self {
        let code = if is_available(code) { code } else { FALLBACK };
        Self {
            code: code.to_string(),
            current: load(source_of(code)),
            fallback: if code == FALLBACK { HashMap::new() } else { load(source_of(FALLBACK)) },
        }
    }

    pub fn localize_error(&self, text: &str) -> String {
        self.localize_error_at(text, 0)
    }

    fn localize_error_at(&self, text: &str, depth: usize) -> String {
        if text.is_empty() || self.code == FALLBACK || depth > 4 {
            return text.to_string();
        }
        for pattern in backend_patterns() {
            let Some(found) = pattern.matcher.captures(text) else { continue };
            let Some(target) = self.current.get(&format!("backend.{}", pattern.key)) else { return text.to_string() };
            let mut out = String::new();
            let mut rest = target.as_str();
            while let Some(open) = rest.find('{') {
                let Some(close) = rest[open..].find('}') else { break };
                let inner = &rest[open + 1..open + close];
                out.push_str(&rest[..open]);
                match inner.parse::<usize>() {
                    Ok(index) => out.push_str(&self.localize_error_at(found.get(index + 1).map(|m| m.as_str()).unwrap_or(""), depth + 1)),
                    Err(_) => out.push_str(&rest[open..open + close + 1]),
                }
                rest = &rest[open + close + 1..];
            }
            out.push_str(rest);
            return out;
        }
        text.to_string()
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn has(&self, key: &str) -> bool {
        self.current.contains_key(key) || self.fallback.contains_key(key)
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.current.get(key).or_else(|| self.fallback.get(key)).map(String::as_str).unwrap_or(key)
    }

    pub fn tp(&self, key: &str, params: &[(&str, &dyn Display)]) -> String {
        let mut value = self.t(key).to_string();
        for (name, v) in params {
            value = value.replace(&format!("{{{}}}", name), &v.to_string());
        }
        value
    }
}
