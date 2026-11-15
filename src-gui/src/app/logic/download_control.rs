use serde_json::{json, Value};

use super::util::{escape_html, format_bytes, str_of};
use super::{emulator, progress, select, shortcuts, ui};
use crate::app::state::ParsedData;
use crate::app::App;
use crate::t;

const SHUTDOWN_DELAY_SECONDS: u32 = 60;

pub fn show_cancel_modal(app: &mut App) {
    app.tasks.call("get_settings", Value::Null, |app, settings: Result<Value, String>| {
        let keep = settings.map(|s| s["cancel_keep_files"].as_bool().unwrap_or(false)).unwrap_or(false);
        let body = t!(app, if keep { "modals.cancel.bodyKeep" } else { "modals.cancel.body" });
        app.ui.set_html("cancel-modal-body", body);
        let yes = t!(app, if keep { "modals.cancel.yesKeep" } else { "modals.cancel.yes" });
        app.ui.set_text("btn-cancel-yes", yes);
        app.ui.show("cancel-modal");
    });
}

pub fn hide_cancel_modal(app: &mut App) {
    app.ui.hide("cancel-modal");
}

pub fn toggle_pause_download(app: &mut App) {
    let Some(job) = app.state.job_id.clone() else { return };
    let will_pause = !app.state.paused;
    app.tasks.call("pause_download", json!({ "jobId": job, "paused": will_pause }), move |app, result: Result<Value, String>| {
        if result.is_err() {
            return;
        }
        app.state.paused = will_pause;
        let engine = app.state.current_engine.clone().unwrap_or_else(|| "native".to_string());
        app.emit_event("download_paused", json!({ "paused": will_pause, "engine": engine }));
        let percent = app.state.taskbar.percent.map(f64::from);
        progress::set_taskbar_progress(app, percent);
        let label = t!(app, if will_pause { "progress.resume" } else { "progress.pause" });
        app.ui.set_text("btn-pause", label);
        let line = t!(app, if will_pause { "progress.pausedLine" } else { "progress.resumedLine" });
        progress::append_terminal_line(app, line, "info");
    });
}

pub fn cancel_download(app: &mut App) {
    hide_cancel_modal(app);
    let Some(job) = app.state.job_id.clone() else { return };
    app.ui.set_disabled("btn-cancel", true);
    let cancelling = escape_html(app.i18n.t("progress.cancelling"));
    app.ui.set_html("btn-cancel", cancelling);
    let line = t!(app, "progress.cancellingLine");
    progress::append_terminal_line(app, line, "info");
    app.tasks.call("cancel_download", json!({ "jobId": job }), |app, result: Result<Value, String>| {
        let Err(error) = result else { return };
        let lower = error.to_lowercase();
        if lower.contains("not found") || lower.contains("not running") {
            let line = t!(app, "progress.noLongerRunning");
            progress::append_terminal_line(app, line, "info");
            let ended = t!(app, "progress.ended");
            progress::show_completion(app, false, ended);
        } else {
            let line = t!(app, "progress.cancelFailed", message = app.i18n.localize_error(&error));
            progress::append_terminal_line(app, line, "error");
            app.ui.show("btn-next-step");
            app.ui.hide("btn-cancel");
        }
    });
}

fn trim_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

pub fn show_disk_space(app: &mut App, free_gb: f64, drive: &str) {
    let gb = trim_number(free_gb);
    let (classes, key): (&[&'static str], &str) = if free_gb < 2.0 {
        (&["disk-space-info", "disk-space-info--danger"], "progress.diskCritical")
    } else if free_gb < 10.0 {
        (&["disk-space-info", "disk-space-info--warning"], "progress.diskLow")
    } else {
        (&["disk-space-info"], "progress.diskFree")
    };
    app.ui.set_classes("disk-space-info", classes);
    let text = t!(app, key, gb = gb, drive = drive);
    app.ui.set_text("disk-space-text", text);
}

pub fn show_notification(app: &mut App, title: String, body: String) {
    if app.ctx.input(|i| i.focused) {
        return;
    }
    app.platform.notify(&title, &body);
}

pub fn play_notification_sound(app: &mut App) {
    if app.state.notification_sound_enabled {
        app.platform.beep();
    }
}

fn shutdown_tick(app: &mut App, token: u64) {
    if app.state.shutdown_timer != Some(token) {
        return;
    }
    app.state.shutdown_remaining = app.state.shutdown_remaining.saturating_sub(1);
    if app.state.shutdown_remaining == 0 {
        return shutdown_now(app);
    }
    render_shutdown_countdown(app);
    schedule_shutdown_tick(app, token);
}

fn schedule_shutdown_tick(app: &mut App, token: u64) {
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(1000).await;
        tasks.post(move |app| shutdown_tick(app, token));
    });
}

pub fn maybe_schedule_shutdown(app: &mut App) {
    if app.state.queue_running || !app.ui.checked("shutdown-after-toggle") {
        return;
    }
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.shutdown_timer = Some(token);
    app.state.shutdown_remaining = SHUTDOWN_DELAY_SECONDS;
    render_shutdown_countdown(app);
    app.ui.set_disabled("btn-shutdown-now", false);
    app.ui.show("shutdown-modal");
    app.emit_event("shutdown_after", json!({ "action": "countdown" }));
    schedule_shutdown_tick(app, token);
}

fn render_shutdown_countdown(app: &mut App) {
    let text = t!(app, "modals.shutdown.body", seconds = app.state.shutdown_remaining);
    app.ui.set_text("shutdown-modal-body", text);
}

pub fn abort_scheduled_shutdown(app: &mut App) {
    if app.state.shutdown_timer.take().is_some() {
        app.emit_event("shutdown_after", json!({ "action": "aborted" }));
    }
    app.ui.hide("shutdown-modal");
    app.ui.set_checked("shutdown-after-toggle", false);
}

pub fn shutdown_now(app: &mut App) {
    app.state.shutdown_timer = None;
    app.ui.set_disabled("btn-shutdown-now", true);
    let running = t!(app, "modals.shutdown.running");
    app.ui.set_text("shutdown-modal-body", running);
    progress::commit_pending_history(app);
    let followup = pending_followup(app);
    let download_dir = app.state.download_dir.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let saved = match followup {
            Some(followup) => tasks.invoke_raw("save_pending_followup", json!({ "followup": followup })).await.is_ok(),
            None => false,
        };
        let _ = tasks.invoke_raw("emit_telemetry_event", json!({ "kind": "shutdown_after", "props": { "action": "powered_off", "followup": saved } })).await;
        if let Err(error) = tasks.invoke_raw("power_off_system", Value::Null).await {
            if saved {
                let _ = tasks.invoke_raw("clear_pending_followup", json!({ "downloadDir": download_dir })).await;
            }
            tasks.post(move |app| {
                app.emit_event("shutdown_after", json!({ "action": "failed" }));
                let text = t!(app, "modals.shutdown.failed", message = app.i18n.localize_error(&error));
                app.ui.set_text("shutdown-modal-body", text);
                app.ui.set_disabled("btn-shutdown-now", false);
                app.ui.set_checked("shutdown-after-toggle", false);
            });
        }
    });
}

pub struct Confirm {
    pub title: String,
    pub body: String,
    pub confirm: String,
    pub cancel: Option<String>,
    pub danger: bool,
}

pub fn ask_confirm(app: &mut App, confirm: Confirm, answer: impl FnOnce(&mut App, bool) + 'static) {
    app.ui.set_text("confirm-modal-title", confirm.title);
    app.ui.set_text("confirm-modal-body", confirm.body);
    app.ui.set_text("btn-confirm-yes", confirm.confirm);
    let cancel = confirm.cancel.unwrap_or_else(|| t!(app, "common.cancel"));
    app.ui.set_text("btn-confirm-no", cancel);
    app.ui.set_class("btn-confirm-yes", "btn--danger", confirm.danger);
    app.ui.set_class("btn-confirm-yes", "btn--primary", !confirm.danger);
    ui::ask(app, "confirm-modal", answer);
}

pub fn confirm_free_space(app: &mut App, needed: u64, updating: bool, then: impl FnOnce(&mut App, bool) + Send + 'static) {
    if updating || needed == 0 {
        return then(app, true);
    }
    let dir = select::get_download_dir(app);
    let args = json!({ "downloadDir": (!dir.is_empty()).then_some(dir), "needed": needed });
    app.tasks.call("check_free_space", args, move |app, result: Result<Value, String>| {
        let Ok(result) = result else { return then(app, true) };
        if result.is_null() || result["enough"].as_bool().unwrap_or(false) || result["free"].is_null() {
            return then(app, true);
        }
        let size = |value: &Value| format_bytes(value.as_f64().unwrap_or(0.0)).unwrap_or_else(|| "0 MB".to_string());
        let confirm = Confirm {
            title: t!(app, "modals.space.title"),
            body: t!(app, "modals.space.body", needed = size(&result["needed"]), free = size(&result["free"]), path = str_of(&result, "path")),
            confirm: t!(app, "modals.space.start"),
            cancel: None,
            danger: false,
        };
        ask_confirm(app, confirm, then);
    });
}

pub fn ask_resume_mode(app: &mut App, answer: impl FnOnce(&mut App, Option<&'static str>) + 'static) {
    app.state.resume_answer = Some(Box::new(answer));
    app.ui.show("resume-modal");
}

fn finish_resume(app: &mut App, mode: Option<&'static str>) {
    app.ui.hide("resume-modal");
    if let Some(answer) = app.state.resume_answer.take() {
        answer(app, mode);
    }
}

fn pending_followup(app: &App) -> Option<Value> {
    let has_next = app.state.shortcut_supported || app.state.steam_library_supported || app.state.emulator_available;
    let app_id = shortcuts::current_app_id_for_steam(app)?;
    let dir = app.state.download_dir.clone()?;
    if app.state.download_failed || !has_next {
        return None;
    }
    Some(json!({
        "app_id": app_id,
        "game_name": app.state.game_name,
        "header_image": app.state.header_image,
        "download_dir": dir,
        "created_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    }))
}

pub fn save_pending_followup(app: &mut App) {
    if let Some(followup) = pending_followup(app) {
        app.tasks.fire("save_pending_followup", json!({ "followup": followup }));
    }
}

pub fn show_pending_followup_if_any(app: &mut App) {
    app.tasks.call("get_pending_followup", Value::Null, |app, followup: Result<Value, String>| {
        let Ok(followup) = followup else { return };
        if followup.is_null() {
            return;
        }
        let name = super::util::opt_str(&followup, "game_name").unwrap_or_else(|| format!("App {}", str_of(&followup, "app_id")));
        let body = t!(app, "modals.followup.body", name = name);
        app.ui.set_text("followup-modal-body", body);
        app.emit_event("followup", json!({ "action": "offered" }));
        app.state.followup = Some(followup);
        app.ui.show("followup-modal");
    });
}

fn resume_pending_followup(app: &mut App, followup: Value) {
    app.state.download_dir = super::util::opt_str(&followup, "download_dir");
    app.state.game_name = super::util::opt_str(&followup, "game_name");
    app.state.header_image = super::util::opt_str(&followup, "header_image");
    app.state.set_parsed(Some(ParsedData { main_app_id: str_of(&followup, "app_id"), ..Default::default() }));
    app.state.download_failed = false;
    emulator::check_emulator_support_then(app, |app| {
        if app.state.shortcut_supported {
            shortcuts::go_to_shortcut_step(app);
        } else if app.state.steam_library_supported {
            shortcuts::go_to_steam_library_step(app);
        } else if app.state.emulator_available {
            emulator::go_to_emulator_step(app, false);
        }
    });
}

pub fn check_dot_net(app: &mut App) {
    app.tasks.call("get_settings", Value::Null, |app, settings: Result<Value, String>| {
        let Ok(settings) = settings else { return };
        if settings["use_native_downloader"].as_bool() != Some(false) {
            return app.ui.hide("dotnet-warning");
        }
        if app.state.dotnet_warning_dismissed {
            return;
        }
        app.tasks.call("check_dotnet", Value::Null, |app, result: Result<Value, String>| {
            if result.is_ok_and(|r| !r["installed"].as_bool().unwrap_or(false)) {
                app.ui.show("dotnet-warning");
            }
        });
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-cancel" => show_cancel_modal(app),
        "btn-pause" => toggle_pause_download(app),
        "btn-cancel-yes" => cancel_download(app),
        "btn-cancel-no" | "cancel-modal>modal__backdrop" => hide_cancel_modal(app),
        "btn-shutdown-abort" | "shutdown-modal>modal__backdrop" => abort_scheduled_shutdown(app),
        "btn-shutdown-now" => shutdown_now(app),
        "btn-confirm-yes" => ui::answer(app, "confirm-modal", true),
        "btn-confirm-no" | "confirm-modal>modal__backdrop" => ui::answer(app, "confirm-modal", false),
        "btn-resume-fast" => finish_resume(app, Some("fast")),
        "btn-resume-verify" => finish_resume(app, Some("verify")),
        "btn-resume-cancel" | "resume-modal>modal__backdrop" => finish_resume(app, None),
        "btn-followup-later" => {
            app.emit_event("followup", json!({ "action": "later" }));
            app.ui.hide("followup-modal");
        }
        "followup-modal>modal__backdrop" => app.ui.hide("followup-modal"),
        "btn-followup-discard" | "btn-followup-resume" => {
            let Some(followup) = app.state.followup.take() else { return true };
            app.tasks.fire("clear_pending_followup", json!({ "downloadDir": followup["download_dir"] }));
            app.ui.hide("followup-modal");
            if id == "btn-followup-resume" {
                app.emit_event("followup", json!({ "action": "resumed" }));
                resume_pending_followup(app, followup);
            } else {
                app.emit_event("followup", json!({ "action": "discarded" }));
            }
        }
        "dotnet-warning-dismiss" => {
            app.ui.hide("dotnet-warning");
            app.state.dotnet_warning_dismissed = true;
        }
        "dotnet-install-link" => app.open_url("https://dotnet.microsoft.com/en-us/download/dotnet/thank-you/runtime-desktop-9.0.16-windows-x64-installer"),
        _ => return false,
    }
    true
}
