use serde_json::{json, Value};

use super::util::str_of;
use super::ui;
use crate::app::App;
use crate::t;

const ISSUE_URL: &str = "https://github.com/MCbabel/Steam-Manifest-Downloader/issues/new";
const ISSUE_URL_LIMIT: usize = 7000;
const LOG_ROW: &str = "settings-row-2";

#[derive(Default)]
pub struct Prefill {
    pub source: &'static str,
    pub what: String,
    pub log: bool,
}

fn field(info: &Value, key: &str, fallback: &str) -> String {
    super::util::opt_str(info, key).unwrap_or_else(|| fallback.to_string())
}

pub fn copy_build_info(app: &mut App) {
    let Some(info) = app.state.settings.build_info.clone() else { return };
    let diagnostic = super::util::opt_str(&info, "diagnosticId");
    let text = [
        format!("Channel: {}", str_of(&info, "channel")),
        format!("Version: {}", str_of(&info, "version")),
        format!("Commit: {}", str_of(&info, "gitSha")),
        format!("Build date: {}", str_of(&info, "buildDate")),
        format!("Profile: {}", str_of(&info, "profile")),
        format!("Platform: {}/{}", str_of(&info, "targetOs"), str_of(&info, "targetArch")),
        format!("Package: {}", field(&info, "package", "unknown")),
        format!("Engine: {}", field(&info, "engine", "unknown")),
        format!("Language: {}", app.i18n.code()),
        format!("Diagnostic ID: {}", diagnostic.clone().unwrap_or_else(|| app.i18n.t("settings.diagnosticIdOff").to_string())),
    ]
    .join("\n");
    if diagnostic.is_some() {
        app.emit_event("diagnostics_copied", Value::Null);
    }
    app.ctx.copy_text(text);
    let copied = t!(app, "common.copied");
    app.ui.set_text("btn-copy-build-info", copied);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(1500).await;
        tasks.post(|app| app.ui.clear_content("btn-copy-build-info"));
    });
}

fn diagnostic_lines(app: &App, info: &Value) -> Vec<String> {
    vec![
        format!("App version: {} ({}, {})", field(info, "version", "unknown"), field(info, "channel", "unknown"), field(info, "gitSha", "unknown")),
        format!("Platform: {}/{}, {}", field(info, "targetOs", "?"), field(info, "targetArch", "?"), field(info, "package", "unknown")),
        format!("Engine: {}", field(info, "engine", "unknown")),
        format!("Language: {}", app.i18n.code()),
        format!("Diagnostic ID: {}", field(info, "diagnosticId", "statistics off")),
    ]
}

fn recent_log_lines(app: &App, max: usize) -> Vec<String> {
    let lines = &app.state.terminal;
    lines[lines.len().saturating_sub(max)..].iter().map(|(_, text)| text.clone()).collect()
}

fn with_build_info(app: &mut App, then: impl FnOnce(&mut App, Value) + Send + 'static) {
    if let Some(info) = app.state.settings.build_info.clone() {
        return then(app, info);
    }
    app.tasks.call("get_build_info", Value::Null, move |app, info: Result<Value, String>| {
        let info = info.unwrap_or_else(|_| json!({}));
        app.state.settings.build_info = Some(info.clone());
        then(app, info);
    });
}

pub fn open_bug_report(app: &mut App, prefill: Prefill) {
    app.ui.set_value("bug-title", "");
    app.ui.set_value("bug-what", prefill.what);
    app.ui.set_value("bug-steps", "");
    app.ui.set_value("bug-expected", "");
    for id in ["bug-title", "bug-what", "bug-steps", "bug-expected"] {
        app.ui.remove_class(id, "is-invalid");
    }
    let has_log = !app.state.terminal.is_empty();
    app.ui.set_checked("bug-include-log", prefill.log && has_log);
    app.ui.set_disabled("bug-include-log", !has_log);
    app.ui.set_class(LOG_ROW, "is-disabled", !has_log);
    app.ui.set_checked("bug-include-diag", true);
    app.ui.hide("bug-report-error");
    app.state.bug_report_source = if prefill.source.is_empty() { "manual" } else { prefill.source };
    with_build_info(app, |app, info| {
        let preview = diagnostic_lines(app, &info).join(" · ");
        app.ui.set_text("bug-diag-preview", preview);
        app.ui.show("bug-report-modal");
        app.ui.request_focus("bug-title");
    });
}

fn close_bug_report(app: &mut App) {
    app.ui.hide("bug-report-modal");
}

fn form_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => out.push(byte as char),
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

struct Parts {
    what: String,
    steps: String,
    expected: String,
    diag: Option<Vec<String>>,
    log: Option<Vec<String>>,
}

fn issue_url(info: &Value, title: &str, parts: &Parts) -> String {
    let os = match info["targetOs"].as_str() {
        Some("windows") => "Windows",
        Some("linux") => "Linux",
        Some("macos") => "macOS",
        _ => "",
    };
    let fields = [
        ("template", "bug_report.yml".to_string()),
        ("title", format!("[Bug] {title}")),
        ("what", parts.what.trim().to_string()),
        ("steps", parts.steps.trim().to_string()),
        ("expected", parts.expected.trim().to_string()),
        ("version", str_of(info, "version")),
        ("os", os.to_string()),
        ("environment", parts.diag.as_ref().map(|lines| lines.join("\n")).unwrap_or_default()),
        ("logs", parts.log.as_ref().map(|lines| lines.join("\n")).unwrap_or_default()),
        ("extra", "Sent from the app's bug report form.".to_string()),
    ];
    let query: Vec<String> = fields.iter().filter(|(_, value)| !value.is_empty()).map(|(key, value)| format!("{key}={}", form_encode(value))).collect();
    format!("{ISSUE_URL}?{}", query.join("&"))
}

fn shorten(text: &mut String) {
    let keep = (text.chars().count() as f64 * 0.75).floor() as usize;
    *text = text.chars().take(keep).collect::<String>() + " …";
}

fn submit_bug_report(app: &mut App) {
    let title = app.ui.value("bug-title").trim().to_string();
    let what = app.ui.value("bug-what").to_string();
    app.ui.set_class("bug-title", "is-invalid", title.is_empty());
    app.ui.set_class("bug-what", "is-invalid", what.trim().is_empty());
    if title.is_empty() || what.trim().is_empty() {
        let required = t!(app, "bugReport.required");
        app.ui.set_text("bug-report-error", required);
        app.ui.show("bug-report-error");
        return app.ui.request_focus(if title.is_empty() { "bug-title" } else { "bug-what" });
    }
    with_build_info(app, move |app, info| {
        let with_diag = app.ui.checked("bug-include-diag");
        let with_log = app.ui.checked("bug-include-log");
        let mut parts = Parts {
            what,
            steps: app.ui.value("bug-steps").to_string(),
            expected: app.ui.value("bug-expected").to_string(),
            diag: with_diag.then(|| diagnostic_lines(app, &info)),
            log: with_log.then(|| recent_log_lines(app, 60)),
        };
        let mut url = issue_url(&info, &title, &parts);
        while url.len() > ISSUE_URL_LIMIT && parts.log.as_ref().is_some_and(|log| log.len() > 5) {
            if let Some(log) = parts.log.as_mut() {
                let cut = log.len().div_ceil(3);
                log.drain(..cut);
            }
            url = issue_url(&info, &title, &parts);
        }
        for key in 0..3 {
            loop {
                let text = match key {
                    0 => &mut parts.what,
                    1 => &mut parts.steps,
                    _ => &mut parts.expected,
                };
                if url.len() <= ISSUE_URL_LIMIT || text.chars().count() <= 200 {
                    break;
                }
                shorten(text);
                url = issue_url(&info, &title, &parts);
            }
        }
        app.open_url(&url);
        app.emit_event("bug_report_opened", json!({ "source": app.state.bug_report_source, "diag": with_diag, "log": with_log }));
        close_bug_report(app);
        let opened = t!(app, "bugReport.opened");
        ui::show_toast(app, opened, "success", 6000);
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-report-bug" => open_bug_report(app, Prefill::default()),
        "btn-settings-report-bug" => open_bug_report(app, Prefill { source: "settings", ..Default::default() }),
        "btn-report-failure" => {
            let what = app.state.last_failure_text.clone().map(|text| format!("{}\n\n{text}", app.i18n.t("bugReport.failedPrefix"))).unwrap_or_default();
            open_bug_report(app, Prefill { source: "download_failed", what, log: true });
        }
        "btn-bug-cancel" | "bug-report-modal>modal__backdrop" => close_bug_report(app),
        "btn-bug-open" => submit_bug_report(app),
        "btn-copy-build-info" => copy_build_info(app),
        _ => return false,
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    match id {
        "bug-title" | "bug-what" => {
            if !app.ui.value(id).trim().is_empty() {
                app.ui.remove_class(id, "is-invalid");
            }
            if !app.ui.has_class("bug-title", "is-invalid") && !app.ui.has_class("bug-what", "is-invalid") {
                app.ui.hide("bug-report-error");
            }
        }
        "bug-steps" | "bug-expected" => {}
        _ => return false,
    }
    true
}

pub fn changed(_app: &mut App, id: &str) -> bool {
    matches!(id, "bug-include-diag" | "bug-include-log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_prefilled_issue_link() {
        assert_eq!(form_encode("a b&c/ä"), "a+b%26c%2F%C3%A4");
        let parts = Parts { what: " crash ".to_string(), steps: String::new(), expected: String::new(), diag: Some(vec!["x".to_string(), "y".to_string()]), log: None };
        let url = issue_url(&json!({ "version": "1.5.0", "targetOs": "linux" }), "Title", &parts);
        assert_eq!(url, "https://github.com/MCbabel/Steam-Manifest-Downloader/issues/new?template=bug_report.yml&title=%5BBug%5D+Title&what=crash&version=1.5.0&os=Linux&environment=x%0Ay&extra=Sent+from+the+app%27s+bug+report+form.");
        let mut long = "abcdefgh".to_string();
        shorten(&mut long);
        assert_eq!(long, "abcdef …");
    }
}
