use std::collections::VecDeque;
use std::time::Instant;

use serde_json::{json, Value};

use super::util::{escape_html, format_bytes, icon, str_of, ICON_ALERT_TRIANGLE, ICON_CHECK_CIRCLE, ICON_X};
use super::{download_control, emulator, onboarding, queue, select, settings, shortcuts, steps, telemetry, ui};
use crate::app::state::Action;
use crate::app::App;
use crate::dom::{el, Dom};
use crate::t;

const PIPELINE_STEPS: [&str; 13] = [
    "checking_branch",
    "branch_found",
    "downloading_keyvdf",
    "downloading_manifests",
    "downloading_manifest",
    "downloading_manifest_hub",
    "manifest_hub_rate_limited",
    "generating_keys",
    "keys_generated",
    "starting_downloader",
    "running_downloader",
    "paused",
    "resumed",
];
const SPEED_WINDOW_MS: u128 = 3000;
const SPEED_MIN_WINDOW_MS: u128 = 1500;
const ETA_DISPLAY_INTERVAL_MS: u128 = 1500;
const ADVICE_IDS: [&str; 10] = ["no_api_key", "no_sources", "app_missing", "no_depot_key", "account_no_depot_key", "manifest_missing", "rate_limited", "network", "disk", "unknown"];
const TERMINAL_ELEMENTS: usize = 300;

#[derive(Clone, Debug)]
pub struct DepotProgress {
    pub depot_id: String,
    pub status: &'static str,
    pub text: String,
}

#[derive(Default)]
pub struct SpeedTracker {
    pub last_percent: f64,
    pub last_time: Option<Instant>,
    pub samples: VecDeque<(f64, Instant)>,
    pub byte_samples: VecDeque<(f64, f64, Instant)>,
    pub current_depot_size: u64,
    pub stale: bool,
    pub last_update: Option<Instant>,
    pub last_display: Option<Instant>,
}

#[derive(Default)]
pub struct Taskbar {
    pub key: String,
    pub depot_index: u64,
    pub depot_total: u64,
    pub percent: Option<u8>,
}

fn set_bar(app: &mut App, id: &str, percent: f64) {
    app.ui.set_style(id, "width", format!("{}%", percent));
}

pub fn init_progress_ui(app: &mut App, depots: &[Value]) {
    set_bar(app, "progress-bar-fill", 0.0);
    let initializing = t!(app, "progress.initializing");
    app.ui.set_text("progress-status", initializing);
    app.state.terminal.clear();
    app.ui.hide("completion-message");
    ui::hide_next_steps(app, "failure-advice");
    app.ui.hide("feedback-prompt");
    app.state.download_failed = false;
    app.ui.hide("btn-next-step");
    app.ui.show("btn-cancel");
    app.ui.set_disabled("btn-cancel", false);
    let cancel = format!("{} <span>{}</span>", icon(ICON_X), escape_html(app.i18n.t("progress.cancel")));
    app.ui.set_html("btn-cancel", cancel);
    app.state.paused = false;
    app.state.last_skipped_shown = 0;
    reset_taskbar_progress(app);
    set_taskbar_progress(app, Some(0.0));
    let native = app.state.current_engine.as_deref() == Some("native");
    app.ui.set_hidden("btn-pause", !native);
    let pause = t!(app, "progress.pause");
    app.ui.set_text("btn-pause", pause);
    app.ui.set_disabled("btn-pause", false);
    app.ui.set_hidden("native-alpha-banner", !native);
    app.ui.hide("disk-space-info");
    set_bar(app, "depot-progress-fill", 0.0);
    app.ui.set_text("depot-progress-text", "0%");
    app.ui.hide("download-speed-info");
    app.state.speed.stale = false;
    let waiting = t!(app, "progress.depotWaiting");
    app.state.depot_progress = depots.iter().map(|depot| DepotProgress { depot_id: str_of(depot, "depotId"), status: "pending", text: waiting.clone() }).collect();
}

pub fn connect_progress_listener(app: &mut App) {
    app.state.progress_listening = true;
    let line = t!(app, "progress.connected");
    append_terminal_line(app, line, "info");
}

pub fn cleanup_progress_listener(app: &mut App) {
    app.state.progress_listening = false;
}

pub fn handle_progress_message(app: &mut App, msg: Value) {
    if !app.state.progress_listening {
        return;
    }
    let job = str_of(&msg, "jobId");
    if !job.is_empty() && app.state.job_id.is_none() && app.state.dl_started_at.is_some() {
        app.state.job_id = Some(job.clone());
    }
    if !job.is_empty() && app.state.job_id.as_deref() != Some(job.as_str()) {
        return;
    }
    match msg["type"].as_str().unwrap_or("") {
        "status" => {
            if msg["step"] == "disk_space" {
                download_control::show_disk_space(app, msg["freeGB"].as_f64().unwrap_or(0.0), &str_of(&msg, "drive"));
            } else {
                handle_status_update(app, &msg);
            }
        }
        "output" => handle_output(app, &msg),
        "depot_complete" => {
            let text = t!(app, "progress.depotComplete");
            update_depot_status(app, &str_of(&msg, "depotId"), "done", text);
            update_overall_progress(app, msg["current"].as_f64().unwrap_or(0.0), msg["total"].as_f64().unwrap_or(0.0));
            update_depot_download_progress(app, 100.0);
        }
        "manifest_source" => handle_manifest_source(app, &msg),
        "complete" => handle_complete(app, &msg),
        "error" => handle_error(app, &msg),
        "cancelled" => handle_cancelled(app, &msg),
        _ => {}
    }
}

fn handle_status_update(app: &mut App, msg: &Value) {
    let step = msg["step"].as_str().unwrap_or("");
    if PIPELINE_STEPS.contains(&step) {
        app.state.dl_stage = Some(step.to_string());
    }
    let depot_id = str_of(msg, "depotId");
    let native = app.state.current_engine.as_deref() == Some("native");
    match step {
        "checking_branch" => {
            let text = t!(app, "progress.checkingBranch", appId = str_of(msg, "appId"));
            app.ui.set_text("progress-status", text.clone());
            append_terminal_line(app, text, "info");
        }
        "branch_found" => {
            let text = if msg["key"].is_string() { event_text(app, msg) } else { t!(app, "progress.branchFound", info = str_of(msg, "lastUpdated")) };
            append_terminal_line(app, format!("✓ {text}"), "success");
        }
        "manifest_hub_rate_limited" | "retrying_depot" | "depot_up_to_date" | "removed_stale_files" => {
            if !str_of(msg, "message").is_empty() {
                let kind = if matches!(step, "manifest_hub_rate_limited" | "retrying_depot") { "warn" } else { "info" };
                let text = event_text(app, msg);
                append_terminal_line(app, text, kind);
            }
        }
        "downloading_manifests" => {
            let text = t!(app, "progress.manifestsStart", total = str_of(msg, "total"));
            app.ui.set_text("progress-status", text);
        }
        "downloading_manifest" => {
            let (current, total) = (msg["current"].as_f64().unwrap_or(0.0), msg["total"].as_f64().unwrap_or(0.0));
            if current > 0.0 && total > 0.0 {
                let text = t!(app, "progress.manifestOne", current = current, total = total, depotId = depot_id);
                app.ui.set_text("progress-status", text);
                update_overall_progress(app, current - 1.0, total * 2.0);
            }
            let status = t!(app, "progress.depotManifest");
            update_depot_status(app, &depot_id, "active", status);
            let file = str_of(msg, "filename");
            if !file.is_empty() {
                let line = t!(app, "progress.downloadingFile", file = file);
                append_terminal_line(app, line, "info");
            }
        }
        "downloading_manifest_hub" => {
            let manifest_id = str_of(msg, "manifestId");
            let text = t!(app, "progress.customManifest", depotId = depot_id);
            app.ui.set_text("progress-status", text);
            let status = t!(app, "progress.customManifestStatus", id = manifest_id);
            update_depot_status(app, &depot_id, "active", status);
            let line = t!(app, "progress.customManifestLine", depotId = depot_id, manifestId = manifest_id);
            append_terminal_line(app, line, "info");
        }
        "generating_keys" => {
            let text = t!(app, "progress.generatingKeys");
            app.ui.set_text("progress-status", text.clone());
            append_terminal_line(app, text, "info");
        }
        "keys_generated" => {
            let text = t!(app, "progress.keysGenerated", count = str_of(msg, "depotCount"));
            append_terminal_line(app, format!("✓ {text}"), "success");
        }
        "starting_downloader" => {
            let text = t!(app, if native { "progress.runningNativeStart" } else { "progress.runningDdmStart" }, total = str_of(msg, "total"));
            app.ui.set_text("progress-status", text);
        }
        "running_downloader" => {
            let now = Instant::now();
            app.state.speed.samples.clear();
            app.state.speed.byte_samples.clear();
            app.state.speed.last_time = None;
            app.state.speed.last_percent = 0.0;
            app.state.speed.last_update = Some(now);
            app.state.speed.stale = true;
            if !depot_id.is_empty() {
                let size = app.state.parsed_data.as_ref().and_then(|data| data.depots.iter().find(|d| d.depot_id == depot_id)).and_then(|d| d.size_bytes).unwrap_or(0);
                app.state.speed.current_depot_size = size;
            }
            let (current, total) = (msg["current"].as_u64().unwrap_or(0), msg["total"].as_u64().unwrap_or(0));
            if current > 0 && total > 0 {
                app.state.taskbar.depot_index = current;
                app.state.taskbar.depot_total = total;
                taskbar_depot_progress(app, 0.0);
                let text = t!(app, if native { "progress.runningNative" } else { "progress.runningDdm" }, current = current, total = total, depotId = depot_id);
                app.ui.set_text("progress-status", text);
                let base = if app.state.parsed_data.is_some() { app.state.selected_depots.len() as f64 } else { 0.0 };
                update_overall_progress(app, base + current as f64 - 1.0, base + total as f64);
            }
            let status = t!(app, "progress.depotDownloading");
            update_depot_status(app, &depot_id, "active", status);
            let command = str_of(msg, "command");
            if !command.is_empty() {
                let text = if msg["key"].is_string() { event_text(app, msg) } else { command };
                append_terminal_line(app, format!("> {text}"), "info");
            }
        }
        _ => {}
    }
}

pub fn event_text(app: &App, msg: &Value) -> String {
    if let Some(key) = msg["key"].as_str() {
        if app.i18n.has(key) {
            let mut text = app.i18n.t(key).to_string();
            if let Some(params) = msg["params"].as_object() {
                for (name, value) in params {
                    let raw = value.as_str().map(str::to_string).unwrap_or_else(|| value.to_string());
                    let shown = if name == "error" || name == "reason" { app.i18n.localize_error(&raw) } else { raw };
                    text = text.replace(&format!("{{{name}}}"), &shown);
                }
            }
            return text;
        }
    }
    app.i18n.localize_error(&str_of(msg, "message"))
}

fn handle_manifest_source(app: &mut App, msg: &Value) {
    let source = str_of(msg, "source");
    let label_key = format!("events.src.label.{source}");
    let label = if app.i18n.has(&label_key) { app.i18n.t(&label_key).to_string() } else if source.is_empty() { "?".to_string() } else { source.clone() };
    let text = t!(app, "events.line", label = label, depot = str_of(msg, "depotId"), text = event_text(app, msg));
    let kind = match source.as_str() {
        "manifesthub_fallback" => "warn",
        "manifesthub_unavailable" => "stderr",
        _ => "info",
    };
    append_terminal_line(app, text, kind);
    if source == "manifesthub_unavailable" {
        app.state.suggest_mh_key = true;
    }
}

fn handle_output(app: &mut App, msg: &Value) {
    let kind = if msg["stream"] == "stderr" { "stderr" } else { "stdout" };
    let (completed, total) = (msg["completedBytes"].as_f64(), msg["totalBytes"].as_f64());
    if let (Some(completed), Some(total)) = (completed, total) {
        if total > 0.0 {
            let percent = msg["percent"].as_f64().unwrap_or(completed * 100.0 / total);
            update_depot_download_progress_bytes(app, percent, completed, total, msg["networkBytes"].as_f64());
            let skipped = msg["skippedChunks"].as_u64().unwrap_or(0);
            if skipped > 0 {
                let milestone = skipped / 25;
                if milestone > app.state.last_skipped_shown {
                    app.state.last_skipped_shown = milestone;
                    let mb = format!("{:.1}", msg["skippedBytes"].as_f64().unwrap_or(0.0) / (1024.0 * 1024.0));
                    let line = t!(app, "progress.resumedChunks", count = skipped, mb = mb);
                    append_terminal_line(app, format!("✓ {line}"), "success");
                }
            }
            return;
        }
    }
    let text = super::util::opt_str(msg, "output").unwrap_or_else(|| str_of(msg, "line"));
    if text.is_empty() {
        return;
    }
    let percent = leading_percent(&text);
    append_terminal_line(app, text, kind);
    if let Some(percent) = percent {
        update_depot_download_progress(app, percent);
    }
}

fn leading_percent(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let end = trimmed.find('%')?;
    let number = &trimmed[..end];
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if whole.is_empty() || whole.len() > 3 || !digits(whole) || fraction.len() > 2 || !digits(fraction) || (number.contains('.') && fraction.is_empty()) {
        return None;
    }
    number.parse().ok()
}

pub fn set_taskbar_progress(app: &mut App, percent: Option<f64>) {
    let percent = percent.map(|p| p.clamp(0.0, 100.0).floor() as u8);
    app.state.taskbar.percent = percent;
    let paused = percent.is_some() && app.state.paused;
    let key = format!("{percent:?}|{paused}");
    if key == app.state.taskbar.key {
        return;
    }
    app.state.taskbar.key = key;
    let base = t!(app, "appName");
    let title = match percent {
        None => base,
        Some(p) => {
            let pause = if paused { format!(" ({})", app.i18n.t("progress.pausedShort")) } else { String::new() };
            let game = app.state.game_name.as_ref().map(|name| format!("{name} – ")).unwrap_or_default();
            format!("{p}%{pause} · {game}{base}")
        }
    };
    app.set_window_progress(percent, paused, title);
}

pub fn reset_taskbar_progress(app: &mut App) {
    app.state.taskbar.depot_index = 0;
    app.state.taskbar.depot_total = 0;
    set_taskbar_progress(app, None);
}

fn taskbar_depot_progress(app: &mut App, depot_percent: f64) {
    let total = match app.state.taskbar.depot_total {
        0 => app.state.dl_depot_count.unwrap_or(1).max(1) as f64,
        total => total as f64,
    };
    let done = app.state.taskbar.depot_index.saturating_sub(1) as f64;
    set_taskbar_progress(app, Some(((done + depot_percent.min(100.0) / 100.0) / total) * 100.0));
}

fn update_depot_download_progress_bytes(app: &mut App, percent: f64, completed: f64, total: f64, network: Option<f64>) {
    taskbar_depot_progress(app, percent);
    set_bar(app, "depot-progress-fill", percent.min(100.0));
    let size = match (format_bytes(completed), format_bytes(total)) {
        _ if total <= 0.0 => String::new(),
        (done, all) => format!(" — {} / {}", done.unwrap_or_else(|| "null".to_string()), all.unwrap_or_else(|| "null".to_string())),
    };
    app.ui.set_text("depot-progress-text", format!("{percent:.1}%{size}"));
    update_speed_and_eta_bytes(app, completed, total, network);
}

fn show_speed(app: &mut App, speed: String, eta: String) {
    app.ui.show("download-speed-info");
    app.ui.set_text("download-speed", speed);
    app.ui.set_text("download-eta", eta);
}

fn update_speed_and_eta_bytes(app: &mut App, completed: f64, total: f64, network: Option<f64>) {
    let now = Instant::now();
    let tracker = &mut app.state.speed;
    tracker.last_update = Some(now);
    tracker.byte_samples.push_back((network.unwrap_or(completed), completed, now));
    while tracker.byte_samples.len() > 2 && now.duration_since(tracker.byte_samples[0].2).as_millis() > SPEED_WINDOW_MS {
        tracker.byte_samples.pop_front();
    }
    if tracker.byte_samples.len() < 2 {
        return;
    }
    let oldest = tracker.byte_samples[0];
    let newest = tracker.byte_samples[tracker.byte_samples.len() - 1];
    let elapsed = newest.2.duration_since(oldest.2).as_millis();
    let network_delta = newest.0 - oldest.0;
    if elapsed < SPEED_MIN_WINDOW_MS || network_delta <= 0.0 {
        return;
    }
    let seconds = elapsed as f64 / 1000.0;
    let network_rate = network_delta / seconds;
    let unpacked_rate = (newest.1 - oldest.1) / seconds;
    let remaining = (total - completed).max(0.0);
    let eta = if unpacked_rate > 0.0 { remaining / unpacked_rate } else { f64::INFINITY };
    if tracker.last_display.is_some_and(|at| now.duration_since(at).as_millis() < ETA_DISPLAY_INTERVAL_MS) {
        return;
    }
    tracker.last_display = Some(now);
    let mbps = network_rate / (1024.0 * 1024.0);
    let speed = if mbps >= 1.0 { format!("↓ {mbps:.1} MB/s") } else { format!("↓ {:.0} KB/s", network_rate / 1024.0) };
    show_speed(app, speed, format_eta(eta));
}

pub fn update_depot_download_progress(app: &mut App, percent: f64) {
    taskbar_depot_progress(app, percent);
    set_bar(app, "depot-progress-fill", percent.min(100.0));
    app.ui.set_text("depot-progress-text", format!("{percent:.1}%"));
    update_speed_and_eta(app, percent);
}

fn update_speed_and_eta(app: &mut App, percent: f64) {
    let now = Instant::now();
    let tracker = &mut app.state.speed;
    tracker.last_update = Some(now);
    if tracker.last_time.is_none() {
        tracker.last_time = Some(now);
        tracker.last_percent = percent;
        return;
    }
    tracker.samples.push_back((percent, now));
    if tracker.samples.len() > 10 {
        tracker.samples.pop_front();
    }
    if tracker.samples.len() < 2 {
        return;
    }
    let oldest = tracker.samples[0];
    let newest = tracker.samples[tracker.samples.len() - 1];
    let seconds = newest.1.duration_since(oldest.1).as_secs_f64();
    let delta = newest.0 - oldest.0;
    if seconds <= 0.0 || delta <= 0.0 {
        return;
    }
    let rate = delta / seconds;
    let eta = (100.0 - percent) / rate;
    if tracker.last_display.is_some_and(|at| now.duration_since(at).as_millis() < ETA_DISPLAY_INTERVAL_MS) {
        return;
    }
    tracker.last_display = Some(now);
    let speed = if tracker.current_depot_size > 0 {
        format!("↓ {:.1} MB/s", (tracker.current_depot_size as f64 * delta / 100.0) / seconds / (1024.0 * 1024.0))
    } else {
        format!("↓ {rate:.2}%/s")
    };
    show_speed(app, speed, format_eta(eta));
}

fn format_eta(seconds: f64) -> String {
    if !seconds.is_finite() || !(0.0..=86400.0).contains(&seconds) {
        return "calculating...".to_string();
    }
    let total = seconds as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("~{h}h {m}m remaining")
    } else if m > 0 {
        format!("~{m}m {s}s remaining")
    } else {
        format!("~{s}s remaining")
    }
}

fn format_elapsed(seconds: f64) -> String {
    let total = seconds as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

pub fn new_entry_id() -> String {
    let a = u64::from_str_radix(&telemetry::telemetry_nonce(), 16).unwrap_or(0);
    let b = u64::from_str_radix(&telemetry::telemetry_nonce(), 16).unwrap_or(0);
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16,
        (a & 0xfff) as u16,
        (((b >> 48) as u16) & 0x3fff) | 0x8000,
        b & 0xffff_ffff_ffff
    )
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn build_pending_history_entry(app: &mut App, msg: &Value) {
    let Some(download_dir) = app.state.download_dir.clone() else { return };
    let results = msg["results"].as_array().cloned().unwrap_or_default();
    let total = if results.is_empty() { if app.state.parsed_data.is_some() { app.state.selected_depots.len() } else { 0 } } else { results.len() };
    let success = if results.is_empty() { total } else { results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count() };
    let status = if total > 0 && success == total { "complete" } else { "partial" };
    let app_id = app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).filter(|id| !id.is_empty()).or_else(|| app.state.search_app_id.map(|id| id.to_string())).unwrap_or_default();
    let depot_ids: Vec<String> =
        if results.is_empty() { app.state.selected_depots.iter().cloned().collect() } else { results.iter().map(|r| str_of(r, "depotId")).filter(|id| !id.is_empty()).collect() };
    app.state.pending_history_entry = Some(json!({
        "id": new_entry_id(),
        "app_id": app_id,
        "game_name": app.state.game_name,
        "header_image": app.state.header_image,
        "depot_count": total,
        "depots_downloaded": success,
        "status": status,
        "download_dir": download_dir,
        "started_at": app.state.download_started_at.clone().unwrap_or_else(now_iso),
        "completed_at": now_iso(),
        "source_repo": app.state.search_repo,
        "depot_ids": depot_ids,
    }));
}

pub fn commit_pending_history(app: &mut App) {
    if let Some(entry) = app.state.pending_history_entry.take() {
        app.tasks.fire("record_history_entry", json!({ "entry": entry }));
    }
}

fn handle_complete(app: &mut App, msg: &Value) {
    app.state.speed.stale = false;
    let outcome = msg["diag"]["outcome"].as_str().unwrap_or("complete");
    let all_ok = outcome == "complete";
    let results = msg["results"].as_array().cloned().unwrap_or_default();
    if all_ok {
        set_bar(app, "progress-bar-fill", 100.0);
    } else if !results.is_empty() {
        let ok = results.iter().filter(|r| r["success"].as_bool().unwrap_or(false)).count();
        set_bar(app, "progress-bar-fill", ((ok as f64 / results.len() as f64) * 100.0).round());
    }
    let status = if all_ok {
        format!(r#"<span class="status-success">{} {}</span>"#, icon(ICON_CHECK_CIRCLE), escape_html(app.i18n.t("progress.statusComplete")))
    } else {
        format!(r#"<span class="status-warning">{} {}</span>"#, icon(ICON_ALERT_TRIANGLE), escape_html(app.i18n.t("progress.statusIncomplete")))
    };
    app.ui.set_html("progress-status", status);
    if all_ok {
        update_depot_download_progress(app, 100.0);
    }
    app.ui.hide("download-speed-info");
    let props = telemetry::merged([json!({ "success": all_ok }), telemetry::job_context(app), msg["diag"].clone()]);
    app.emit_event("download_completed", props);
    build_pending_history_entry(app, msg);
    let summary = event_text(app, msg);
    show_completion(app, all_ok, summary.clone());
    if all_ok {
        onboarding::maybe_show_feedback_prompt(app);
    } else {
        show_failure_advice(app, &msg["advice"]);
    }
    for result in &results {
        let ok = result["success"].as_bool().unwrap_or(false);
        let text = t!(app, if ok { "progress.depotComplete" } else { "progress.depotFailed" });
        update_depot_status(app, &str_of(result, "depotId"), if ok { "done" } else { "error" }, text);
    }
    append_terminal_line(app, format!("\n{summary}"), if all_ok { "success" } else { "error" });
    if app.state.repair_running {
        let repaired: f64 = results.iter().map(|r| r["downloadedBytes"].as_f64().unwrap_or(0.0)).sum();
        let line = if repaired > 0.0 { t!(app, "progress.repairDone", size = format_bytes(repaired).unwrap_or_default()) } else { t!(app, "progress.repairClean") };
        append_terminal_line(app, format!("✓ {line}"), "success");
    }
    let game = app.state.game_name.clone().unwrap_or_else(|| t!(app, "common.game"));
    if all_ok {
        let (title, body) = (t!(app, "notifications.completeTitle"), t!(app, "notifications.completeBody", name = game));
        download_control::show_notification(app, title, body);
    } else {
        let title = t!(app, "notifications.partialTitle");
        let body = if summary.is_empty() { t!(app, "notifications.partialBody", name = game) } else { summary };
        download_control::show_notification(app, title, body);
    }
    download_control::play_notification_sound(app);
    cleanup_progress_listener(app);
    emulator::check_emulator_support(app);
    download_control::maybe_schedule_shutdown(app);
    queue::queue_after_download(app, if all_ok { "complete" } else { "partial" });
}

fn handle_error(app: &mut App, msg: &Value) {
    let depot_id = str_of(msg, "depotId");
    if !depot_id.is_empty() {
        let text = t!(app, "progress.depotError");
        update_depot_status(app, &depot_id, "error", text);
    }
    let error_text = event_text(app, msg);
    let line = t!(app, "progress.errorLine", message = error_text);
    append_terminal_line(app, line, "error");
    if !depot_id.is_empty() {
        return;
    }
    app.state.speed.stale = false;
    app.ui.hide("download-speed-info");
    let key = msg["key"].as_str().map(str::to_string).unwrap_or_else(|| telemetry::error_key_or_unmatched(&str_of(msg, "message")));
    let props = telemetry::merged([json!({ "success": false }), telemetry::job_context(app), msg["diag"].clone(), json!({ "err_key": key })]);
    app.emit_event("download_completed", props);
    show_completion(app, false, error_text.clone());
    show_failure_advice(app, &msg["advice"]);
    let (title, body) = (t!(app, "notifications.failedTitle"), t!(app, "notifications.failedBody", message = error_text));
    download_control::show_notification(app, title, body);
    download_control::play_notification_sound(app);
    cleanup_progress_listener(app);
    download_control::maybe_schedule_shutdown(app);
    queue::queue_after_download(app, "failed");
}

fn handle_cancelled(app: &mut App, msg: &Value) {
    app.state.speed.stale = false;
    set_bar(app, "progress-bar-fill", 0.0);
    let status = t!(app, "progress.cancelledStatus");
    app.ui.set_text("progress-status", status);
    app.ui.hide("download-speed-info");
    let localized = match msg["step"].as_str() {
        Some("cancelled_kept") => t!(app, "progress.cancelledKept"),
        Some("cancelled_cleanup") => t!(app, "progress.cancelledCleanup"),
        _ if !str_of(msg, "message").is_empty() => event_text(app, msg),
        _ => t!(app, "progress.cancelledCleanup"),
    };
    append_terminal_line(app, format!("\n{localized}"), "error");
    let props = telemetry::merged([
        json!({ "success": false }),
        telemetry::job_context(app),
        json!({
            "outcome": "cancelled",
            "depots_total": app.state.dl_depot_count.unwrap_or(0),
            "last_stage": app.state.dl_stage.clone().unwrap_or_else(|| "unknown".to_string()),
            "duration_bucket": telemetry::elapsed_bucket(app),
            "engine": app.state.current_engine.clone().unwrap_or_else(|| "native".to_string()),
        }),
        msg["diag"].clone(),
    ]);
    app.emit_event("download_completed", props);
    show_completion(app, false, localized);
    cleanup_progress_listener(app);
    queue::queue_after_download(app, "cancelled");
}

fn update_depot_status(app: &mut App, depot_id: &str, status: &'static str, text: String) {
    if let Some(item) = app.state.depot_progress.iter_mut().find(|item| item.depot_id == depot_id) {
        item.status = status;
        item.text = text;
    }
}

fn update_overall_progress(app: &mut App, current: f64, total: f64) {
    if total <= 0.0 {
        return;
    }
    set_bar(app, "progress-bar-fill", ((current / total) * 100.0).round().min(99.0));
}

pub fn append_terminal_line(app: &mut App, text: impl Into<String>, kind: &'static str) {
    app.state.terminal.push((kind, text.into()));
    app.state.terminal_scroll = true;
}

pub fn show_completion(app: &mut App, success: bool, message: String) {
    reset_taskbar_progress(app);
    app.state.job_id = None;
    app.ui.set_classes("completion-message", &["completion-message", if success { "completion-message--success" } else { "completion-message--error" }]);
    app.ui.set_text("completion-message", message.clone());
    app.ui.hide("btn-cancel");
    app.ui.hide("btn-pause");
    app.state.download_failed = !success;
    app.state.last_failure_text = (!success).then_some(message);
    app.ui.set_hidden("btn-report-failure", success);
    app.ui.show("btn-next-step");
    if success {
        emulator::update_next_button_text(app);
    } else {
        let back = t!(app, "progress.backToSelection");
        app.ui.set_text("btn-next-step", back);
    }
}

fn advice_action(action: &str) -> Option<Action> {
    Some(match action {
        "enter_mh_key" => ui::action(|app| {
            select::go_back_to_select(app);
            show_mh_key_required_hint(app);
        }),
        "sources" | "proxy" => ui::action(|app| settings::open_settings(app, Some("sources"))),
        "upload" => ui::action(|app| {
            reset_app(app);
            steps::switch_tab(app, "upload");
        }),
        "retry" => ui::action(|app| {
            select::go_back_to_select(app);
            select::start_download(app);
        }),
        "back" => ui::action(|app| {
            select::go_back_to_select(app);
            app.ui.request_focus("download-dir");
        }),
        _ => return None,
    })
}

fn show_failure_advice(app: &mut App, advice: &Value) {
    let id = str_of(advice, "id");
    if !ADVICE_IDS.contains(&id.as_str()) {
        return;
    }
    let mut actions = Vec::new();
    for action in advice["actions"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        let Some(run) = advice_action(action) else { continue };
        let (advice_id, name) = (id.clone(), action.to_string());
        actions.push((
            t!(app, &format!("advice.actions.{action}")),
            ui::action(move |app| {
                app.emit_event("advice_action", json!({ "advice": advice_id, "action": name }));
                run(app);
            }),
        ));
    }
    let (title, text) = (t!(app, &format!("advice.{id}.title")), t!(app, &format!("advice.{id}.text")));
    ui::render_next_steps(app, "failure-advice", title, text, true, actions);
    app.ui.show("failure-advice");
    app.ui.scroll_into_view("failure-advice");
    app.emit_event("advice_shown", json!({ "advice": id }));
}

fn show_mh_hint(app: &mut App, key: &str) {
    app.state.mh_hint = Some(t!(app, key));
    app.ui.add_class("mh-apikey", "dd-path__input--error");
    app.ui.request_focus("mh-apikey");
}

pub fn show_mh_key_required_hint(app: &mut App) {
    show_mh_hint(app, "select.manifestHubRequired");
}

pub fn hide_mh_key_required_hint(app: &mut App) {
    app.state.mh_hint = None;
    app.ui.remove_class("mh-apikey", "dd-path__input--error");
}

pub fn show_mh_key_suggestion_hint(app: &mut App) {
    hide_mh_key_required_hint(app);
    show_mh_hint(app, "select.manifestHubAfterFailure");
}

pub fn active_update_dir(app: &App, app_id: &str) -> Option<String> {
    match (&app.state.update_dir, &app.state.update_app_id) {
        (Some(dir), Some(id)) if id == app_id => Some(dir.clone()),
        _ => None,
    }
}

pub fn active_repair(app: &App, app_id: &str) -> Option<Value> {
    active_update_dir(app, app_id).and(app.state.repair_manifests.clone())
}

pub fn render_update_notice(app: &mut App, step: u8) {
    let app_id = app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).unwrap_or_default();
    match active_update_dir(app, &app_id) {
        Some(dir) if step == 2 => {
            let key = if active_repair(app, &app_id).is_some() { "select.repairNotice" } else { "select.updateNotice" };
            let html = t!(app, key, path = escape_html(&dir));
            app.ui.set_html("update-mode-notice", html);
            app.ui.show("update-mode-notice");
        }
        _ => app.ui.hide("update-mode-notice"),
    }
}

pub fn reset_app(app: &mut App) {
    commit_pending_history(app);
    app.state.update_dir = None;
    app.state.update_app_id = None;
    app.state.repair_manifests = None;
    if let Some(orphan) = app.state.job_id.clone() {
        let props = telemetry::abandon_props(app);
        app.emit_event("download_abandoned", props);
        app.tasks.fire("cancel_download", json!({ "jobId": orphan }));
    }
    telemetry::clear_job_telemetry_state(app);
    app.state.set_parsed(None);
    app.state.selected_depots.clear();
    app.state.job_id = None;
    app.state.game_name = None;
    app.state.game_denuvo = false;
    app.state.header_image = None;
    app.state.download_dir = None;
    app.state.depot_manifests.clear();
    app.state.search_repos.clear();
    app.state.selected_repo = None;
    app.state.search_app_id = None;
    app.state.search_sha = None;
    app.state.search_key_vdf_keys = None;
    app.state.emulator_available = false;
    emulator::reset_state(app);
    shortcuts::reset_state(app);
    cleanup_progress_listener(app);
    app.ui.hide("btn-next-step");
    app.ui.hide("game-info-banner");
    app.ui.hide("game-info-loading");
    app.ui.hide("search-results");
    app.ui.hide("search-next-row");
    app.ui.hide("search-error");
    app.ui.hide("search-game-banner");
    app.ui.hide("manifest-loading");
    steps::reset_upload(app);
    steps::reset_patch_only_tab(app);
    steps::go_to_step(app, 1);
    if !app.state.queue_running {
        let tasks = app.tasks.clone();
        app.tasks.spawn(async move {
            tasks.sleep(300).await;
            tasks.post(download_control::show_pending_followup_if_any);
        });
    }
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-next-step" => {
            if app.state.download_failed {
                app.state.download_failed = false;
                steps::go_to_step(app, 2);
                if app.state.suggest_mh_key {
                    app.state.suggest_mh_key = false;
                    show_mh_key_suggestion_hint(app);
                }
            } else if app.state.shortcut_supported {
                shortcuts::go_to_shortcut_step(app);
            } else if app.state.steam_library_supported {
                shortcuts::go_to_steam_library_step(app);
            } else if app.state.emulator_available {
                emulator::go_to_emulator_step(app, false);
            } else {
                reset_app(app);
            }
        }
        _ => return false,
    }
    true
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "depot-progress-list" => {
            let markup: String = app
                .state
                .depot_progress
                .iter()
                .map(|item| {
                    let glyph = match item.status {
                        "active" => "◉",
                        "done" => "✓",
                        "error" => "✗",
                        _ => "●",
                    };
                    format!(
                        r#"<div class="depot-progress-item" id="depot-progress-{id}"><div class="depot-progress-item__icon depot-progress-item__icon--{status}">{glyph}</div><div class="depot-progress-item__label">Depot {id}</div><div class="depot-progress-item__status">{text}</div></div>"#,
                        id = escape_html(&item.depot_id),
                        status = item.status,
                        text = escape_html(&item.text),
                    )
                })
                .collect();
            dom.markup(markup);
        }
        "terminal-output" => {
            let lines = &app.state.terminal;
            let older = lines.len().saturating_sub(TERMINAL_ELEMENTS);
            for chunk in lines[..older].chunks(200) {
                let text = chunk.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>().join("\n");
                dom.el("div.terminal__line--stdout", |dom| dom.text(text));
            }
            let last = lines.len().saturating_sub(1);
            for (index, (kind, text)) in lines.iter().enumerate().skip(older) {
                let element = match *kind {
                    "stderr" => "div.terminal__line--stderr",
                    "warn" => "div.terminal__line--warn",
                    "error" => "div.terminal__line--error",
                    "info" => "div.terminal__line--info",
                    "success" => "div.terminal__line--success",
                    _ => "div.terminal__line--stdout",
                };
                dom.el(el(element).scroll_into_view(index == last && app.state.terminal_scroll), |dom| dom.text(text));
            }
            app.state.terminal_scroll = false;
        }
        "mh-apikey-required" => {
            if let Some(hint) = &app.state.mh_hint {
                dom.el("p.dd-path__hint.dd-path__hint--error#mh-apikey-required", |dom| dom.text(hint));
            }
        }
        _ => return false,
    }
    true
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    if !app.state.speed.stale {
        return;
    }
    ctx.request_repaint_after(std::time::Duration::from_secs(1));
    let Some(last) = app.state.speed.last_update else { return };
    let waiting = last.elapsed().as_secs_f64();
    if waiting > 5.0 {
        let (speed, eta) = (t!(app, "progress.largeFile"), t!(app, "progress.waitingFor", time = format_elapsed(waiting)));
        show_speed(app, speed, eta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_percent_lines_and_formats_times() {
        assert_eq!(leading_percent(" 42.5% done"), Some(42.5));
        assert_eq!(leading_percent("100%"), Some(100.0));
        assert_eq!(leading_percent("1234% x"), None);
        assert_eq!(leading_percent("abc 5%"), None);
        assert_eq!(format_eta(59.0), "~59s remaining");
        assert_eq!(format_eta(3725.0), "~1h 2m remaining");
        assert_eq!(format_eta(f64::INFINITY), "calculating...");
        assert_eq!(format_elapsed(65.0), "1m 5s");
        assert_eq!(new_entry_id().len(), 36);
    }
}
