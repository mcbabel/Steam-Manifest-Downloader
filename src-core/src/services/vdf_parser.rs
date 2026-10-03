use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

const SEAN_WHO_XOR_KEY: &[u8] = b"Scalping dogs, I'll fuck you";

// Matches a depot block of the form:
//     "1995891"
//     {
//         "DecryptionKey" "hexvalue"
//     }
fn depot_block_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r#"(?si)"(\d+)"\s*\{[^}]*"DecryptionKey"\s+"([^"]+)"[^}]*\}"#)
            .expect("depot-block pattern is a valid regex")
    })
}

// `repo` only matters for sean-who, whose keys are XOR-obfuscated with SEAN_WHO_XOR_KEY.
pub fn parse_key_vdf(vdf_content: &str, repo: Option<&str>) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for cap in depot_block_pattern().captures_iter(vdf_content) {
        let depot_id = cap[1].to_string();
        let mut depot_key = cap[2].to_string();

        if let Some(r) = repo {
            if r.contains("sean-who") {
                depot_key = xor_decrypt_hex(&depot_key, SEAN_WHO_XOR_KEY);
            }
        }

        result.insert(depot_id, depot_key);
    }

    result
}

pub fn xor_decrypt_hex(hex_string: &str, xor_key: &[u8]) -> String {
    let bytes = match hex_decode(hex_string) {
        Some(b) => b,
        None => return hex_string.to_string(),
    };

    let result: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ xor_key[i % xor_key.len()])
        .collect();

    hex_encode(&result)
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}