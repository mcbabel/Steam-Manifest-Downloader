use std::rc::Rc;

use egui::{Key, Modifiers};
use serde_json::{json, Value};

use super::{history, settings, sources, steps};
use crate::app::state::{Action, NextSteps};
use crate::app::{view, App};
use crate::dom::{el, Dom};
use crate::i18n;
use crate::icons;
use crate::t;

pub type Next = Box<dyn FnOnce(&mut App)>;

pub fn init_theme(app: &mut App) {
    update_theme_button(app);
}

pub fn toggle_theme(app: &mut App) {
    app.toggle_theme();
    update_theme_button(app);
}

pub fn update_theme_button(app: &mut App) {
    let label = t!(app, if app.theme.is_dark() { "header.themeToLight" } else { "header.themeToDark" });
    app.ui.set_attr("btn-theme-toggle", "title", label);
}

fn shortcut_label(app: &App, key: &str) -> String {
    format!("{}+{}", app.i18n.t("shortcuts.ctrl"), key)
}

pub fn apply_shortcut_hints(app: &mut App) {
    for (id, key, combo) in [("btn-history", "header.history", "H"), ("btn-settings", "header.settings", ","), ("drop-zone", "shortcuts.openFile", "O")] {
        let title = format!("{} ({})", app.i18n.t(key), shortcut_label(app, combo));
        app.ui.set_attr(id, "title", title);
    }
}

pub fn focus_search_field(app: &mut App) -> bool {
    if app.state.current_step == 1 {
        steps::switch_tab(app, "search");
        sources::refresh_sources_ui(app);
        app.ui.request_focus("search-appid-input");
        return true;
    }
    if app.state.current_step == 2 {
        app.ui.request_focus("depotSearch");
        return true;
    }
    false
}

pub fn topmost_visible_dialog(app: &App) -> Option<&'static str> {
    view::MODALS.iter().rev().map(|(id, _, _)| *id).find(|id| !app.ui.hidden(id))
}

pub fn shortcuts(app: &mut App, ctx: &egui::Context) {
    let pressed = |key: Key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key));
    let modal = topmost_visible_dialog(app);
    let history_open = modal == Some("history-modal");
    let mut handled = None;
    if pressed(Key::H) {
        if history_open {
            history::close_history(app);
            handled = Some("history");
        } else if modal.is_none() {
            history::open_history(app);
            handled = Some("history");
        }
    } else if pressed(Key::Comma) {
        if modal.is_none() {
            settings::open_settings(app, None);
            handled = Some("settings");
        }
    } else if pressed(Key::O) {
        if modal.is_none() && app.state.current_step == 1 {
            steps::switch_tab(app, "upload");
            steps::open_file_dialog(app);
            handled = Some("open");
        }
    } else if pressed(Key::F) {
        if history_open {
            app.ui.request_focus("history-search");
            handled = Some("search");
        } else if modal.is_none() && focus_search_field(app) {
            handled = Some("search");
        }
    }
    if let Some(key) = handled {
        app.emit_event("shortcut_key", json!({ "key": key }));
    }
}

pub fn modal_keys(app: &mut App, ctx: &egui::Context) {
    let Some(modal) = topmost_visible_dialog(app) else { return };
    if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
        if let Some((_, cancel)) = view::MODAL_CANCEL.iter().find(|(id, _)| *id == modal) {
            app.click(cancel);
        }
    }
}

pub fn render_next_steps(app: &mut App, container: &'static str, title: String, text: String, error: bool, actions: Vec<(String, Action)>) {
    app.ui.set_classes(container, if error { &["next-steps", "next-steps--error"] } else { &["next-steps"] });
    app.state.next_steps.insert(container, NextSteps { title, text, error, actions });
}

pub fn hide_next_steps(app: &mut App, container: &'static str) {
    app.ui.hide(container);
    app.state.next_steps.remove(container);
}

pub fn action(f: impl Fn(&mut App) + 'static) -> Action {
    Rc::new(f)
}

pub fn fill_next_steps(app: &mut App, container: &str, dom: &mut Dom<'_>) -> bool {
    let Some(steps) = app.state.next_steps.get(container) else { return false };
    let mut clicked: Option<Action> = None;
    if !steps.title.is_empty() {
        dom.el("div.next-steps__title", |dom| dom.text(&steps.title));
    }
    if !steps.text.is_empty() {
        dom.el("div.next-steps__text", |dom| dom.text(&steps.text));
    }
    if !steps.actions.is_empty() {
        dom.el("div.next-steps__actions", |dom| {
            for (index, (label, action)) in steps.actions.iter().enumerate() {
                let element = el("button.btn.btn--small").class(if index == 0 { "btn--primary" } else { "btn--outline" });
                if dom.button(element, |dom| dom.text(label)).clicked() {
                    clicked = Some(action.clone());
                }
            }
        });
    }
    if let Some(action) = clicked {
        action(app);
    }
    true
}

pub fn language_cards(app: &mut App, dom: &mut Dom<'_>, active: &str) -> Option<&'static str> {
    let _ = app;
    let mut picked = None;
    for (code, label) in i18n::AVAILABLE {
        let element = el("button.language-card").class_if("is-active", code == active).attr("data-lang", code).label(label);
        let clicked = dom
            .button(element, |dom| {
                dom.el("span.language-card__flag", |dom| {
                    dom.icon("svg", if code == "de" { icons::FLAG_DE } else { icons::FLAG_GB });
                });
                dom.el("span.language-card__label", |dom| dom.text(label));
            })
            .clicked();
        if clicked {
            picked = Some(code);
        }
    }
    picked
}

pub fn show_language_picker_if_needed(app: &mut App, next: Next) {
    if app.language_stored {
        return next(app);
    }
    app.state.boot_next = Some(next);
    app.ui.show("language-picker");
}

fn pick_first_language(app: &mut App, code: &'static str) {
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Ok(mut fresh) = tasks.invoke::<Value>("get_settings", Value::Null).await {
            fresh["language"] = json!(code);
            let _ = tasks.invoke_raw("save_settings", json!({ "settings": fresh })).await;
        }
        tasks.post(move |app| {
            if code != app.i18n.code() {
                app.set_language(code);
            }
            app.ui.hide("language-picker");
            if let Some(next) = app.state.boot_next.take() {
                next(app);
            }
        });
    });
}

pub fn ask(app: &mut App, modal: &'static str, answer: impl FnOnce(&mut App, bool) + 'static) {
    app.state.answers.insert(modal, Box::new(answer));
    app.ui.show(modal);
}

pub fn answer(app: &mut App, modal: &'static str, yes: bool) {
    app.ui.hide(modal);
    if let Some(answer) = app.state.answers.remove(modal) {
        answer(app, yes);
    }
}

pub fn show_toast(app: &mut App, text: impl Into<String>, kind: &str, duration_ms: u64) {
    let class = match kind {
        "success" => "app-toast--success",
        "error" => "app-toast--error",
        "warning" => "app-toast--warning",
        _ => "app-toast--info",
    };
    app.ui.set_text("app-toast", text);
    app.ui.set_classes("app-toast", &["app-toast", class]);
    app.state.toast_timer += 1;
    let token = app.state.toast_timer;
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(duration_ms).await;
        tasks.post(move |app| {
            if app.state.toast_timer == token {
                app.ui.hide("app-toast");
            }
        });
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-theme-toggle" => toggle_theme(app),
        _ => return false,
    }
    true
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "btn-theme-toggle" => {
            if app.theme.is_dark() {
                dom.icon(el("svg.theme-icon.theme-icon--moon#theme-icon").size(16.0, 16.0), icons::MOON);
            } else {
                dom.icon(el("svg.theme-icon.theme-icon--sun#theme-icon").size(16.0, 16.0), icons::SUN);
            }
            true
        }
        "language-picker-cards" => {
            let active = app.i18n.code().to_string();
            if let Some(code) = language_cards(app, dom, &active) {
                pick_first_language(app, code);
            }
            true
        }
        _ => fill_next_steps(app, id, dom),
    }
}
