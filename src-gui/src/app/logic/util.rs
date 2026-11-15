use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

pub use crate::dom::markup::escape as escape_html;

#[macro_export]
macro_rules! t {
    ($app:expr, $key:expr) => {
        $app.i18n.t($key).to_string()
    };
    ($app:expr, $key:expr, $($name:ident = $value:expr),+ $(,)?) => {
        $app.i18n.tp($key, &[$((stringify!($name), &$value as &dyn std::fmt::Display)),+])
    };
}

pub fn format_bytes(bytes: f64) -> Option<String> {
    if !(bytes > 0.0) {
        return None;
    }
    let gb = bytes / (1024.0 * 1024.0 * 1024.0);
    if gb >= 1.0 {
        return Some(format!("{:.2} GB", gb));
    }
    let mb = bytes / (1024.0 * 1024.0);
    if mb >= 1.0 {
        return Some(format!("{:.2} MB", mb));
    }
    Some(format!("{:.2} KB", bytes / 1024.0))
}

pub fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn file_name(path: &str) -> String {
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

pub fn extension(path: &str) -> String {
    path.rsplit('.').next().unwrap_or("").to_lowercase()
}

pub fn is_numeric_input(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

pub fn is_valid_app_id(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() || text.len() > 10 || text.starts_with('0') || !text.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    text.parse::<u64>().is_ok_and(|value| value <= 4_294_967_295)
}

pub fn parse_date(text: &str) -> Option<DateTime<Local>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(text) {
        return Some(parsed.with_timezone(&Local));
    }
    if let Ok(parsed) = DateTime::parse_from_rfc2822(text) {
        return Some(parsed.with_timezone(&Local));
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
            return Local.from_local_datetime(&naive).single();
        }
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d") {
        return chrono::Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0)?).with_timezone(&Local).into();
    }
    None
}

const MONTHS_EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn date_medium(locale: &str, date: &DateTime<Local>) -> String {
    use chrono::Datelike;
    if locale == "de" {
        format!("{:02}.{:02}.{}", date.day(), date.month(), date.year())
    } else {
        format!("{} {}, {}", MONTHS_EN[date.month0() as usize], date.day(), date.year())
    }
}

pub fn time_short(locale: &str, date: &DateTime<Local>) -> String {
    use chrono::Timelike;
    if locale == "de" {
        format!("{:02}:{:02}", date.hour(), date.minute())
    } else {
        let (pm, hour) = date.hour12();
        format!("{}:{:02} {}", hour, date.minute(), if pm { "PM" } else { "AM" })
    }
}

pub fn date_time_medium_short(locale: &str, text: &str) -> String {
    match parse_date(text) {
        Some(date) => format!("{}, {}", date_medium(locale, &date), time_short(locale, &date)),
        None => text.to_string(),
    }
}

pub fn icon(name: &str) -> String {
    format!(r#"<svg class="btn-icon" data-icon="{}"></svg>"#, name)
}

pub const ICON_UPLOAD: &str = "folder";
pub const ICON_FOLDER_OPEN: &str = "folder_open";
pub const ICON_REFRESH: &str = "refresh";
pub const ICON_UPDATE: &str = "arrow_up_circle";
pub const ICON_TRASH: &str = "trash";
pub const ICON_X: &str = "x";
pub const ICON_CHECK: &str = "check";
pub const ICON_CHECK_CIRCLE: &str = "check_circle";
pub const ICON_ALERT_TRIANGLE: &str = "alert_triangle";
pub const ICON_SETTINGS: &str = "settings_alt";
pub const ICON_PLAY: &str = "play";
pub const ICON_GAMEPAD: &str = "gamepad";
pub const ICON_SHIELD_CHECK: &str = "shield_check";

pub fn str_of(value: &serde_json::Value, key: &str) -> String {
    match &value[key] {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::Bool(flag) => flag.to_string(),
        _ => String::new(),
    }
}

pub fn opt_str(value: &serde_json::Value, key: &str) -> Option<String> {
    let text = str_of(value, key);
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes_and_checks_app_ids() {
        assert_eq!(format_bytes(0.0), None);
        assert_eq!(format_bytes(1536.0).as_deref(), Some("1.50 KB"));
        assert_eq!(format_bytes(5.0 * 1024.0 * 1024.0 * 1024.0).as_deref(), Some("5.00 GB"));
        assert!(is_valid_app_id("480"));
        assert!(!is_valid_app_id("0480"));
        assert!(!is_valid_app_id("4294967296"));
        assert!(is_numeric_input(" 12 "));
        assert_eq!(file_name("C:\\a\\b.lua"), "b.lua");
        assert_eq!(extension("x/y.LUA"), "lua");
        assert_eq!(capitalize("windows"), "Windows");
    }
}
