use serde_json::{json, Value};

use super::{settings, ui};
use crate::app::App;
use crate::dom::Dom;
use crate::t;

fn sources_of(settings: &Value) -> Vec<String> {
    settings["depot_sources"].as_array().map(|list| list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default()
}

pub fn validate_source_url(app: &App, raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() {
        return Some(app.i18n.t("errors.sourceEmpty").to_string());
    }
    let lower = url.to_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Some(app.i18n.t("errors.sourceProtocol").to_string());
    }
    None
}

pub fn refresh_sources_ui(app: &mut App) {
    app.tasks.call("get_settings", Value::Null, |app, settings: Result<Value, String>| {
        let saved = settings.map(|s| sources_of(&s)).unwrap_or_default();
        let empty = saved.is_empty();
        app.ui.set_hidden("sources-empty-state", !empty);
        app.ui.set_hidden("search-input-block", empty);
        if app.state.pending_sources.is_none() {
            app.state.saved_sources = saved;
        }
    });
}

fn show_error(app: &mut App, error_el: &'static str, message: String) {
    app.ui.set_text(error_el, message);
    app.ui.show(error_el);
}

pub fn add_depot_source(app: &mut App, raw_url: String, error_el: &'static str, input: &'static str) {
    app.ui.hide(error_el);
    if let Some(error) = validate_source_url(app, &raw_url) {
        return show_error(app, error_el, error);
    }
    let url = raw_url.trim().to_string();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let mut settings = tasks.invoke::<Value>("get_settings", Value::Null).await.unwrap_or(Value::Null);
        let mut sources = sources_of(&settings);
        if sources.contains(&url) {
            tasks.post(move |app| {
                let message = t!(app, "search.sourceExists");
                show_error(app, error_el, message);
            });
            return;
        }
        sources.push(url);
        if settings.is_object() {
            settings["depot_sources"] = json!(sources);
            let _ = tasks.invoke_raw("save_settings", json!({ "settings": settings })).await;
        }
        tasks.post(move |app| {
            refresh_sources_ui(app);
            app.ui.set_value(input, "");
        });
    });
}

pub fn add_pending_source(app: &mut App, raw_url: &str, error_el: &'static str) -> bool {
    app.ui.hide(error_el);
    let url = raw_url.trim().to_string();
    let sources = app.state.pending_sources.clone().unwrap_or_default();
    let message = validate_source_url(app, raw_url).or_else(|| sources.contains(&url).then(|| t!(app, "search.sourceExists")));
    if let Some(message) = message {
        show_error(app, error_el, message);
        return false;
    }
    let mut next = sources;
    next.push(url);
    app.state.pending_sources = Some(next);
    true
}

pub fn remove_pending_source(app: &mut App, index: usize) {
    let sources = app.state.pending_sources.clone().unwrap_or_default();
    if index >= sources.len() {
        return;
    }
    let remove = move |app: &mut App| {
        let mut sources = app.state.pending_sources.clone().unwrap_or_default();
        if index < sources.len() {
            sources.remove(index);
        }
        app.state.pending_sources = Some(sources);
        settings::update_settings_dirty(app);
    };
    if app.state.pristine_sources.as_ref().is_some_and(|list| list.contains(&sources[index])) {
        ui::ask(app, "remove-source-modal", move |app, yes| {
            if yes {
                remove(app);
            }
        });
    } else {
        remove(app);
    }
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-sources-empty-add" => {
            let value = app.ui.value("sources-empty-input").to_string();
            add_depot_source(app, value, "sources-empty-error", "sources-empty-input");
        }
        "btn-sources-add" => {
            let value = app.ui.value("sources-add-input").to_string();
            if add_pending_source(app, &value, "sources-add-error") {
                app.ui.set_value("sources-add-input", "");
            }
            settings::update_settings_dirty(app);
        }
        "btn-remove-source-yes" => ui::answer(app, "remove-source-modal", true),
        "btn-remove-source-no" | "remove-source-modal>modal__backdrop" => ui::answer(app, "remove-source-modal", false),
        _ => match id.strip_prefix("source-remove:").and_then(|index| index.parse::<usize>().ok()) {
            Some(index) => remove_pending_source(app, index),
            None => return false,
        },
    }
    true
}

pub fn is_empty(app: &App, id: &str) -> Option<bool> {
    (id == "sources-list").then(|| app.state.pending_sources.as_ref().unwrap_or(&app.state.saved_sources).is_empty())
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    if id != "sources-list" {
        return false;
    }
    let sources = app.state.pending_sources.clone().unwrap_or_else(|| app.state.saved_sources.clone());
    let remove_label = super::util::escape_html(app.i18n.t("settings.removeSource"));
    let mut markup = String::new();
    for (index, source) in sources.iter().enumerate() {
        let safe = super::util::escape_html(source);
        markup.push_str(&format!(r#"<li><span title="{safe}">{safe}</span><button data-click="source-remove:{index}">{remove_label}</button></li>"#));
    }
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}
