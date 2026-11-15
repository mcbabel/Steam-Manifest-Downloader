pub mod account;
pub mod bug_report;
pub mod download_control;
pub mod emulator;
pub mod emulator_extras;
pub mod emulator_settings;
pub mod events;
pub mod history;
pub mod journey;
pub mod onboarding;
pub mod picker;
pub mod select_picker;
pub mod progress;
pub mod queue;
pub mod remote;
pub mod search;
pub mod select;
pub mod settings;
pub mod shortcuts;
pub mod sources;
pub mod steps;
pub mod telemetry;
pub mod ui;
pub mod updater;
pub mod util;

use egui::Key;
use serde_json::Value;

use super::App;
use crate::dom::{Dom, El};

pub(crate) fn start(app: &mut App) {
    app.state.current_step = 1;
    app.state.mode = "upload".to_string();
    ui::init_theme(app);
    ui::apply_shortcut_hints(app);
    select::load_settings_and_defaults(app);
    shortcuts::check_shortcut_support(app);
    queue::refresh_queue(app);
    account::init_account(app);
    emulator::init(app);
    settings::init(app);
    history::init_history_toolbar(app);
    download_control::check_dot_net(app);
    sources::refresh_sources_ui(app);
    ui::show_language_picker_if_needed(app, Box::new(|app| telemetry::init_telemetry_consent(app, Box::new(|app| onboarding::show_welcome_if_needed(app, Box::new(|app| onboarding::show_whats_new_if_needed(app, Box::new(boot_done))))))));
}

fn boot_done(app: &mut App) {
    download_control::show_pending_followup_if_any(app);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(1500).await;
        tasks.post(updater::check_for_updates);
    });
    remote::init_remote(app);
}

pub(crate) fn resolve_url(app: &App, url: &str) -> String {
    let url = emulator::resolve_url(app, url);
    remote::resolve_url(app, &url)
}

pub(crate) fn language_changed(app: &mut App) {
    ui::update_theme_button(app);
    ui::apply_shortcut_hints(app);
    emulator_settings::language_changed(app);
}

pub(crate) fn event(app: &mut App, channel: &str, payload: Value) {
    if channel == "download-progress" {
        progress::handle_progress_message(app, payload);
    } else if !emulator::event(app, channel, &payload) {
        updater::event(app, channel, &payload);
    }
}

pub(crate) fn frame(app: &mut App, ctx: &egui::Context) {
    account::frame(app, ctx);
    select_picker::frame(app, ctx);
    settings::frame(app, ctx);
    history::frame(app, ctx);
    ui::modal_keys(app, ctx);
    ui::shortcuts(app, ctx);
    steps::frame(app, ctx);
    progress::frame(app, ctx);
    telemetry::frame(app, ctx);
    emulator::frame(app, ctx);
    journey::frame(app, ctx);
}

pub(crate) fn click(app: &mut App, id: &str) -> bool {
    if let Some(url) = id.strip_prefix("open:") {
        app.open_url(url);
        return true;
    }
    events::click(app, id)
        || ui::click(app, id)
        || steps::click(app, id)
        || search::click(app, id)
        || sources::click(app, id)
        || select::click(app, id)
        || progress::click(app, id)
        || download_control::click(app, id)
        || shortcuts::click(app, id)
        || queue::click(app, id)
        || account::click(app, id)
        || emulator::click(app, id)
        || settings::click(app, id)
        || history::click(app, id)
        || updater::click(app, id)
        || onboarding::click(app, id)
        || bug_report::click(app, id)
        || remote::click(app, id)
        || telemetry::click(app, id)
}

pub(crate) fn changed(app: &mut App, id: &str) -> bool {
    select::changed(app, id) || shortcuts::changed(app, id) || account::changed(app, id) || emulator::changed(app, id) || settings::changed(app, id) || history::changed(app, id) || bug_report::changed(app, id)
}

pub(crate) fn input(app: &mut App, id: &str) -> bool {
    steps::input(app, id) || select::input(app, id) || shortcuts::input(app, id) || account::input(app, id) || emulator::input(app, id) || settings::input(app, id) || history::input(app, id) || bug_report::input(app, id)
}

pub(crate) fn key(app: &mut App, id: &str, key: Key) -> bool {
    steps::key(app, id, key);
    settings::key(app, id, key)
}

pub(crate) fn is_empty(app: &App, id: &str) -> Option<bool> {
    sources::is_empty(app, id).or_else(|| remote::is_empty(app, id))
}

pub(crate) fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    ui::fill(app, id, dom) || steps::fill(app, id, dom) || search::fill(app, id, dom) || sources::fill(app, id, dom) || select::fill(app, id, dom) || progress::fill(app, id, dom) || shortcuts::fill(app, id, dom) || queue::fill(app, id, dom) || account::fill(app, id, dom) || emulator::fill(app, id, dom) || settings::fill(app, id, dom) || history::fill(app, id, dom) || updater::fill(app, id, dom) || remote::fill(app, id, dom)
}

pub(crate) fn select(app: &mut App, dom: &mut Dom<'_>, _element: El, id: &'static str, options: &[(&'static str, &'static str, &'static str)]) {
    select_picker::render(app, dom, id, options);
}

pub(crate) fn image(app: &mut App, dom: &mut Dom<'_>, element: El, key: &'static str) {
    let source = app.ui.attr(key, "src").map(str::to_string).unwrap_or_default();
    dom.image(element, &source, false);
}
