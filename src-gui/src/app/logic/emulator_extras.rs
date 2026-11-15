use serde_json::{json, Value};

use super::emulator::{self, current_app_id_for_emu, get_selected_emu_targets, relativize_emu_path, set_emu_badge};
use super::emulator_settings as fields;
use super::util::{date_medium, escape_html, format_bytes, opt_str, str_of, time_short};
use super::{select_picker, telemetry, ui};
use crate::app::App;
use crate::dom::{el, Dom};
use crate::platform::FileFilter;
use crate::t;

const EXTRA_TABS: [&str; 4] = ["saves", "profile", "network", "mods"];
const STATUS_IDS: [&str; 6] = ["emu-saves-status", "emu-ach-status", "emu-profile-status", "emu-broadcasts-status", "emu-lobby-status", "emu-mods-status"];

fn emu_extra_targets(app: &App) -> Vec<String> {
    let list = if app.state.emu_edit_mode { app.state.emu.edit_targets.clone() } else { get_selected_emu_targets(app) };
    list.iter().map(|t| str_of(t, "path")).filter(|p| !p.is_empty()).collect()
}

fn set_status(app: &mut App, id: &str, kind: &str, text: String) {
    app.ui.set_hidden(id, text.is_empty());
    app.ui.set_class(id, "dd-path__hint--error", kind == "error");
    app.ui.set_class(id, "dd-path__hint--success", kind == "success");
    app.ui.set_text(id, text);
}

fn armed_click(app: &mut App, button: &'static str, label_key: &'static str) -> bool {
    if app.state.emu.armed.remove(button).is_some() {
        let label = t!(app, label_key);
        app.ui.set_text(button, label);
        app.ui.remove_class(button, "btn--danger");
        return true;
    }
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.emu.armed.insert(button, token);
    let confirm = t!(app, "emulator.achievements.confirm");
    app.ui.set_text(button, confirm);
    app.ui.add_class(button, "btn--danger");
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(4000).await;
        tasks.post(move |app| {
            if app.state.emu.armed.get(button) == Some(&token) {
                app.state.emu.armed.remove(button);
                let label = t!(app, label_key);
                app.ui.set_text(button, label);
                app.ui.remove_class(button, "btn--danger");
            }
        });
    });
    false
}

fn emu_game_patched(app: &App) -> bool {
    app.state.emu_edit_mode || app.state.emu.apply_complete || app.state.emu.scan.iter().any(|f| f["is_patched"] == true)
}

pub fn sync_emu_after_patch(app: &mut App) {
    let patched = emu_game_patched(app);
    app.ui.set_hidden("emu-tab-saves", !patched);
    if !patched && app.ui.has_class("emu-tab-saves", "is-active") {
        emulator::select_emu_tab(app, "files");
    }
    if !patched {
        app.ui.hide("emu-lobby-section");
    }
    let later = patched || emu_extra_targets(app).is_empty();
    app.ui.set_hidden("emu-lobby-later", later);
}

fn emu_extra_panel_state(app: &mut App, section: &str) -> bool {
    let has = !emu_extra_targets(app).is_empty();
    app.ui.set_hidden(&format!("emu-needs-{section}"), has);
    app.ui.set_flag(&format!("emu-extra-hide-{section}"), !has);
    if section == "network" {
        app.ui.set_hidden("emu-lobby-section", !has);
    }
    sync_emu_after_patch(app);
    has
}

pub fn refresh_emu_extra_tab(app: &mut App, section: &str) {
    if !EXTRA_TABS.contains(&section) || !emu_extra_panel_state(app, section) {
        return;
    }
    match section {
        "saves" => {
            refresh_emu_saves(app);
            refresh_emu_achievements(app);
        }
        "profile" => refresh_emu_profile(app),
        "network" => refresh_emu_network(app),
        _ => refresh_emu_mods(app),
    }
}

fn refresh_emu_saves(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    let app_id = current_app_id_for_emu(app);
    app.tasks.call("emu_save_folders", json!({ "target": target, "appId": app_id }), |app, folders: Result<Vec<Value>, String>| {
        let folders = folders.unwrap_or_default();
        let keep = app.state.emu.save_folder.as_ref().and_then(|current| folders.iter().find(|f| f["path"] == current["path"])).cloned();
        app.state.emu.save_folder = keep.or_else(|| folders.iter().find(|f| f["exists"] == true).cloned()).or_else(|| folders.first().cloned());
        app.state.emu.save_folders = folders;
        sync_save_buttons(app);
    });
}

fn sync_save_buttons(app: &mut App) {
    let folder = app.state.emu.save_folder.clone();
    let exists = folder.as_ref().is_some_and(|f| f["exists"] == true);
    app.ui.set_disabled("btn-emu-saves-open", !exists);
    app.ui.set_disabled("btn-emu-saves-backup", !exists);
    app.ui.set_disabled("btn-emu-saves-restore", folder.is_none());
    let badge = if exists { folder.map(|f| str_of(&f, "files")).unwrap_or_default() } else { String::new() };
    set_emu_badge(app, "saves", &badge, "");
}

fn emu_backup_name(app: &App) -> String {
    let source = app.state.game_name.clone().filter(|name| !name.is_empty()).unwrap_or_else(|| {
        let id = current_app_id_for_emu(app);
        if id.is_empty() { "game".to_string() } else { id }
    });
    let mut base = String::new();
    let mut gap = false;
    for chr in source.chars() {
        if chr.is_ascii_alphanumeric() || matches!(chr, '_' | '.' | '-') {
            base.push(chr);
            gap = false;
        } else if !gap {
            base.push('_');
            gap = true;
        }
    }
    let base: String = base.chars().take(60).collect();
    format!("{base}-saves-{}.zip", chrono::Utc::now().format("%Y-%m-%d"))
}

fn zip_filter() -> Vec<FileFilter> {
    vec![FileFilter { name: "ZIP".to_string(), extensions: vec!["zip".to_string()] }]
}

fn saves_done(app: &mut App, action: &'static str, key: &'static str, result: Result<Value, String>) {
    match result {
        Ok(count) => {
            let text = t!(app, key, count = count);
            set_status(app, "emu-saves-status", "success", text);
            app.emit_event("emu_saves", json!({ "action": action, "ok": true }));
        }
        Err(error) => {
            let text = app.i18n.localize_error(&error);
            set_status(app, "emu-saves-status", "error", text);
            app.emit_event("emu_saves", json!({ "action": action, "ok": false }));
        }
    }
}

fn backup_emu_saves(app: &mut App) {
    let Some(folder) = app.state.emu.save_folder.clone().filter(|f| f["exists"] == true) else { return };
    let name = emu_backup_name(app);
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(dest) = platform.save_file(name, zip_filter()).await else { return };
        tasks
            .ask(|app| {
                let working = t!(app, "emulator.saves.working");
                set_status(app, "emu-saves-status", "busy", working);
            })
            .await;
        let result = tasks.invoke::<Value>("emu_backup_saves", json!({ "folder": folder["path"], "dest": dest.to_string_lossy() })).await;
        tasks.post(move |app| saves_done(app, "backup", "emulator.saves.backedUp", result));
    });
}

fn restore_emu_saves(app: &mut App) {
    let Some(folder) = app.state.emu.save_folder.clone() else { return };
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(archive) = platform.pick_file(None, zip_filter()).await else { return };
        tasks
            .ask(|app| {
                let working = t!(app, "emulator.saves.working");
                set_status(app, "emu-saves-status", "busy", working);
            })
            .await;
        let result = tasks.invoke::<Value>("emu_restore_saves", json!({ "folder": folder["path"], "archive": archive.to_string_lossy() })).await;
        tasks.post(move |app| {
            saves_done(app, "restore", "emulator.saves.restored", result);
            refresh_emu_saves(app);
            refresh_emu_achievements(app);
        });
    });
}

fn emu_game_language(app: &App) -> String {
    fields::gather(app).and_then(|g| opt_str(&g, "language")).unwrap_or_else(|| "english".to_string())
}

fn load_picture(app: &mut App, path: String) {
    if path.is_empty() || app.state.emu.pictures.contains_key(&path) {
        return;
    }
    app.state.emu.pictures.insert(path.clone(), String::new());
    app.tasks.call("emu_image", json!({ "path": path.clone() }), move |app, url: Result<Value, String>| {
        if let Some(url) = url.ok().and_then(|url| url.as_str().map(str::to_string)).filter(|url| !url.is_empty()) {
            app.state.emu.pictures.insert(path, url);
        }
    });
}

fn refresh_emu_achievements(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    let args = json!({ "target": target, "appId": current_app_id_for_emu(app), "lang": emu_game_language(app) });
    app.tasks.call("emu_achievements", args, |app, view: Result<Value, String>| {
        let view = view.unwrap_or_else(|_| json!({ "rows": [], "stats_files": 0 }));
        let rows = view["rows"].as_array().cloned().unwrap_or_default();
        let earned = rows.iter().filter(|r| r["earned"] == true).count();
        let count = if rows.is_empty() { String::new() } else { t!(app, "emulator.achievements.count", earned = earned, total = rows.len()) };
        app.ui.set_text("emu-ach-count", count);
        let none = t!(app, "emulator.achievements.none");
        app.ui.set_text("emu-ach-empty", none);
        app.ui.set_hidden("emu-ach-empty", !rows.is_empty());
        app.ui.set_disabled("btn-emu-ach-reset", earned == 0);
        app.ui.set_disabled("btn-emu-stats-reset", view["stats_files"].as_u64().unwrap_or(0) == 0);
        for row in &rows {
            load_picture(app, str_of(row, "icon"));
        }
        app.state.emu.achievements = rows;
    });
}

fn reset_achievements(app: &mut App, names: Option<Vec<String>>) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    let all = names.is_none();
    let args = json!({ "target": target, "appId": current_app_id_for_emu(app), "names": names });
    app.tasks.call("emu_reset_achievements", args, move |app, result: Result<Value, String>| {
        match result {
            Ok(count) => {
                if all {
                    let text = t!(app, "emulator.achievements.lockedCount", count = count);
                    set_status(app, "emu-ach-status", "success", text);
                }
                app.emit_event("emu_achievements", json!({ "action": if all { "lock_all" } else { "lock_one" } }));
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                set_status(app, "emu-ach-status", "error", text);
            }
        }
        refresh_emu_achievements(app);
    });
}

fn reset_emu_stats(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    app.tasks.call("emu_reset_stats", json!({ "target": target, "appId": current_app_id_for_emu(app) }), |app, result: Result<Value, String>| {
        match result {
            Ok(count) => {
                let text = t!(app, "emulator.achievements.statsCleared", count = count);
                set_status(app, "emu-ach-status", "success", text);
                app.emit_event("emu_achievements", json!({ "action": "clear_stats" }));
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                set_status(app, "emu-ach-status", "error", text);
            }
        }
        refresh_emu_achievements(app);
    });
}

fn asset_filters(kind: &str) -> Vec<FileFilter> {
    let (name, extensions): (&str, &[&str]) = match kind {
        "avatar" => ("PNG / JPG", &["png", "jpg", "jpeg"]),
        "font" => ("TrueType / OpenType", &["ttf", "otf", "ttc"]),
        _ => ("WAV", &["wav"]),
    };
    vec![FileFilter { name: name.to_string(), extensions: extensions.iter().map(|e| e.to_string()).collect() }]
}

fn refresh_emu_profile(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    app.tasks.call("emu_overlay_assets", json!({ "target": target }), |app, assets: Result<Value, String>| {
        let assets = assets.ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
        let avatar = str_of(&assets, "avatar");
        app.ui.set_hidden("btn-emu-avatar-clear", avatar.is_empty());
        app.state.emu.pictures.remove(&avatar);
        load_picture(app, avatar);
        let own = t!(app, "emulator.profile.ownSound");
        let values = [("font", opt_str(&assets, "font")), ("achievement", opt_str(&assets, "achievement_sound").map(|_| own.clone())), ("friend", opt_str(&assets, "friend_sound").map(|_| own.clone()))];
        for (kind, value) in values {
            app.ui.set_hidden(&format!("emu-asset-clear-{kind}"), value.is_none());
            let text = value.unwrap_or_else(|| t!(app, if kind == "font" { "emulator.profile.defaultFont" } else { "emulator.profile.noSound" }));
            app.ui.set_text(&format!("emu-asset-value-{kind}"), text);
        }
        let count = ["avatar", "font", "achievement_sound", "friend_sound"].iter().filter(|key| opt_str(&assets, key).is_some()).count();
        set_emu_badge(app, "profile", &if count > 0 { count.to_string() } else { String::new() }, if count > 0 { "on" } else { "" });
        app.state.emu.overlay_assets = assets;
        fill_emu_system_fonts(app);
    });
}

fn emu_font_file_name(path: &str) -> String {
    let base = path.rsplit(['\\', '/']).next().unwrap_or("");
    let cleaned: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '_' }).collect();
    cleaned.trim_start_matches('.').to_string()
}

fn apply_system_fonts(app: &mut App) {
    let fonts = app.state.emu.system_fonts.clone().unwrap_or_default();
    let current = str_of(&app.state.emu.overlay_assets, "font");
    let head = t!(app, if fonts.is_empty() { "emulator.profile.systemFontsNone" } else { "emulator.profile.systemFontsPick" });
    let mut options = vec![(String::new(), head)];
    let mut selected = String::new();
    for font in &fonts {
        let path = str_of(font, "path");
        if selected.is_empty() && !current.is_empty() && emu_font_file_name(&path) == current {
            selected = path.clone();
        }
        options.push((path, str_of(font, "name")));
    }
    select_picker::set_options(app, "emu-font-system", options);
    app.ui.set_value("emu-font-system", selected);
    app.ui.set_disabled("emu-font-system", fonts.is_empty());
}

fn fill_emu_system_fonts(app: &mut App) {
    if app.state.emu.system_fonts.is_some() {
        return apply_system_fonts(app);
    }
    if std::mem::replace(&mut app.state.emu.fonts_loading, true) {
        return;
    }
    app.tasks.call("emu_installed_fonts", Value::Null, |app, list: Result<Vec<Value>, String>| {
        app.state.emu.system_fonts = Some(list.unwrap_or_default());
        app.state.emu.fonts_loading = false;
        apply_system_fonts(app);
    });
}

fn profile_done(app: &mut App, asset: &str, action: &str, source: Option<&str>, result: Result<Value, String>) {
    let ok = result.is_ok();
    match result {
        Ok(_) => {
            let text = if action == "set" { t!(app, "emulator.profile.saved") } else { String::new() };
            set_status(app, "emu-profile-status", if action == "set" { "success" } else { "" }, text);
        }
        Err(error) => {
            let text = app.i18n.localize_error(&error);
            set_status(app, "emu-profile-status", "error", text);
        }
    }
    if ok || action == "set" {
        let mut props = json!({ "asset": asset, "action": action, "ok": ok });
        if let Some(source) = source {
            props["source"] = json!(source);
        }
        app.emit_event("emu_profile", props);
    }
    refresh_emu_profile(app);
}

fn set_emu_system_font(app: &mut App) {
    let targets = emu_extra_targets(app);
    let path = app.ui.value("emu-font-system").to_string();
    if targets.is_empty() || path.is_empty() {
        return;
    }
    app.tasks.call("emu_set_font", json!({ "targets": targets, "src": path }), |app, result: Result<Value, String>| profile_done(app, "font", "set", Some("system"), result));
}

fn pick_emu_asset(app: &mut App, kind: String) {
    let targets = emu_extra_targets(app);
    if targets.is_empty() {
        return;
    }
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(src) = platform.pick_file(None, asset_filters(&kind)).await else { return };
        let src = src.to_string_lossy().to_string();
        let result = match kind.as_str() {
            "avatar" => tasks.invoke::<Value>("emu_set_avatar", json!({ "targets": targets, "src": src })).await,
            "font" => tasks.invoke::<Value>("emu_set_font", json!({ "targets": targets, "src": src })).await,
            _ => tasks.invoke::<Value>("emu_set_sound", json!({ "targets": targets, "kind": kind, "src": src })).await,
        };
        tasks.post(move |app| {
            if result.is_ok() && kind == "avatar" {
                app.ui.set_checked("emu-set-enable_account_avatar", true);
            }
            profile_done(app, &kind, "set", None, result);
        });
    });
}

fn clear_emu_asset(app: &mut App, kind: String) {
    let targets = emu_extra_targets(app);
    if targets.is_empty() {
        return;
    }
    let (command, args) = match kind.as_str() {
        "avatar" => ("emu_remove_avatar", json!({ "targets": targets })),
        "font" => ("emu_remove_font", json!({ "targets": targets })),
        _ => ("emu_remove_sound", json!({ "targets": targets, "kind": kind })),
    };
    app.tasks.call(command, args, move |app, result: Result<Value, String>| {
        if result.is_ok() && kind == "avatar" {
            app.ui.set_checked("emu-set-enable_account_avatar", false);
        }
        profile_done(app, &kind, "remove", None, result);
    });
}

fn refresh_emu_network(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    app.tasks.call("emu_broadcasts", json!({ "target": target }), |app, list: Result<Vec<String>, String>| {
        let list = list.unwrap_or_default();
        app.ui.set_value("emu-broadcasts", list.join("\n"));
        set_emu_badge(app, "network", &if list.is_empty() { String::new() } else { list.len().to_string() }, if list.is_empty() { "" } else { "on" });
    });
}

fn save_emu_broadcasts(app: &mut App) {
    let targets = emu_extra_targets(app);
    if targets.is_empty() {
        return;
    }
    let text = app.ui.value("emu-broadcasts").to_string();
    let list: Vec<&str> = text.split('\n').collect();
    let entered = text.split(|c: char| c.is_whitespace() || c == ',' || c == ';').filter(|part| !part.is_empty()).count();
    app.tasks.call("emu_set_broadcasts", json!({ "targets": targets, "list": list }), move |app, bad: Result<Vec<String>, String>| {
        match bad {
            Ok(bad) => {
                let text = if bad.is_empty() { t!(app, "emulator.network.saved") } else { t!(app, "emulator.network.rejected", list = bad.join(", ")) };
                set_status(app, "emu-broadcasts-status", if bad.is_empty() { "success" } else { "error" }, text);
                app.emit_event("emu_network", json!({ "action": "broadcasts", "count": telemetry::count_bucket(entered.saturating_sub(bad.len())) }));
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                set_status(app, "emu-broadcasts-status", "error", text);
            }
        }
        refresh_emu_network(app);
    });
}

pub fn start_emu_lobby_connect(app: &mut App, allow_download: bool) {
    let (Some(target), Some(dir)) = (emu_extra_targets(app).into_iter().next(), app.state.download_dir.clone()) else { return };
    let preparing = t!(app, "emulator.network.lobbyPreparing");
    set_status(app, "emu-lobby-status", "busy", preparing);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let path = match tasks.invoke::<String>("emu_lobby_connect", json!({ "gameDir": dir, "target": target, "allowDownload": allow_download })).await {
            Ok(path) => path,
            Err(error) => {
                return tasks.post(move |app| {
                    if let Some(info) = error.strip_prefix("EMU_DOWNLOAD_CONFIRM_REQUIRED:").and_then(|raw| serde_json::from_str::<Value>(raw).ok()) {
                        app.state.emu.pending_download = None;
                        app.state.emu.lobby_retry = true;
                        emulator::show_emu_download_confirm(app, &info);
                        return set_status(app, "emu-lobby-status", "", String::new());
                    }
                    let text = if error == "GBE_TOOLS_MISSING" { t!(app, "emulator.network.lobbyMissing") } else { app.i18n.localize_error(&error) };
                    set_status(app, "emu-lobby-status", "error", text);
                    app.emit_event("emu_network", json!({ "action": "lobby", "ok": false }));
                });
            }
        };
        let launched = tasks.invoke_raw("emu_launch_console", json!({ "path": path })).await;
        tasks.post(move |app| {
            app.state.emu.download_listening = false;
            match launched {
                Ok(_) => {
                    let text = t!(app, "emulator.network.lobbyStarted");
                    set_status(app, "emu-lobby-status", "success", text);
                    app.emit_event("emu_network", json!({ "action": "lobby", "ok": true }));
                }
                Err(error) => {
                    let proton = error == "WINDOWS_TOOL_ON_LINUX";
                    let text = if proton { t!(app, "emulator.network.lobbyProton", path = relativize_emu_path(app, &path)) } else { app.i18n.localize_error(&error) };
                    set_status(app, "emu-lobby-status", if proton { "success" } else { "error" }, text);
                    app.emit_event("emu_network", json!({ "action": "lobby", "ok": proton }));
                }
            }
        });
    });
}

fn refresh_emu_mods(app: &mut App) {
    let Some(target) = emu_extra_targets(app).into_iter().next() else { return };
    app.tasks.call("emu_mods", json!({ "target": target }), |app, mods: Result<Vec<Value>, String>| {
        let mods = mods.unwrap_or_default();
        app.ui.set_hidden("emu-mods-empty", !mods.is_empty());
        for item in &mods {
            load_picture(app, str_of(item, "preview"));
        }
        set_emu_badge(app, "mods", &if mods.is_empty() { String::new() } else { mods.len().to_string() }, if mods.is_empty() { "" } else { "on" });
        app.state.emu.mods = mods;
    });
}

fn remove_emu_mod(app: &mut App, id: String) {
    let targets = emu_extra_targets(app);
    app.tasks.call("emu_remove_mod", json!({ "targets": targets, "id": id }), |app, result: Result<Value, String>| {
        match result {
            Ok(_) => app.emit_event("emu_mod", json!({ "action": "remove", "ok": true })),
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                set_status(app, "emu-mods-status", "error", text);
            }
        }
        refresh_emu_mods(app);
    });
}

fn add_emu_mod(app: &mut App, directory: bool) {
    let targets = emu_extra_targets(app);
    let id = app.ui.value("emu-mod-id").trim().to_string();
    if targets.is_empty() {
        return;
    }
    if id.is_empty() {
        let text = t!(app, "emulator.mods.needId");
        set_status(app, "emu-mods-status", "error", text);
        return app.ui.request_focus("emu-mod-id");
    }
    let app_id = current_app_id_for_emu(app);
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let picked = if directory { platform.pick_folder(None).await } else { platform.pick_file(None, Vec::new()).await };
        let Some(src) = picked else { return };
        tasks
            .ask(|app| {
                let adding = t!(app, "emulator.mods.adding");
                set_status(app, "emu-mods-status", "busy", adding);
            })
            .await;
        let result = tasks.invoke::<Value>("emu_add_mod", json!({ "targets": targets, "appId": app_id, "id": id, "src": src.to_string_lossy() })).await;
        tasks.post(move |app| {
            match result {
                Ok(item) => {
                    let other = opt_str(&item, "other_app");
                    let note = other.as_ref().map(|other| format!("\n{}", t!(app, "emulator.mods.otherApp", app = other))).unwrap_or_default();
                    let text = format!("{}{note}", t!(app, "emulator.mods.added", title = str_of(&item, "title")));
                    set_status(app, "emu-mods-status", if other.is_some() { "error" } else { "success" }, text);
                    app.ui.set_value("emu-mod-id", "");
                    app.emit_event("emu_mod", json!({ "action": "add", "ok": true, "details": item["title"] != item["id"], "other_app": other.is_some() }));
                }
                Err(error) => {
                    let text = app.i18n.localize_error(&error);
                    set_status(app, "emu-mods-status", "error", text);
                    app.emit_event("emu_mod", json!({ "action": "add", "ok": false }));
                }
            }
            refresh_emu_mods(app);
        });
    });
}

pub fn emu_cold_client_exe(app: &App) -> Option<String> {
    if app.state.emu_edit_mode || !app.ui.checked("emu-method-coldclient") {
        return None;
    }
    Some(app.ui.value("emu-cold-exe").to_string()).filter(|exe| !exe.is_empty())
}

pub fn refresh_emu_method(app: &mut App) {
    let windows = app.state.emu.scan.iter().any(|f| f["platform"] == "windows" && f["loader"].is_null());
    let show = !app.state.emu_edit_mode && windows;
    app.ui.set_hidden("emu-method-section", !show);
    let Some(dir) = app.state.download_dir.clone().filter(|dir| show && app.state.emu.cold_dir.as_ref() != Some(dir)) else { return };
    app.state.emu.cold_dir = Some(dir.clone());
    app.tasks.call("detect_executables", json!({ "downloadDir": dir }), |app, result: Result<Value, String>| {
        let exes: Vec<Value> = result
            .ok()
            .and_then(|r| r["executables"].as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e["platform"] == "windows" && !str_of(e, "name").to_lowercase().starts_with("steamclient_loader"))
            .collect();
        let options = exes.iter().map(|e| (str_of(e, "path"), relativize_emu_path(app, &str_of(e, "path")))).collect();
        let selected = exes.iter().find(|e| e["recommended"] == true).or(exes.first()).map(|e| str_of(e, "path")).unwrap_or_default();
        select_picker::set_options(app, "emu-cold-exe", options);
        app.ui.set_value("emu-cold-exe", selected);
        app.ui.set_disabled("emu-method-coldclient", exes.is_empty());
    });
}

fn sync_emu_method_ui(app: &mut App) {
    let cold = app.ui.checked("emu-method-coldclient");
    app.ui.set_hidden("emu-cold-options", !cold);
    app.ui.set_class("emu-variant-section", "emu-section--muted", cold);
}

pub fn note_emu_cold_client(app: &mut App, results: &[Value], fresh: bool) {
    let running = results.iter().any(|r| r["shortcut"] == "steam_running");
    app.ui.set_hidden("emu-cold-steam", !running);
    let ok = results.iter().any(|r| r["success"] == true && (fresh || !r["shortcut"].is_null()));
    let links: u64 = results.iter().map(|r| r["links"].as_u64().unwrap_or(0)).sum();
    if links > 0 {
        let text = t!(app, "emulator.method.linksDone", count = links);
        ui::show_toast(app, text, "success", 4000);
    }
    if fresh {
        let shortcut = results.iter().find_map(|r| r["shortcut"].as_str()).unwrap_or("none").to_string();
        app.emit_event("coldclient_used", json!({ "action": "setup", "ok": ok, "shortcut": shortcut, "links": links > 0 }));
    }
}

fn close_steam_for_cold_client(app: &mut App) {
    app.ui.set_disabled("btn-emu-cold-steam", true);
    let args = json!({ "gameDir": app.state.download_dir, "back": false, "closeSteam": true });
    app.tasks.call("emu_coldclient_shortcut", args, |app, states: Result<Vec<String>, String>| {
        match states {
            Ok(states) => {
                let ok = states.iter().any(|state| state == "updated");
                let text = t!(app, if ok { "emulator.method.shortcutDone" } else { "emulator.method.shortcutFailed" });
                ui::show_toast(app, text, if ok { "success" } else { "error" }, 4000);
                app.emit_event("coldclient_used", json!({ "action": "shortcut", "ok": ok }));
                if ok {
                    app.ui.hide("emu-cold-steam");
                }
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                ui::show_toast(app, text, "error", 4000);
            }
        }
        app.ui.set_disabled("btn-emu-cold-steam", false);
    });
}

pub fn reset_emu_extras_view(app: &mut App) {
    app.state.emu.save_folder = None;
    for tab in EXTRA_TABS {
        set_emu_badge(app, tab, "", "");
    }
    for id in STATUS_IDS {
        set_status(app, id, "", String::new());
    }
    app.ui.hide("emu-cold-steam");
    app.ui.set_checked("emu-method-replace", true);
    app.ui.set_checked("emu-method-coldclient", false);
    app.state.emu.cold_dir = None;
    sync_emu_method_ui(app);
    sync_emu_after_patch(app);
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-emu-saves-open" => {
            if let Some(path) = app.state.emu.save_folder.as_ref().map(|f| str_of(f, "path")) {
                app.tasks.call("open_folder", json!({ "path": path }), |app, result: Result<Value, String>| {
                    if let Err(error) = result {
                        let text = app.i18n.localize_error(&error);
                        ui::show_toast(app, text, "error", 4000);
                    }
                });
            }
        }
        "btn-emu-saves-backup" => backup_emu_saves(app),
        "btn-emu-saves-restore" => restore_emu_saves(app),
        "btn-emu-ach-reset" => {
            if armed_click(app, "btn-emu-ach-reset", "emulator.achievements.resetAll") {
                reset_achievements(app, None);
            }
        }
        "btn-emu-stats-reset" => {
            if armed_click(app, "btn-emu-stats-reset", "emulator.achievements.resetStats") {
                reset_emu_stats(app);
            }
        }
        "btn-emu-avatar" => pick_emu_asset(app, "avatar".to_string()),
        "btn-emu-avatar-clear" => clear_emu_asset(app, "avatar".to_string()),
        "btn-emu-broadcasts" => save_emu_broadcasts(app),
        "btn-emu-lobby" => start_emu_lobby_connect(app, false),
        "btn-emu-mod-folder" => add_emu_mod(app, true),
        "btn-emu-mod-file" => add_emu_mod(app, false),
        "btn-emu-cold-steam" => close_steam_for_cold_client(app),
        _ => {
            if let Some(kind) = id.strip_prefix("asset-pick:") {
                pick_emu_asset(app, kind.to_string());
            } else if let Some(kind) = id.strip_prefix("asset-clear:") {
                clear_emu_asset(app, kind.to_string());
            } else if let Some(path) = id.strip_prefix("save-folder:") {
                app.state.emu.save_folder = app.state.emu.save_folders.iter().find(|f| f["path"] == path).cloned();
                refresh_emu_saves(app);
            } else if let Some(name) = id.strip_prefix("ach-lock:") {
                reset_achievements(app, Some(vec![name.to_string()]));
            } else if let Some(item) = id.strip_prefix("mod-remove:") {
                remove_emu_mod(app, item.to_string());
            } else {
                return false;
            }
        }
    }
    true
}

pub fn changed(app: &mut App, id: &str) -> bool {
    match id {
        "emu-font-system" => set_emu_system_font(app),
        "emu-method-replace" | "emu-method-coldclient" => sync_emu_method_ui(app),
        "emu-cold-exe" => {}
        _ => return false,
    }
    true
}

pub fn input(_app: &mut App, id: &str) -> bool {
    matches!(id, "emu-broadcasts" | "emu-mod-id")
}

fn picture(app: &App, path: &str, class: &str) -> String {
    match app.state.emu.pictures.get(path).filter(|url| !url.is_empty()) {
        Some(url) => format!(r#"<span class="{class}"><img src="{}" style="width:100%;height:100%;object-fit:cover;border-radius:inherit" data-hide-on-error></span>"#, escape_html(url)),
        None => format!(r#"<span class="{class}"></span>"#),
    }
}

fn locale_date_time(app: &App, seconds: i64) -> String {
    match chrono::DateTime::from_timestamp(seconds, 0) {
        Some(utc) => {
            let local = utc.with_timezone(&chrono::Local);
            format!("{}, {}", date_medium(app.i18n.code(), &local), time_short(app.i18n.code(), &local))
        }
        None => "–".to_string(),
    }
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    let markup: String = match id {
        "emu-saves-list" => {
            let folders = &app.state.emu.save_folders;
            if folders.is_empty() {
                format!(r#"<p class="dd-path__hint">{}</p>"#, escape_html(app.i18n.t("emulator.saves.none")))
            } else {
                folders
                    .iter()
                    .map(|folder| {
                        let path = escape_html(&str_of(folder, "path"));
                        let active = app.state.emu.save_folder.as_ref().is_some_and(|current| current["path"] == folder["path"]);
                        let meta = if folder["exists"] == true {
                            let date = folder["modified"].as_i64().filter(|m| *m > 0).map(|m| locale_date_time(app, m)).unwrap_or_else(|| "–".to_string());
                            t!(app, "emulator.saves.meta", files = str_of(folder, "files"), size = folder["size_bytes"].as_f64().and_then(format_bytes).unwrap_or_else(|| "0 B".to_string()), date = date)
                        } else {
                            t!(app, "emulator.saves.empty")
                        };
                        let place = if folder["portable"] == true { "portable".to_string() } else { str_of(folder, "place") };
                        format!(
                            r#"<button class="emu-extra-row{}" data-click="save-folder:{path}"><span class="emu-extra-row__main"><span class="emu-extra-row__title" title="{path}">{path}</span><span class="emu-extra-row__meta">{}</span></span><span class="emu-file-tag">{}</span></button>"#,
                            if active { " is-active" } else { "" },
                            escape_html(&meta),
                            escape_html(app.i18n.t(&format!("emulator.saves.place.{place}"))),
                        )
                    })
                    .collect()
            }
        }
        "emu-ach-list" => app
            .state
            .emu
            .achievements
            .iter()
            .map(|row| {
                let earned = row["earned"] == true;
                let when = row["earned_time"].as_i64().filter(|time| earned && *time > 0).and_then(|time| chrono::DateTime::from_timestamp(time, 0)).map(|utc| date_medium(app.i18n.code(), &utc.with_timezone(&chrono::Local)));
                let description = if row["hidden"] == true && !earned { app.i18n.t("emulator.achievements.hidden").to_string() } else { str_of(row, "description") };
                let state = if earned {
                    match when {
                        Some(date) => t!(app, "emulator.achievements.earnedOn", date = date),
                        None => t!(app, "emulator.achievements.earned"),
                    }
                } else {
                    t!(app, "emulator.achievements.locked")
                };
                let lock = if earned {
                    format!(r#"<button class="btn btn--small btn--outline" data-click="ach-lock:{}">{}</button>"#, escape_html(&str_of(row, "name")), escape_html(app.i18n.t("emulator.achievements.lock")))
                } else {
                    String::new()
                };
                format!(
                    r#"<div class="emu-ach{}">{}<span class="emu-ach__text"><span class="emu-ach__name">{}</span><span class="emu-ach__desc">{}</span></span><span class="emu-ach__state">{}</span>{lock}</div>"#,
                    if earned { " emu-ach--earned" } else { "" },
                    picture(app, &str_of(row, "icon"), "emu-ach__icon"),
                    escape_html(&str_of(row, "display_name")),
                    escape_html(&description),
                    escape_html(&state),
                )
            })
            .collect(),
        "emu-mods-list" => app
            .state
            .emu
            .mods
            .iter()
            .map(|item| {
                let meta = t!(app, "emulator.mods.meta", id = str_of(item, "id"), files = str_of(item, "files"), size = item["size_bytes"].as_f64().and_then(format_bytes).unwrap_or_else(|| "0 B".to_string()));
                format!(
                    r#"<div class="emu-extra-row emu-mod">{}<span class="emu-extra-row__main"><span class="emu-extra-row__title">{}</span><span class="emu-extra-row__meta">{}</span></span><button class="btn btn--small btn--outline" data-click="mod-remove:{}">{}</button></div>"#,
                    picture(app, &str_of(item, "preview"), "emu-mod__img"),
                    escape_html(&str_of(item, "title")),
                    escape_html(&meta),
                    escape_html(&str_of(item, "id")),
                    escape_html(app.i18n.t("emulator.mods.remove")),
                )
            })
            .collect(),
        "emu-avatar-preview" => {
            let avatar = str_of(&app.state.emu.overlay_assets, "avatar");
            if let Some(url) = app.state.emu.pictures.get(&avatar).filter(|url| !url.is_empty()).cloned() {
                dom.image(el("img").css("width", "100%").css("height", "100%").css("object-fit", "cover").css("border-radius", "inherit"), &url, true);
            }
            return true;
        }
        _ => return false,
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}
