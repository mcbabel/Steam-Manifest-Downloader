use egui::Key;
use serde_json::{json, Value};

use super::util::{escape_html, extension, file_name, is_numeric_input, is_valid_app_id, str_of};
use super::{emulator, progress, select, telemetry, ui};
use crate::app::state::{Depot, ParsedData};
use crate::app::App;
use crate::dom::Dom;
use crate::platform::FileFilter;
use crate::t;

pub const STEP_SECTIONS: [(u8, &str); 6] =
    [(1, "step-upload"), (2, "step-select"), (3, "step-progress"), (4, "step-shortcut"), (5, "step-emulator"), (6, "step-steam-library")];
pub const STEP_ITEMS: [(u8, &str); 6] =
    [(1, "steps-item-1"), (2, "steps-item-2"), (3, "steps-item-3"), (4, "step4-indicator"), (6, "step6-indicator"), (5, "step5-indicator")];
const KEY_FILE_EXTS: [&str; 4] = ["json", "csv", "vdf", "txt"];

fn visible_items(app: &App) -> Vec<u8> {
    STEP_ITEMS.iter().filter(|(_, id)| !app.ui.hidden(id)).map(|(step, _)| *step).collect()
}

pub fn go_to_step(app: &mut App, step: u8) {
    app.state.current_step = step;
    app.ui.set_class("app", "app--wide", step == 5);
    if step == 5 {
        emulator::sync_emu_game_data_key(app);
        let tab = if app.state.emu_edit_mode { "emulator" } else { "files" };
        emulator::select_emu_tab(app, tab);
        emulator::update_emu_overview(app);
    }
    if step != 2 {
        select::cancel_auto_start(app);
    }
    progress::render_update_notice(app, step);
    for (number, id) in STEP_SECTIONS {
        app.ui.set_class(id, "active", number == step);
        app.ui.set_class(id, "hidden", number != step);
    }
    let visible = visible_items(app);
    let current = visible.iter().position(|s| *s == step);
    let skipped_entry = app.state.emu_standalone || app.state.emu_edit_mode;
    for (number, id) in STEP_ITEMS {
        let mine = visible.iter().position(|s| *s == number);
        app.ui.set_class(id, "active", number == step);
        let completed = !skipped_entry && matches!((current, mine), (Some(current), Some(mine)) if mine < current);
        app.ui.set_class(id, "completed", completed);
    }
}

pub fn renumber_steps(app: &mut App) {
    for (index, step) in visible_items(app).into_iter().enumerate() {
        app.ui.set_text(&format!("steps-number-{step}"), (index + 1).to_string());
    }
}

pub fn switch_tab(app: &mut App, tab: &str) {
    app.state.mode = tab.to_string();
    for (name, button, content) in
        [("upload", "tab-upload", "tab-content-upload"), ("search", "tab-search", "tab-content-search"), ("patchOnly", "tab-patch-only", "tab-content-patch-only")]
    {
        app.ui.set_class(button, "active", name == tab);
        app.ui.set_class(content, "active", name == tab);
    }
}

pub fn open_file_dialog(app: &mut App) {
    let filters = vec![FileFilter { name: t!(app, "upload.filterName"), extensions: ["lua", "st", "json", "csv", "vdf", "txt"].map(String::from).to_vec() }];
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Some(path) = platform.pick_file(None, filters).await {
            let path = path.to_string_lossy().to_string();
            tasks.post(move |app| handle_file_path(app, path, None));
        }
    });
}

pub fn reset_upload(app: &mut App) {
    app.ui.hide("file-info");
    app.ui.show("drop-zone");
    app.ui.hide("upload-error");
    app.ui.hide("upload-loading");
    app.state.set_parsed(None);
}

fn show_key_file_choice(app: &mut App, file_path: String, apps: Vec<Value>) {
    let title = t!(app, "upload.chooseGame");
    let text = t!(app, "upload.chooseGameText", count = apps.len());
    let actions = apps
        .iter()
        .take(12)
        .map(|entry| {
            let app_id = str_of(entry, "appId");
            let name = str_of(entry, "name");
            let label = if name.is_empty() { app_id.clone() } else { format!("{name} ({app_id})") };
            let path = file_path.clone();
            (
                label,
                ui::action(move |app| {
                    app.ui.hide("upload-choice");
                    handle_file_path(app, path.clone(), Some(app_id.clone()));
                }),
            )
        })
        .collect();
    ui::render_next_steps(app, "upload-choice", title, text, false, actions);
    app.ui.show("upload-choice");
}

pub fn handle_file_path(app: &mut App, file_path: String, app_id: Option<String>) {
    let ext = extension(&file_path);
    let key_file = KEY_FILE_EXTS.contains(&ext.as_str());
    if ext != "lua" && ext != "st" && !key_file {
        let message = t!(app, "upload.wrongFile");
        return show_upload_error(app, message);
    }
    app.ui.hide("upload-choice");
    if key_file && app_id.is_none() {
        let path = file_path.clone();
        app.tasks.call("key_file_apps", json!({ "path": file_path }), move |app, apps: Result<Vec<Value>, String>| match apps {
            Ok(apps) if apps.len() > 1 => {
                app.ui.hide("upload-error");
                show_key_file_choice(app, path, apps);
            }
            _ => parse_file(app, path, None),
        });
        return;
    }
    parse_file(app, file_path, app_id);
}

fn parse_file(app: &mut App, file_path: String, app_id: Option<String>) {
    let ext = extension(&file_path);
    app.ui.hide("drop-zone");
    app.ui.show("file-info");
    app.ui.set_text("file-name", file_name(&file_path));
    app.ui.hide("upload-error");
    app.ui.show("upload-loading");
    let app_id_arg = app_id.as_ref().and_then(|id| id.parse::<u64>().ok());
    app.tasks.call("parse_lua_file", json!({ "path": file_path, "appId": app_id_arg }), move |app, raw: Result<Value, String>| match raw {
        Ok(raw) => {
            app.state.account_mode = false;
            let depots = raw["depots"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|d| Depot {
                            depot_id: str_of(d, "depot_id"),
                            manifest_id: super::util::opt_str(d, "manifest_id").unwrap_or_else(|| "N/A".to_string()),
                            depot_key: super::util::opt_str(d, "depot_key"),
                            size_bytes: d["size_bytes"].as_u64().filter(|size| *size > 0),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let all_app_ids = raw["all_app_ids"].as_array().map(|list| list.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect()).unwrap_or_default();
            let count = depots.len();
            app.state.set_parsed(Some(ParsedData { main_app_id: str_of(&raw, "main_app_id"), depots, all_app_ids }));
            app.state.mode = "upload".to_string();
            app.ui.hide("upload-loading");
            app.emit_event("lua_parsed", json!({ "depot_count": count, "format": ext }));
            select::show_selection_step(app);
        }
        Err(error) => {
            app.ui.hide("upload-loading");
            let message = app.i18n.localize_error(&error);
            show_upload_error(app, message);
            telemetry::report_error(app, "upload", Some(&error), None);
        }
    });
}

pub fn show_upload_error(app: &mut App, message: String) {
    app.ui.set_text("upload-error", message);
    app.ui.show("upload-error");
}

pub fn hide_autocomplete(app: &mut App, dropdown: &'static str) {
    app.state.autocomplete_timer += 1;
    app.ui.hide(dropdown);
    app.state.autocomplete.remove(dropdown);
}

fn show_autocomplete_loading(app: &mut App, dropdown: &'static str) {
    let entry = app.state.autocomplete.entry(dropdown).or_default();
    entry.loading = true;
    entry.results = None;
    app.ui.show(dropdown);
}

fn trigger_autocomplete(app: &mut App, input: &'static str, dropdown: &'static str, query: String) {
    show_autocomplete_loading(app, dropdown);
    let asked = query.clone();
    app.tasks.call("search_steam_games", json!({ "query": query }), move |app, results: Result<Vec<Value>, String>| match results {
        Ok(results) => {
            let current = app.ui.value(input).trim().to_string();
            if current == asked || (!is_numeric_input(&current) && current.chars().count() >= 2) {
                let entry = app.state.autocomplete.entry(dropdown).or_default();
                entry.loading = false;
                entry.results = Some(results);
                app.ui.show(dropdown);
            }
        }
        Err(_) => hide_autocomplete(app, dropdown),
    });
}

pub fn on_autocomplete_input(app: &mut App, input: &'static str, dropdown: &'static str) {
    let value = app.ui.value(input).trim().to_string();
    app.state.autocomplete_timer += 1;
    if value.is_empty() || is_numeric_input(&value) || value.chars().count() < 2 {
        return hide_autocomplete(app, dropdown);
    }
    let token = app.state.autocomplete_timer;
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(400).await;
        tasks.post(move |app| {
            if app.state.autocomplete_timer == token {
                trigger_autocomplete(app, input, dropdown, value);
            }
        });
    });
}

fn fill_autocomplete(app: &mut App, dropdown: &'static str, dom: &mut Dom<'_>) {
    let Some(entry) = app.state.autocomplete.get(dropdown) else { return };
    let markup = match &entry.results {
        None => format!(r#"<div class="search-autocomplete__loading">{}</div>"#, escape_html(app.i18n.t("search.loading"))),
        Some(results) if results.is_empty() => format!(r#"<div class="search-autocomplete__empty">{}</div>"#, escape_html(app.i18n.t("search.noGamesFound"))),
        Some(results) => results
            .iter()
            .map(|item| {
                let app_id = escape_html(&str_of(item, "appId"));
                format!(
                    r#"<div class="search-autocomplete__item" data-click="autocomplete:{dropdown}:{app_id}"><img class="search-autocomplete__img" src="{}" data-hide-on-error><span class="search-autocomplete__name">{}</span><span class="search-autocomplete__appid">{app_id}</span></div>"#,
                    escape_html(&str_of(item, "image")),
                    escape_html(&str_of(item, "name")),
                )
            })
            .collect(),
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
}

pub fn browse_patch_only_dir(app: &mut App) {
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Some(selected) = platform.pick_folder(None).await {
            let selected = selected.to_string_lossy().to_string();
            tasks.post(move |app| {
                app.ui.set_value("patch-only-dir", selected);
                hide_patch_only_error(app);
            });
        }
    });
}

pub fn show_patch_only_error(app: &mut App, message: String) {
    app.ui.set_text("patch-only-error", message);
    app.ui.show("patch-only-error");
}

pub fn hide_patch_only_error(app: &mut App) {
    app.ui.hide("patch-only-error");
}

pub fn reset_patch_only_tab(app: &mut App) {
    app.ui.set_value("patch-only-dir", "");
    app.ui.set_value("patch-only-appid", "");
    hide_autocomplete(app, "patch-only-autocomplete");
    app.ui.hide("patch-only-loading");
    app.ui.set_disabled("btn-patch-only-start", false);
    hide_patch_only_error(app);
}

pub fn start_patch_only_emulator(app: &mut App) {
    hide_autocomplete(app, "patch-only-autocomplete");
    hide_patch_only_error(app);
    let game_dir = app.ui.value("patch-only-dir").trim().to_string();
    let app_id = app.ui.value("patch-only-appid").trim().to_string();
    if game_dir.is_empty() {
        let message = t!(app, "patchOnly.errorNoFolder");
        return show_patch_only_error(app, message);
    }
    if !is_valid_app_id(&app_id) {
        let message = t!(app, "patchOnly.errorAppId");
        return show_patch_only_error(app, message);
    }
    app.ui.set_disabled("btn-patch-only-start", true);
    app.ui.show("patch-only-loading");
    let dir = game_dir.clone();
    app.tasks.call("emu_scan_game_dir", json!({ "gameDir": game_dir }), move |app, scanned: Result<Vec<Value>, String>| {
        app.ui.hide("patch-only-loading");
        app.ui.set_disabled("btn-patch-only-start", false);
        let scanned = match scanned {
            Ok(scanned) => scanned,
            Err(error) => {
                let message = t!(app, "patchOnly.errorScan", message = app.i18n.localize_error(&error));
                return show_patch_only_error(app, message);
            }
        };
        if scanned.is_empty() {
            let message = t!(app, "patchOnly.errorNoApiFiles");
            return show_patch_only_error(app, message);
        }
        emulator::start_standalone(app, dir, app_id, scanned);
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "drop-zone" => open_file_dialog(app),
        "file-remove" => reset_upload(app),
        "btn-patch-only-browse" => browse_patch_only_dir(app),
        "btn-patch-only-start" => start_patch_only_emulator(app),
        _ => {
            let Some(rest) = id.strip_prefix("autocomplete:") else { return false };
            let Some((dropdown, app_id)) = rest.split_once(':') else { return false };
            if dropdown == "search-autocomplete" {
                app.ui.set_value("search-appid-input", app_id);
                hide_autocomplete(app, "search-autocomplete");
                super::search::perform_search(app);
            } else {
                app.ui.set_value("patch-only-appid", app_id);
                hide_autocomplete(app, "patch-only-autocomplete");
            }
        }
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    match id {
        "search-appid-input" => on_autocomplete_input(app, "search-appid-input", "search-autocomplete"),
        "patch-only-appid" => on_autocomplete_input(app, "patch-only-appid", "patch-only-autocomplete"),
        "patch-only-dir" => {}
        _ => return false,
    }
    true
}

pub fn key(app: &mut App, id: &str, key: Key) {
    match (id, key) {
        ("search-appid-input", Key::Enter) => {
            hide_autocomplete(app, "search-autocomplete");
            super::search::perform_search(app);
        }
        ("search-appid-input", Key::Escape) => hide_autocomplete(app, "search-autocomplete"),
        ("patch-only-appid", Key::Enter) => {
            hide_autocomplete(app, "patch-only-autocomplete");
            start_patch_only_emulator(app);
        }
        ("patch-only-appid", Key::Escape) => hide_autocomplete(app, "patch-only-autocomplete"),
        _ => {}
    }
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "search-autocomplete" | "patch-only-autocomplete" => fill_autocomplete(app, id, dom),
        _ => return false,
    }
    true
}

fn dropped_files(app: &mut App, ctx: &egui::Context) {
    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    app.ui.set_class("drop-zone", "drag-over", hovering);
    let dropped = ctx.input(|i| i.raw.dropped_files.first().map(|file| file.path().to_string_lossy().to_string()));
    if let Some(path) = dropped.filter(|path| !path.is_empty()) {
        handle_file_path(app, path, None);
    }
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    dropped_files(app, ctx);
    if !ctx.input(|i| i.pointer.any_pressed()) {
        return;
    }
    let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) else { return };
    for (input, dropdown) in [("search-appid-input", "search-autocomplete"), ("patch-only-appid", "patch-only-autocomplete")] {
        if app.ui.hidden(dropdown) {
            continue;
        }
        let inside = [input, dropdown].iter().any(|id| app.rects.get(id).is_some_and(|rect| rect.contains(pointer)));
        if !inside {
            hide_autocomplete(app, dropdown);
        }
    }
}
