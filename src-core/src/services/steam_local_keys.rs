//! Depot keys from the installed Steam's config/config.vdf. Read only, never stored or sent.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalKeys {
    pub steam_found: bool,
    pub keys: HashMap<String, String>,
}

pub async fn read(depot_ids: &[String]) -> LocalKeys {
    let wanted: Vec<String> = depot_ids.to_vec();
    tokio::task::spawn_blocking(move || read_blocking(&wanted))
        .await
        .unwrap_or_default()
}

fn read_blocking(wanted: &[String]) -> LocalKeys {
    let Some(path) = config_path() else {
        return LocalKeys::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return LocalKeys::default();
    };
    let mut keys = parse_keys(&text);
    keys.retain(|id, _| wanted.iter().any(|w| w == id));
    LocalKeys { steam_found: true, keys }
}

fn config_path() -> Option<PathBuf> {
    crate::services::steam_library::steam_candidates()
        .into_iter()
        .map(|dir| dir.join("config").join("config.vdf"))
        .find(|p| p.is_file())
}

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Str(&'a str),
    Open,
    Close,
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                out.push(Token::Open);
                i += 1;
            }
            b'}' => {
                out.push(Token::Close);
                i += 1;
            }
            b'"' => {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end] != b'"' {
                    end += if bytes[end] == b'\\' { 2 } else { 1 };
                }
                let end = end.min(bytes.len());
                out.push(Token::Str(&text[start..end]));
                i = end + 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

pub fn parse_keys(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut path: Vec<&str> = Vec::new();
    let toks = tokens(text);
    let mut i = 0;
    while i < toks.len() {
        match (&toks[i], toks.get(i + 1)) {
            (Token::Str(name), Some(Token::Open)) => {
                path.push(name);
                i += 2;
            }
            (Token::Str(key), Some(Token::Str(value))) => {
                let in_depot = path.len() >= 2
                    && path[path.len() - 2].eq_ignore_ascii_case("depots")
                    && path[path.len() - 1].bytes().all(|b| b.is_ascii_digit());
                if in_depot && key.eq_ignore_ascii_case("DecryptionKey") && is_key(value) {
                    out.insert(path[path.len() - 1].to_string(), value.to_ascii_lowercase());
                }
                i += 2;
            }
            (Token::Close, _) => {
                path.pop();
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

fn is_key(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

    #[test]
    fn reads_depot_keys_from_config_vdf() {
        let text = format!(
            r#""InstallConfigStore"
{{
	"Software"
	{{
		"Valve"
		{{
			"Steam"
			{{
				"CellID"		"1"
				"depots"
				{{
					"1001"
					{{
						"DecryptionKey"		"{KEY}"
					}}
					"221"
					{{
						"DecryptionKey"		"not-a-key"
					}}
				}}
				"Accounts" {{ "someone" {{ "SteamID" "7656119" }} }}
			}}
		}}
	}}
}}"#
        );
        let keys = parse_keys(&text);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys.get("1001").map(String::as_str), Some(KEY));
    }

    #[test]
    fn ignores_keys_outside_the_depots_block() {
        let text = format!(r#""Steam" {{ "apps" {{ "5" {{ "DecryptionKey" "{KEY}" }} }} }}"#);
        assert!(parse_keys(&text).is_empty());
    }
}
