use serde_json::{json, Value};

use super::util::{escape_html, opt_str, str_of};
use super::{emulator, progress, steps, telemetry};
use crate::app::App;
use crate::dom::Dom;
use crate::platform::FileFilter;
use crate::t;

pub fn check_shortcut_support(app: &mut App) {
    app.tasks.call("is_shortcut_supported", Value::Null, |app, result: Result<Value, String>| {
        if let Ok(result) = result {
            app.state.shortcut_supported = result["supported"].as_bool().unwrap_or(false);
            if app.state.shortcut_supported {
                app.ui.show("step4-connector");
                app.ui.show("step4-indicator");
            }
        }
        check_steam_library_support(app, steps::renumber_steps);
    });
}

pub fn check_steam_library_support(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    app.tasks.call("steam_library_detect", Value::Null, move |app, install: Result<Value, String>| {
        match install {
            Ok(install) => {
                app.state.steam_library_supported = true;
                app.state.steam_library_user = Some(install);
                app.state.steam_library_error = None;
            }
            Err(error) => {
                app.state.steam_library_supported = false;
                app.state.steam_library_user = None;
                app.state.steam_library_error = Some(error);
            }
        }
        let supported = app.state.steam_library_supported;
        let show_linux_step = supported && !app.state.shortcut_supported;
        app.ui.set_hidden("step6-connector", !show_linux_step);
        app.ui.set_hidden("step6-indicator", !show_linux_step);
        app.ui.set_hidden("shortcut-steam-row", !app.state.shortcut_supported);
        app.ui.set_disabled("shortcut-steam-library", !supported);
        if !supported {
            app.ui.set_checked("shortcut-steam-library", false);
        }
        let hint = if supported {
            t!(app, "steamLibrary.windowsToggleHint")
        } else {
            match &app.state.steam_library_error {
                Some(error) if !error.is_empty() => app.i18n.localize_error(error),
                _ => t!(app, "steamLibrary.notDetected"),
            }
        };
        app.ui.set_text("shortcut-steam-hint", hint);
        app.ui.set_hidden("btn-shortcut-steam-choose", supported);
        then(app);
    });
}

pub fn go_to_steam_library_step(app: &mut App) {
    steps::go_to_step(app, 6);
    reset_steam_buttons(app);
    match &app.state.steam_library_user {
        Some(user) => {
            app.ui.remove_class("steam-library-status", "emu-release-status--busy");
            app.ui.add_class("steam-library-status", "emu-release-status--ready");
            let name = opt_str(user, "persona_name").unwrap_or_else(|| str_of(user, "user_id3"));
            let text = t!(app, "steamLibrary.detected", name = name);
            app.ui.set_text("steam-library-status", text);
        }
        None => {
            app.ui.remove_class("steam-library-status", "emu-release-status--ready");
            let text = t!(app, "steamLibrary.notDetected");
            app.ui.set_text("steam-library-status", text);
        }
    }
    let name = app.state.game_name.clone().unwrap_or_default();
    app.ui.set_value("steam-game-name", name);
    app.ui.set_value("steam-launch-options", "");
    app.ui.hide("steam-library-result");
    detect_into(app, Target::Steam);
}

#[derive(Clone, Copy, PartialEq)]
enum Target {
    Shortcut,
    Steam,
}

impl Target {
    fn input(self) -> &'static str {
        if self == Target::Steam { "steam-exe-path" } else { "shortcut-exe-path" }
    }

    fn button(self) -> &'static str {
        if self == Target::Steam { "btn-steam-add" } else { "btn-create-shortcuts" }
    }

    fn section(self) -> &'static str {
        if self == Target::Steam { "steam-detected-section" } else { "shortcut-detected-section" }
    }
}

fn detect_into(app: &mut App, target: Target) {
    let Some(dir) = app.state.download_dir.clone() else { return };
    let scanning = t!(app, if target == Target::Steam { "shortcut.exePlaceholder" } else { "shortcut.scanning" });
    app.ui.set_value(target.input(), scanning);
    app.ui.set_disabled(target.button(), true);
    app.ui.hide(target.section());
    app.tasks.call("detect_executables", json!({ "downloadDir": dir }), move |app, result: Result<Value, String>| {
        app.ui.set_disabled(target.button(), false);
        let exes = match result {
            Ok(result) => result["executables"].as_array().cloned().unwrap_or_default(),
            Err(_) => {
                app.ui.set_value(target.input(), "");
                if target == Target::Shortcut {
                    let failed = t!(app, "shortcut.detectFailed");
                    app.ui.set_attr(target.input(), "placeholder", failed);
                }
                return;
            }
        };
        if target == Target::Steam {
            app.state.steam_detected_exes = exes.clone();
        } else {
            app.state.shortcut_detected_exes = exes.clone();
        }
        if exes.is_empty() {
            app.ui.set_value(target.input(), "");
            let none = t!(app, "shortcut.noneFound");
            app.ui.set_attr(target.input(), "placeholder", none);
            return;
        }
        let recommended = exes.iter().find(|e| e["recommended"].as_bool().unwrap_or(false)).unwrap_or(&exes[0]);
        app.ui.set_value(target.input(), str_of(recommended, "path"));
        if exes.len() > 1 {
            app.ui.show(target.section());
        }
    });
}

pub fn exe_platform_badge(app: &App, exe: &Value, all: &[Value]) -> String {
    let mut platforms: Vec<&str> = all.iter().filter_map(|e| e["platform"].as_str()).filter(|p| !p.is_empty()).collect();
    platforms.sort_unstable();
    platforms.dedup();
    let Some(platform) = exe["platform"].as_str().filter(|p| !p.is_empty()) else { return String::new() };
    if platforms.len() < 2 {
        return String::new();
    }
    let kind = match platform {
        "linux" => "linux",
        "macos" => "macos",
        _ => "windows",
    };
    format!(r#" <span class="depot-tag depot-tag--{}">{}</span>"#, if kind == "macos" { "mac" } else { kind }, escape_html(app.i18n.t(&format!("depotTags.{kind}"))))
}

fn detected_list(app: &App, exes: &[Value], action: &str) -> String {
    exes.iter()
        .map(|exe| {
            let badge = if exe["recommended"].as_bool().unwrap_or(false) { format!(r#" <span class="shortcut-exe-badge">{}</span>"#, escape_html(app.i18n.t("common.recommended"))) } else { String::new() };
            format!(
                r#"<div class="shortcut-exe-item" data-click="{action}:{path}"><span class="shortcut-exe-item__name">{name}{platform}{badge}</span><span class="shortcut-exe-item__size">{size}</span></div>"#,
                path = escape_html(&str_of(exe, "path")),
                name = escape_html(&str_of(exe, "name")),
                platform = exe_platform_badge(app, exe, exes),
                size = format_shortcut_file_size(exe["size"].as_f64().unwrap_or(0.0)),
            )
        })
        .collect()
}

fn browse(app: &mut App, target: Target) {
    let filters = if app.state.shortcut_supported { vec![FileFilter { name: "Executables".to_string(), extensions: vec!["exe".to_string()] }] } else { Vec::new() };
    let start = app.state.download_dir.clone().map(std::path::PathBuf::from);
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Some(path) = platform.pick_file(start, filters).await {
            let path = path.to_string_lossy().to_string();
            tasks.post(move |app| {
                app.ui.set_value(target.input(), path);
                app.ui.set_disabled(target.button(), false);
            });
        }
    });
}

fn set_steam_library_result(app: &mut App, kind: &str, text: String) {
    let classes: &[&'static str] = match kind {
        "success" => &["completion-message", "completion-message--success"],
        "error" => &["completion-message", "completion-message--error"],
        _ => &["completion-message"],
    };
    app.ui.set_classes("steam-library-result", classes);
    app.ui.set_text("steam-library-result", text);
    app.state.steam_running_prompt = false;
}

fn show_steam_running_prompt(app: &mut App) {
    let text = t!(app, "steamLibrary.steamRunning");
    set_steam_library_result(app, "error", text);
    app.state.steam_running_prompt = true;
}

pub fn current_app_id_for_steam(app: &App) -> Option<String> {
    app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).filter(|id| !id.is_empty()).or_else(|| app.state.search_app_id.map(|id| id.to_string()))
}

fn derive_start_dir(exe_path: &str) -> String {
    let Some(last) = exe_path.rfind(['/', '\\']).filter(|index| *index > 0) else { return String::new() };
    let dir = &exe_path[..last];
    if dir.ends_with('/') || dir.ends_with('\\') {
        dir.to_string()
    } else {
        format!("{}{}", dir, if exe_path.contains('\\') { '\\' } else { '/' })
    }
}

fn switch_steam_button_to_next(app: &mut App) {
    let next = t!(app, "steamLibrary.next");
    app.ui.set_text("btn-steam-add", next);
    app.ui.set_attr("btn-steam-add", "data-mode", "next");
    app.ui.set_disabled("btn-steam-add", false);
    app.ui.hide("btn-steam-skip");
}

pub fn reset_steam_buttons(app: &mut App) {
    let add = t!(app, "steamLibrary.add");
    app.ui.set_text("btn-steam-add", add);
    app.ui.remove_attr("btn-steam-add", "data-mode");
    app.ui.set_disabled("btn-steam-add", false);
    app.ui.show("btn-steam-skip");
}

fn steam_library_continue(app: &mut App) {
    if app.state.emulator_available {
        emulator::go_to_emulator_step(app, false);
    } else {
        progress::reset_app(app);
    }
}

fn perform_steam_library_add(app: &mut App, close_steam: bool) {
    let exe_path = app.ui.value("steam-exe-path").trim().to_string();
    if exe_path.is_empty() {
        let text = t!(app, "steamLibrary.error", message = "no executable selected");
        return set_steam_library_result(app, "error", text);
    }
    let Some(app_id) = current_app_id_for_steam(app) else {
        let text = t!(app, "steamLibrary.error", message = "missing app id");
        return set_steam_library_result(app, "error", text);
    };
    let typed = app.ui.value("steam-game-name").to_string();
    let name = if typed.is_empty() { app.state.game_name.clone().unwrap_or_default() } else { typed };
    let app_name = if name.trim().is_empty() { format!("App {app_id}") } else { name.trim().to_string() };
    let launch_options = app.ui.value("steam-launch-options").trim().to_string();
    app.ui.set_disabled("btn-steam-add", true);
    let adding = t!(app, "steamLibrary.adding");
    set_steam_library_result(app, "busy", adding);
    let args = json!({ "appId": app_id, "appName": app_name, "exePath": exe_path, "startDir": derive_start_dir(&exe_path), "launchOptions": launch_options, "closeSteam": close_steam });
    app.tasks.call("steam_library_add", args, move |app, result: Result<Value, String>| {
        app.ui.set_disabled("btn-steam-add", false);
        match result {
            Ok(result) => {
                let grid = result["grid_files"].as_array().map(Vec::len).unwrap_or(0);
                let restarted = result["steam_restarted"].as_bool().unwrap_or(false);
                let mut message = format!(
                    "{}\n\n{}",
                    t!(app, if restarted { "steamLibrary.successRestarted" } else { "steamLibrary.success" }, name = app_name),
                    t!(app, "steamLibrary.gridArtCount", count = grid)
                );
                if exe_path.to_lowercase().ends_with(".exe") {
                    message.push('\n');
                    message.push_str(app.i18n.t("steamLibrary.protonNote"));
                }
                set_steam_library_result(app, "success", message);
                app.emit_event("library_added", json!({ "from": "step", "ok": true, "restarted": restarted, "grid": grid > 0 }));
                switch_steam_button_to_next(app);
            }
            Err(error) if error == "STEAM_RUNNING" => show_steam_running_prompt(app),
            Err(error) => {
                let text = t!(app, "steamLibrary.error", message = app.i18n.localize_error(&error));
                set_steam_library_result(app, "error", text);
                app.emit_event("library_added", json!({ "from": "step", "ok": false }));
                telemetry::report_error(app, "steam_library", Some(&error), None);
            }
        }
    });
}

pub fn go_to_shortcut_step(app: &mut App) {
    steps::go_to_step(app, 4);
    app.state.shortcuts_created = false;
    reset_shortcut_footer(app);
    check_steam_library_support(app, |app| {
        steps::renumber_steps(app);
        detect_into(app, Target::Shortcut);
    });
}

fn reset_shortcut_footer(app: &mut App) {
    app.ui.set_disabled("btn-create-shortcuts", false);
    let label = t!(app, "shortcut.createShortcuts");
    app.ui.set_text("btn-create-shortcuts", label);
    app.ui.show("btn-shortcut-skip");
}

fn advance_from_shortcut_step(app: &mut App) {
    if app.state.emulator_available {
        emulator::go_to_emulator_step(app, false);
    } else {
        progress::reset_app(app);
    }
}

fn create_shortcuts(app: &mut App) {
    let exe_path = app.ui.value("shortcut-exe-path").trim().to_string();
    if exe_path.is_empty() {
        return;
    }
    let desktop = app.ui.checked("shortcut-desktop");
    let start_menu = app.ui.checked("shortcut-startmenu");
    if !desktop && !start_menu {
        let text = t!(app, "shortcut.noLocation");
        return show_shortcut_status(app, false, text);
    }
    app.ui.set_disabled("btn-create-shortcuts", true);
    let creating = t!(app, "shortcut.creating");
    app.ui.set_text("btn-create-shortcuts", creating);
    let game_name = app.state.game_name.clone().unwrap_or_else(|| "Game".to_string());
    let args = json!({ "exePath": exe_path, "gameName": game_name, "iconPath": null, "createDesktop": desktop, "createStartMenu": start_menu });
    app.tasks.call("create_shortcuts", args, move |app, result: Result<Value, String>| {
        app.ui.set_disabled("btn-create-shortcuts", false);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let text = t!(app, "shortcut.failed", message = app.i18n.localize_error(&error));
                show_shortcut_status(app, false, text);
                let label = t!(app, "shortcut.createShortcuts");
                return app.ui.set_text("btn-create-shortcuts", label);
            }
        };
        let (made_desktop, made_start) = (result["desktop"].as_bool().unwrap_or(false), result["startMenu"].as_bool().unwrap_or(false));
        let mut messages = Vec::new();
        if made_desktop {
            messages.push(t!(app, "shortcut.desktopCreated"));
        }
        if made_start {
            messages.push(t!(app, "shortcut.startMenuCreated"));
        }
        let errors: Vec<String> = result["errors"].as_array().map(|list| list.iter().filter_map(|e| e.as_str().map(str::to_string)).collect()).unwrap_or_default();
        if !errors.is_empty() {
            messages.push(t!(app, "shortcut.errors", list = errors.join(", ")));
        }
        let all_good = (!desktop || made_desktop) && (!start_menu || made_start);
        if all_good {
            app.emit_event("shortcut_created", json!({ "desktop": made_desktop, "start_menu": made_start }));
        }
        show_shortcut_status(app, all_good, format!("{}.", messages.join(". ")));
        if all_good {
            app.state.shortcuts_created = true;
            let next = t!(app, "common.next");
            app.ui.set_text("btn-create-shortcuts", next);
            app.ui.hide("btn-shortcut-skip");
        } else {
            let label = t!(app, "shortcut.createShortcuts");
            app.ui.set_text("btn-create-shortcuts", label);
        }
        if app.ui.checked("shortcut-steam-library") && app.state.steam_library_supported {
            add_to_steam_library_from_shortcut_step(app, exe_path);
        }
    });
}

fn append_shortcut_status(app: &mut App, message: String) {
    let existing = app.ui.text("shortcut-status").unwrap_or("").to_string();
    app.ui.set_text("shortcut-status", if existing.is_empty() { message } else { format!("{existing}\n\n{message}") });
}

fn add_to_steam_library_from_shortcut_step(app: &mut App, exe_path: String) {
    let Some(app_id) = current_app_id_for_steam(app) else { return };
    let app_name = app.state.game_name.clone().unwrap_or_else(|| format!("App {app_id}"));
    let args = json!({ "appId": app_id, "appName": app_name, "exePath": exe_path, "startDir": derive_start_dir(&exe_path), "launchOptions": "" });
    app.tasks.call("steam_library_add", args, move |app, result: Result<Value, String>| match result {
        Ok(result) => {
            let grid = result["grid_files"].as_array().map(Vec::len).unwrap_or(0);
            let message = format!("{} ({})", t!(app, "steamLibrary.success", name = app_name), t!(app, "steamLibrary.gridArtCount", count = grid));
            app.emit_event("library_added", json!({ "from": "shortcuts", "ok": true, "restarted": false, "grid": grid > 0 }));
            append_shortcut_status(app, message);
        }
        Err(error) => {
            app.emit_event("library_added", json!({ "from": "shortcuts", "ok": false }));
            telemetry::report_error(app, "steam_library", Some(&error), None);
            let message = t!(app, "steamLibrary.error", message = app.i18n.localize_error(&error));
            append_shortcut_status(app, message);
        }
    });
}

fn show_shortcut_status(app: &mut App, success: bool, message: String) {
    app.ui.set_classes("shortcut-status", &["completion-message", if success { "completion-message--success" } else { "completion-message--error" }]);
    app.ui.set_text("shortcut-status", message);
}

fn update_create_shortcuts_button(app: &mut App) {
    let any = app.ui.checked("shortcut-desktop") || app.ui.checked("shortcut-startmenu");
    app.ui.set_disabled("btn-create-shortcuts", !any);
}

fn format_shortcut_file_size(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{} B", bytes as u64)
    } else if bytes < 1_048_576.0 {
        format!("{:.1} KB", bytes / 1024.0)
    } else if bytes < 1_073_741_824.0 {
        format!("{:.1} MB", bytes / 1_048_576.0)
    } else {
        format!("{:.2} GB", bytes / 1_073_741_824.0)
    }
}

fn toggle_detected(app: &mut App, list: &str, arrow: &str) {
    let hidden = !app.ui.hidden(list);
    app.ui.set_hidden(list, hidden);
    app.ui.set_text(arrow, if hidden { "▶" } else { "▼" });
}

pub fn reset_state(app: &mut App) {
    app.state.steam_detected_exes.clear();
    app.state.shortcut_detected_exes.clear();
    app.ui.set_value("steam-exe-path", "");
    app.ui.set_value("steam-game-name", "");
    app.ui.set_value("steam-launch-options", "");
    app.ui.hide("steam-library-result");
    app.ui.hide("steam-detected-section");
    reset_steam_buttons(app);
    app.ui.set_checked("shortcut-steam-library", false);
    app.ui.hide("shortcut-status");
    app.ui.hide("shortcut-detected-section");
    app.ui.set_value("shortcut-exe-path", "");
    app.state.shortcuts_created = false;
    reset_shortcut_footer(app);
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-browse-exe" => browse(app, Target::Shortcut),
        "btn-steam-browse-exe" => browse(app, Target::Steam),
        "btn-create-shortcuts" => {
            if app.state.shortcuts_created {
                advance_from_shortcut_step(app);
            } else {
                create_shortcuts(app);
            }
        }
        "btn-shortcut-skip" => advance_from_shortcut_step(app),
        "btn-shortcut-start-over" => progress::reset_app(app),
        "btn-steam-add" => {
            if app.ui.attr("btn-steam-add", "data-mode") == Some("next") {
                steam_library_continue(app);
            } else {
                perform_steam_library_add(app, false);
            }
        }
        "btn-steam-skip" => steam_library_continue(app),
        "btn-toggle-detected" => toggle_detected(app, "shortcut-detected-list", "shortcut-detected-arrow"),
        "btn-steam-toggle-detected" => toggle_detected(app, "steam-detected-list", "steam-detected-arrow"),
        "steam-close-and-add" => perform_steam_library_add(app, true),
        _ => {
            if let Some(path) = id.strip_prefix("shortcut-exe:") {
                app.ui.set_value("shortcut-exe-path", path);
                app.ui.set_disabled("btn-create-shortcuts", false);
            } else if let Some(path) = id.strip_prefix("steam-exe:") {
                app.ui.set_value("steam-exe-path", path);
                app.ui.set_disabled("btn-steam-add", false);
            } else {
                return false;
            }
        }
    }
    true
}

pub fn changed(app: &mut App, id: &str) -> bool {
    match id {
        "shortcut-desktop" | "shortcut-startmenu" => update_create_shortcuts_button(app),
        "shortcut-steam-library" => {}
        _ => return false,
    }
    true
}

pub fn input(_app: &mut App, id: &str) -> bool {
    matches!(id, "shortcut-exe-path" | "steam-exe-path" | "steam-game-name" | "steam-launch-options")
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    let markup = match id {
        "shortcut-detected-list" => detected_list(app, &app.state.shortcut_detected_exes, "shortcut-exe"),
        "steam-detected-list" => detected_list(app, &app.state.steam_detected_exes, "steam-exe"),
        "steam-library-result" => {
            let text = escape_html(app.ui.text("steam-library-result").unwrap_or(""));
            if app.state.steam_running_prompt {
                format!(r#"{text}<div class="completion-message__actions"><button class="btn btn--outline btn--small" data-click="steam-close-and-add">{}</button></div>"#, escape_html(app.i18n.t("steamLibrary.closeAndAdd")))
            } else {
                return false;
            }
        }
        _ => return false,
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_the_start_folder_and_sizes() {
        assert_eq!(derive_start_dir("C:\\games\\x\\game.exe"), "C:\\games\\x\\");
        assert_eq!(derive_start_dir("/games/x/game"), "/games/x/");
        assert_eq!(derive_start_dir("game.exe"), "");
        assert_eq!(format_shortcut_file_size(512.0), "512 B");
        assert_eq!(format_shortcut_file_size(1536.0), "1.5 KB");
        assert_eq!(format_shortcut_file_size(3.0 * 1_073_741_824.0), "3.00 GB");
    }
}
