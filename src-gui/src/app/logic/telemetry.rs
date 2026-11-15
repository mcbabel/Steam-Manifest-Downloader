use std::hash::{BuildHasher, Hasher};
use std::sync::OnceLock;

use regex::Regex;
use serde_json::{json, Map, Value};

use super::ui::{self, Next};
use super::journey;
use crate::app::App;
use crate::i18n;
use crate::t;

pub fn init_telemetry_consent(app: &mut App, next: Next) {
    let parked = app.state.park(next);
    app.tasks.call("get_telemetry_status", Value::Null, move |app, status: Result<Value, String>| {
        let Some(next) = app.state.unpark(parked) else { return };
        let Ok(status) = status else { return next(app) };
        match status["consent"].as_str() {
            Some("pending") => {
                app.state.boot_next = Some(next);
                app.ui.show("telemetry-modal");
            }
            Some("accepted") => {
                app.emit_event("app_start", json!({ "locale": app.i18n.code(), "ui": "native" }));
                match status["behavior"].as_str() {
                    Some("pending") => app.ui.show("behavior-modal"),
                    Some("accepted") => journey::start_journey(app),
                    _ => {}
                }
                next(app);
            }
            _ => next(app),
        }
    });
}

fn close_consent(app: &mut App) {
    app.ui.hide("telemetry-modal");
    if let Some(next) = app.state.boot_next.take() {
        next(app);
    }
}

pub fn accept_telemetry(app: &mut App, everything: bool) {
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let consent = tasks.invoke_raw("set_telemetry_consent", json!({ "accept": true })).await;
        let behavior = match consent {
            Ok(_) => tasks.invoke_raw("set_behavior_consent", json!({ "accept": everything })).await,
            Err(error) => Err(error),
        };
        tasks.post(move |app| {
            if behavior.is_ok() {
                app.emit_event("app_start", json!({ "locale": app.i18n.code(), "ui": "native" }));
                if everything {
                    journey::start_journey(app);
                }
                let thanks = t!(app, "modals.behavior.thanks");
                ui::show_toast(app, thanks, "success", 4000);
            }
            close_consent(app);
        });
    });
}

pub fn answer_behavior(app: &mut App, accept: bool) {
    app.tasks.call("set_behavior_consent", json!({ "accept": accept }), move |app, saved: Result<Value, String>| {
        if saved.is_ok() && accept {
            journey::start_journey(app);
            let thanks = t!(app, "modals.behavior.thanks");
            ui::show_toast(app, thanks, "success", 4000);
        }
        app.ui.hide("behavior-modal");
    });
}

pub fn decline_telemetry(app: &mut App) {
    app.tasks.call("set_telemetry_consent", json!({ "accept": false }), |app, _: Result<Value, String>| close_consent(app));
}

pub fn report_error(app: &mut App, area: &str, raw: Option<&str>, key: Option<&str>) {
    let key = key.map(str::to_string).or_else(|| raw.and_then(i18n::error_key)).unwrap_or_else(|| "unmatched".to_string());
    let id = format!("{area}:{key}");
    if app.state.reported_errors.contains(&id) || app.state.reported_errors.len() >= 40 {
        return;
    }
    app.state.reported_errors.insert(id);
    app.emit_event("error_shown", json!({ "area": area, "key": key }));
}

pub fn error_key_or_unmatched(error: &str) -> String {
    i18n::error_key(error).unwrap_or_else(|| "unmatched".to_string())
}

pub fn sync_telemetry_focus(app: &mut App, focused: bool) {
    if app.state.telemetry_focused == Some(focused) {
        return;
    }
    app.state.telemetry_focused = Some(focused);
    app.tasks.fire("set_telemetry_focus", json!({ "focused": focused }));
}

pub fn telemetry_nonce() -> String {
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    format!("{:016x}", hasher.finish())
}

pub fn classify_start_failure(error: &str) -> Value {
    if error.contains("Invalid App ID") || error.contains("Invalid depot ID") {
        return json!({ "fail_stage": "source_probe", "fail_class": "decode" });
    }
    if error.contains("Cannot create download directory") {
        return json!({ "fail_stage": "disk", "fail_class": "io" });
    }
    json!({ "fail_stage": "unknown", "fail_class": "unknown" })
}

pub fn merge(target: &mut Value, extra: Value) {
    if let (Some(target), Value::Object(extra)) = (target.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
}

pub fn merged(parts: impl IntoIterator<Item = Value>) -> Value {
    let mut out = Value::Object(Map::new());
    for part in parts {
        merge(&mut out, part);
    }
    out
}

pub fn job_context(app: &App) -> Value {
    let mut out = json!({ "job": app.state.dl_nonce, "source_count": app.state.dl_source_count, "had_mh_key": app.state.dl_had_mh_key });
    if let Some(context) = &app.state.dl_context {
        merge(&mut out, context.clone());
    }
    out
}

pub fn clear_job_telemetry_state(app: &mut App) {
    app.state.dl_nonce = None;
    app.state.dl_started_at = None;
    app.state.dl_stage = None;
    app.state.dl_depot_count = None;
    app.state.dl_source_count = None;
    app.state.dl_had_mh_key = false;
    app.state.dl_context = None;
}

pub fn duration_bucket(millis: u128) -> &'static str {
    match millis / 1000 {
        0..=4 => "<5s",
        5..=29 => "5-30s",
        30..=119 => "30s-2m",
        120..=599 => "2-10m",
        600..=3599 => "10-60m",
        _ => ">60m",
    }
}

pub fn elapsed_bucket(app: &App) -> &'static str {
    duration_bucket(app.state.dl_started_at.map(|at| at.elapsed().as_millis()).unwrap_or(0))
}

pub fn count_bucket(count: usize) -> &'static str {
    match count {
        0 => "0",
        1 => "1",
        2..=3 => "2-3",
        4..=10 => "4-10",
        _ => ">10",
    }
}

const EMU_FAIL_CLASSES: [&str; 9] =
    ["emu_binary_missing", "interfaces_failed", "backup_failed", "copy_failed", "settings_write_failed", "no_parent_dir", "folder_missing", "no_backup", "restore_failed"];

pub fn emu_entry_point(app: &App) -> &'static str {
    if app.state.emu_standalone {
        "standalone"
    } else if app.state.emu_edit_mode {
        "history"
    } else {
        "download"
    }
}

pub fn emu_platform_mix(targets: &[Value]) -> &'static str {
    let windows = targets.iter().any(|t| t["platform"] == "windows");
    let linux = targets.iter().any(|t| t["platform"] == "linux");
    match (windows, linux) {
        (true, true) => "mixed",
        (false, true) => "linux",
        (true, false) => "windows",
        _ => "none",
    }
}

pub fn emu_fail_class(results: &[Value]) -> Option<String> {
    let first = results.iter().find(|r| !r["success"].as_bool().unwrap_or(false))?;
    let class = first["failClass"].as_str().unwrap_or("");
    Some(if EMU_FAIL_CLASSES.contains(&class) { class.to_string() } else { "unknown".to_string() })
}

fn hint_patterns() -> &'static [(Regex, &'static str); 3] {
    static PATTERNS: OnceLock<[(Regex, &'static str); 3]> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            (Regex::new(r"(?i)os error 225|virus|unwanted software|unerwünschte software").expect("pattern"), "emulator.hintAntivirusFile"),
            (Regex::new(r"(?i)os error 5\b|os error 13\b|access is denied|zugriff verweigert|permission denied").expect("pattern"), "emulator.hintPermission"),
            (
                Regex::new(r"(?i)GitHub fetch failed|GitHub returned HTTP|download returned HTTP|error sending request|steamless download|not present in release").expect("pattern"),
                "emulator.hintDownload",
            ),
        ]
    })
}

pub fn emu_error_hint(app: &App, error: &str) -> String {
    hint_patterns().iter().find(|(pattern, _)| pattern.is_match(error)).map(|(_, key)| app.i18n.t(key).to_string()).unwrap_or_default()
}

pub fn with_emu_hint(app: &App, text: &str, sources: &[&str]) -> String {
    match sources.iter().map(|source| emu_error_hint(app, source)).find(|hint| !hint.is_empty()) {
        Some(hint) => format!("{text}\n\n{hint}"),
        None => text.to_string(),
    }
}

pub fn classify_emu_command_error(error: &str) -> &'static str {
    if error.contains("AV_BLOCKED") {
        "av_blocked"
    } else if error.contains("GitHub") {
        "release_fetch_failed"
    } else if error.contains("download returned HTTP") {
        "emu_download_failed"
    } else {
        "unknown"
    }
}

pub fn emu_outcome(success: usize, total: usize) -> &'static str {
    if total == 0 {
        "failed"
    } else if success == total {
        "complete"
    } else if success > 0 {
        "partial"
    } else {
        "failed"
    }
}

pub fn abandon_props(app: &App) -> Value {
    merged([
        job_context(app),
        json!({
            "outcome": "abandoned",
            "depots_total": app.state.dl_depot_count.unwrap_or(0),
            "last_stage": app.state.dl_stage.clone().unwrap_or_else(|| "unknown".to_string()),
            "duration_bucket": elapsed_bucket(app),
            "engine": app.state.current_engine.clone().unwrap_or_else(|| "native".to_string()),
        }),
    ])
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-telemetry-accept" => accept_telemetry(app, true),
        "btn-telemetry-basic" => accept_telemetry(app, false),
        "btn-telemetry-decline" => decline_telemetry(app),
        "btn-behavior-accept" => answer_behavior(app, true),
        "btn-behavior-decline" => answer_behavior(app, false),
        _ => return false,
    }
    true
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    let focused = ctx.input(|i| i.focused);
    sync_telemetry_focus(app, focused);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_match_the_dashboard_groups() {
        assert_eq!(duration_bucket(4_999), "<5s");
        assert_eq!(duration_bucket(5_000), "5-30s");
        assert_eq!(duration_bucket(3_600_000), ">60m");
        assert_eq!(count_bucket(0), "0");
        assert_eq!(count_bucket(3), "2-3");
        assert_eq!(count_bucket(11), ">10");
        assert_eq!(emu_outcome(0, 0), "failed");
        assert_eq!(emu_outcome(2, 2), "complete");
        assert_eq!(emu_outcome(1, 2), "partial");
        assert_eq!(telemetry_nonce().len(), 16);
        assert_eq!(classify_emu_command_error("GitHub returned HTTP 403"), "release_fetch_failed");
    }
}
