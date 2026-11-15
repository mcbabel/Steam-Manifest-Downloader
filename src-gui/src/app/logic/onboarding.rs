use serde_json::{json, Value};

use super::ui::Next;
use super::util::{date_medium, opt_str, parse_date, str_of};
use super::{sources, steps, updater};
use crate::app::App;
use crate::t;

const REPO_URL: &str = "https://github.com/MCbabel/Steam-Manifest-Downloader";
const FEEDBACK_MIN_DOWNLOADS: usize = 3;

fn choose_welcome_location(app: &mut App) {
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(selected) = platform.pick_folder(None).await else { return };
        let selected = selected.to_string_lossy().to_string();
        let Ok(mut settings) = tasks.invoke::<Value>("get_settings", Value::Null).await else { return };
        settings["download_location"] = json!(selected);
        if tasks.invoke_raw("save_settings", json!({ "settings": settings })).await.is_err() {
            return;
        }
        tasks.post(move |app| {
            app.state.default_download_dir = selected.clone();
            app.ui.set_value("download-dir", selected.clone());
            app.ui.set_text("welcome-location-path", selected);
            app.emit_event("welcome_choice", json!({ "choice": "location" }));
        });
    });
}

fn apply_welcome_choice(app: &mut App, choice: &str) {
    match choice {
        "search" => {
            steps::switch_tab(app, "search");
            sources::refresh_sources_ui(app);
            app.ui.request_focus("search-appid-input");
        }
        "patch" => steps::switch_tab(app, "patchOnly"),
        "upload" => steps::switch_tab(app, "upload"),
        _ => {}
    }
}

pub fn show_welcome_if_needed(app: &mut App, next: Next) {
    if app.language_stored {
        return next(app);
    }
    let location = if app.state.default_download_dir.is_empty() { t!(app, "welcome.locationUnset") } else { app.state.default_download_dir.clone() };
    app.ui.set_text("welcome-location-path", location);
    app.emit_event("welcome_shown", Value::Null);
    app.state.boot_next = Some(next);
    app.ui.show("welcome-modal");
}

fn finish_welcome(app: &mut App, choice: &str) {
    app.ui.hide("welcome-modal");
    app.emit_event("welcome_choice", json!({ "choice": choice }));
    apply_welcome_choice(app, choice);
    if let Some(next) = app.state.boot_next.take() {
        next(app);
    }
}

pub fn show_whats_new_if_needed(app: &mut App, next: Next) {
    let first = !app.language_stored;
    let parked = app.state.park(next);
    app.tasks.call("get_whats_new", json!({ "firstStart": first }), move |app, info: Result<Value, String>| {
        let Some(next) = app.state.unpark(parked) else { return };
        let Some(info) = info.ok().filter(|info| opt_str(info, "body").is_some()) else { return next(app) };
        let notes = updater::render_release_notes(&str_of(&info, "body"));
        if notes.is_empty() {
            return next(app);
        }
        let title = t!(app, "whatsNew.title", version = format!("v{}", str_of(&info, "version")));
        app.ui.set_text("whats-new-title", title);
        let date = opt_str(&info, "date").and_then(|date| parse_date(&date)).map(|date| date_medium(app.i18n.code(), &date)).unwrap_or_default();
        app.ui.set_text("whats-new-date", date);
        app.ui.set_html("whats-new-notes", notes);
        app.emit_event("whats_new_shown", Value::Null);
        app.state.whats_new_url = opt_str(&info, "releaseUrl");
        app.ui.set_hidden("btn-whats-new-github", app.state.whats_new_url.is_none());
        app.state.boot_next = Some(next);
        app.ui.show("whats-new-modal");
        app.ui.request_focus("btn-whats-new-close");
    });
}

fn close_whats_new(app: &mut App) {
    app.ui.hide("whats-new-modal");
    if let Some(next) = app.state.boot_next.take() {
        next(app);
    }
}

pub fn maybe_show_feedback_prompt(app: &mut App) {
    if !app.ui.hidden("feedback-prompt") {
        return;
    }
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Ok(settings) = tasks.invoke::<Value>("get_settings", Value::Null).await else { return };
        if settings["feedback_prompt_done"].as_bool().unwrap_or(false) {
            return;
        }
        let Ok(entries) = tasks.invoke::<Vec<Value>>("get_history", Value::Null).await else { return };
        if entries.iter().filter(|e| e["status"] == "complete").count() >= FEEDBACK_MIN_DOWNLOADS {
            tasks.post(|app| app.ui.show("feedback-prompt"));
        }
    });
}

fn answer_feedback_prompt(app: &mut App, action: &str) {
    app.ui.hide("feedback-prompt");
    app.emit_event("feedback_prompt", json!({ "action": action }));
    match action {
        "star" => app.open_url(REPO_URL),
        "issue" => app.open_url(&format!("{REPO_URL}/issues/new/choose")),
        _ => {}
    }
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Ok(mut settings) = tasks.invoke::<Value>("get_settings", Value::Null).await {
            settings["feedback_prompt_done"] = json!(true);
            let _ = tasks.invoke_raw("save_settings", json!({ "settings": settings })).await;
        }
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "welcome__choice[data-choice=upload]" => finish_welcome(app, "upload"),
        "welcome__choice[data-choice=search]" => finish_welcome(app, "search"),
        "welcome__choice[data-choice=patch]" => finish_welcome(app, "patch"),
        "btn-welcome-close" | "welcome-modal>modal__backdrop" => finish_welcome(app, "closed"),
        "btn-welcome-location" => choose_welcome_location(app),
        "btn-whats-new-close" | "whats-new-modal>modal__backdrop" => close_whats_new(app),
        "btn-whats-new-github" => {
            if let Some(url) = app.state.whats_new_url.clone() {
                app.open_url(&url);
            }
        }
        "btn[data-feedback=star]" => answer_feedback_prompt(app, "star"),
        "btn[data-feedback=issue]" => answer_feedback_prompt(app, "issue"),
        "feedback-card__close[data-feedback=dismiss]" => answer_feedback_prompt(app, "dismiss"),
        _ => return false,
    }
    true
}
