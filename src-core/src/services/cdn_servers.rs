
use std::sync::RwLock;

use crate::services::steam_session::CdnServer;

pub const REGIONS: &[(u32, &str)] = &[
    (63, "USA – Washington, DC"),
    (50, "USA – Atlanta"),
    (1, "USA – Chicago"),
    (65, "USA – Dallas"),
    (64, "USA – Los Angeles"),
    (76, "USA – Minneapolis"),
    (31, "USA – Seattle"),
    (25, "Brazil – São Paulo"),
    (116, "Argentina – Buenos Aires"),
    (117, "Chile – Santiago"),
    (118, "Peru – Lima"),
    (4, "UK – London"),
    (15, "Netherlands – Amsterdam"),
    (14, "France – Paris"),
    (5, "Germany – Frankfurt"),
    (92, "Austria – Vienna"),
    (38, "Poland – Warsaw"),
    (66, "Sweden – Stockholm"),
    (127, "UAE – Dubai"),
    (26, "South Africa – Johannesburg"),
    (35, "Singapore"),
    (33, "Hong Kong"),
    (32, "Japan – Tokyo"),
    (52, "Australia – Sydney"),
];

pub fn region_name(cell: u32) -> Option<&'static str> {
    REGIONS.iter().find(|(c, _)| *c == cell).map(|(_, n)| *n)
}

const SPREAD: usize = 3;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ServerChoice {
    #[default]
    Auto,
    Region(u32),
    Custom(String),
}

impl ServerChoice {
    pub fn from_settings(region: u32, server: &str) -> Self {
        let server = server.trim();
        if !server.is_empty() {
            ServerChoice::Custom(server.to_string())
        } else if region != 0 {
            ServerChoice::Region(region)
        } else {
            ServerChoice::Auto
        }
    }

    pub fn cell(&self) -> Option<u32> {
        match self {
            ServerChoice::Region(c) => Some(*c),
            _ => None,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            ServerChoice::Auto => "auto",
            ServerChoice::Region(_) => "region",
            ServerChoice::Custom(_) => "custom",
        }
    }
}

pub fn parse_override(raw: &str) -> Result<ServerChoice, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
        return Ok(ServerChoice::Auto);
    }
    if let Ok(cell) = raw.parse::<u32>() {
        return match region_name(cell) {
            Some(_) => Ok(ServerChoice::Region(cell)),
            None => Err(format!("Unknown download region: {}", cell)),
        };
    }
    Ok(ServerChoice::Custom(parse_custom(raw)?))
}

static CHOICE: RwLock<Option<ServerChoice>> = RwLock::new(None);
static FELL_BACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn note_fallback() {
    FELL_BACK.store(true, std::sync::atomic::Ordering::Relaxed);
}

pub fn fell_back() -> bool {
    FELL_BACK.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_choice(choice: ServerChoice) {
    FELL_BACK.store(false, std::sync::atomic::Ordering::Relaxed);
    if let Ok(mut c) = CHOICE.write() {
        *c = Some(choice);
    }
}

pub fn choice() -> ServerChoice {
    CHOICE.read().ok().and_then(|c| c.clone()).unwrap_or_default()
}

pub fn parse_custom(raw: &str) -> Result<String, String> {
    let raw = raw.trim().trim_end_matches('/');
    if raw.is_empty() {
        return Ok(String::new());
    }
    let with_scheme = if raw.contains("://") { raw.to_string() } else { format!("http://{}", raw) };
    let url = reqwest::Url::parse(&with_scheme).map_err(|_| format!("Not a server address: {}", raw))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none_or(str::is_empty) {
        return Err(format!("Not a server address: {}", raw));
    }
    if url.path() != "/" || url.query().is_some() {
        return Err(format!("Only the server, without a path: {}", raw));
    }
    Ok(raw.to_string())
}

pub fn custom_server(raw: &str) -> CdnServer {
    let raw = raw.trim().trim_end_matches('/');
    let (https, host) = match raw.split_once("://") {
        Some((scheme, rest)) => (scheme.eq_ignore_ascii_case("https"), rest.to_string()),
        None => (raw.ends_with(".steamcontent.com"), raw.to_string()),
    };
    CdnServer {
        host,
        https_support: if https { "mandatory".into() } else { String::new() },
        cell_id: 0,
        weighted_load: 0.0,
        allowed_app_ids: Vec::new(),
    }
}

pub fn order(mut servers: Vec<CdnServer>, app_id: u32, choice: &ServerChoice) -> (Vec<CdnServer>, usize) {
    servers.retain(|s| s.allowed_app_ids.is_empty() || s.allowed_app_ids.contains(&app_id));
    servers.sort_by(|a, b| a.weighted_load.total_cmp(&b.weighted_load));
    let mut seen = std::collections::HashSet::new();
    servers.retain(|s| seen.insert(s.host.to_ascii_lowercase()));
    match choice {
        ServerChoice::Auto => {
            let spread = servers.len().clamp(1, SPREAD);
            (servers, spread)
        }
        ServerChoice::Region(cell) => {
            let (mut own, rest): (Vec<_>, Vec<_>) = servers.into_iter().partition(|s| s.cell_id == *cell);
            let spread = own.len().clamp(1, SPREAD);
            own.extend(rest);
            (own, spread)
        }
        ServerChoice::Custom(raw) => {
            let mine = custom_server(raw);
            servers.retain(|s| !s.host.eq_ignore_ascii_case(&mine.host));
            servers.insert(0, mine);
            (servers, 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srv(host: &str, cell: u32, load: f32, apps: &[u32]) -> CdnServer {
        CdnServer {
            host: host.into(),
            https_support: "mandatory".into(),
            cell_id: cell,
            weighted_load: load,
            allowed_app_ids: apps.to_vec(),
        }
    }

    fn hosts(v: &[CdnServer]) -> Vec<&str> {
        v.iter().map(|s| s.host.as_str()).collect()
    }

    #[test]
    fn auto_uses_the_lowest_load_first_and_skips_servers_for_other_apps() {
        let list = vec![srv("a", 5, 140.0, &[]), srv("b", 5, 30.0, &[]), srv("isp", 5, 1.0, &[730]), srv("c", 4, 60.0, &[])];
        let (out, spread) = order(list, 440, &ServerChoice::Auto);
        assert_eq!(hosts(&out), ["b", "c", "a"]);
        assert_eq!(spread, 3);
        let (out, _) = order(vec![srv("isp", 5, 1.0, &[730]), srv("a", 5, 9.0, &[])], 730, &ServerChoice::Auto);
        assert_eq!(hosts(&out), ["isp", "a"]);
    }

    #[test]
    fn a_region_goes_first_and_the_rest_stays_as_fallback() {
        let list = vec![srv("lon", 4, 10.0, &[]), srv("fra2", 5, 50.0, &[]), srv("fra1", 5, 40.0, &[])];
        let (out, spread) = order(list, 1, &ServerChoice::Region(5));
        assert_eq!(hosts(&out), ["fra1", "fra2", "lon"]);
        assert_eq!(spread, 2);
    }

    #[test]
    fn an_own_server_comes_first_alone() {
        let list = vec![srv("x.steamcontent.com", 5, 10.0, &[])];
        let (out, spread) = order(list, 1, &ServerChoice::Custom("lancache.lan".into()));
        assert_eq!(hosts(&out), ["lancache.lan", "x.steamcontent.com"]);
        assert_eq!(spread, 1);
        assert!(!out[0].use_https());
        assert!(custom_server("cache1-fra1.steamcontent.com").use_https());
        assert!(custom_server("https://my.cache:8443").use_https());
        assert_eq!(custom_server("http://10.0.0.5:8080/").host, "10.0.0.5:8080");
    }

    #[test]
    fn own_server_entries_are_checked() {
        assert_eq!(parse_custom(" lancache.lan ").unwrap(), "lancache.lan");
        assert_eq!(parse_custom("http://10.0.0.5:8080/").unwrap(), "http://10.0.0.5:8080");
        assert_eq!(parse_custom("").unwrap(), "");
        assert!(parse_custom("ftp://x").is_err());
        assert!(parse_custom("http://x/depot").is_err());
        assert!(parse_custom("not a host").is_err());
    }

    #[test]
    fn settings_map_to_a_choice() {
        assert_eq!(ServerChoice::from_settings(0, ""), ServerChoice::Auto);
        assert_eq!(ServerChoice::from_settings(5, " "), ServerChoice::Region(5));
        assert_eq!(ServerChoice::from_settings(5, "lan"), ServerChoice::Custom("lan".into()));
        assert!(REGIONS.iter().all(|(c, _)| region_name(*c).is_some()));
        assert_eq!(parse_override("auto").unwrap(), ServerChoice::Auto);
        assert_eq!(parse_override("5").unwrap(), ServerChoice::Region(5));
        assert!(parse_override("7").is_err());
        assert_eq!(parse_override("lancache.lan").unwrap(), ServerChoice::Custom("lancache.lan".into()));
    }
}
