use std::collections::HashMap;

use serde_json::{json, Value};

use super::util::{capitalize, escape_html, file_name, format_bytes, icon, opt_str, str_of, ICON_CHECK, ICON_REFRESH, ICON_UPLOAD};
use super::{account, download_control, progress, queue, steps, telemetry, ui};
use crate::app::state::{Depot, ParsedData};
use crate::app::App;
use crate::dom::{el, Dom};
use crate::platform::FileFilter;
use crate::t;

pub const MH_APIKEY_STORAGE_KEY: &str = "manifestHubApiKey";

pub fn load_settings_and_defaults(app: &mut App) {
    app.tasks.call("get_settings", Value::Null, |app, settings: Result<Value, String>| {
        let Ok(settings) = settings else { return };
        app.state.default_download_dir = str_of(&settings, "download_location");
        app.state.notification_sound_enabled = settings["notification_sound"].as_bool() != Some(false);
        let dir = app.state.default_download_dir.clone();
        app.ui.set_value("download-dir", dir);
    });
}

pub fn get_download_dir(app: &App) -> String {
    let value = app.ui.value("download-dir").trim();
    if value.is_empty() {
        app.state.default_download_dir.clone()
    } else {
        value.to_string()
    }
}

pub fn save_download_dir(app: &mut App) {
    let dir = get_download_dir(app);
    if dir.is_empty() {
        return;
    }
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Ok(mut settings) = tasks.invoke::<Value>("get_settings", Value::Null).await {
            settings["download_location"] = json!(dir);
            let _ = tasks.invoke_raw("save_settings", json!({ "settings": settings })).await;
        }
    });
}

pub fn browse_download_dir(app: &mut App) {
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Some(selected) = platform.pick_folder(None).await {
            let selected = selected.to_string_lossy().to_string();
            tasks.post(move |app| app.ui.set_value("download-dir", selected));
        }
    });
}

pub fn go_back_to_select(app: &mut App) {
    progress::cleanup_progress_listener(app);
    app.state.job_id = None;
    steps::go_to_step(app, 2);
}

fn fetch_game_info(app: &mut App, app_id: String) {
    app.ui.hide("game-info-banner");
    app.ui.show("game-info-loading");
    let asked = app_id.clone();
    app.tasks.call("get_steam_app_info", json!({ "appId": app_id }), move |app, info: Result<Value, String>| {
        if let Some(info) = info.as_ref().ok().filter(|info| !info.is_null()) {
            app.state.game_denuvo = info["denuvo"].as_bool().unwrap_or(false);
            render_denuvo_note(app, &asked);
            if let Some(image) = opt_str(info, "headerImage") {
                app.ui.set_attr("game-header-image", "src", image.clone());
                app.state.header_image = Some(image);
            }
            if let Some(name) = opt_str(info, "name") {
                app.ui.set_text("game-name", name.clone());
                app.state.game_name = Some(name);
            }
            if let Some(description) = opt_str(info, "shortDescription") {
                app.ui.set_text("game-description", description);
            }
            app.ui.hide("game-info-loading");
            app.ui.show("game-info-banner");
            return;
        }
        app.ui.hide("game-info-loading");
    });
}

pub fn report_denuvo(app: &mut App, place: &str, app_id: &str) {
    let key = format!("{place}:{app_id}");
    if !app.state.reported_denuvo.insert(key) {
        return;
    }
    app.emit_event("denuvo_notice", json!({ "where": place }));
}

fn render_denuvo_note(app: &mut App, app_id: &str) {
    app.ui.set_hidden("denuvo-note", !app.state.game_denuvo);
    if app.state.game_denuvo {
        report_denuvo(app, "select", app_id);
    }
}

pub fn show_selection_step(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    render_denuvo_note(app, &data.main_app_id);
    app.ui.set_text("app-id-display", data.main_app_id.clone());
    let found = t!(app, "select.depotsFound", count = data.depots.len());
    app.ui.set_text("depot-count", found);

    if app.state.mode != "search" || app.state.game_name.is_none() {
        fetch_game_info(app, data.main_app_id.clone());
    } else {
        if let Some(image) = app.state.header_image.clone() {
            app.ui.set_attr("game-header-image", "src", image);
        }
        if let Some(name) = app.state.game_name.clone() {
            app.ui.set_text("game-name", name);
        }
        app.ui.hide("game-info-loading");
        if app.state.header_image.is_some() || app.state.game_name.is_some() {
            app.ui.show("game-info-banner");
        }
    }

    if let Some(saved) = app.store.get(MH_APIKEY_STORAGE_KEY).filter(|key| !key.is_empty()) {
        app.ui.set_value("mh-apikey", saved);
    }
    if !app.state.default_download_dir.is_empty() && app.ui.value("download-dir").is_empty() {
        let dir = app.state.default_download_dir.clone();
        app.ui.set_value("download-dir", dir);
    }

    app.ui.hide("latest-manifests-note");
    app.state.selected_depots.clear();
    account::init_branch_section(app);
    cancel_auto_start(app);
    app.state.depot_auto_pending = !app.state.auto_select_all_on_step2;
    app.state.depot_selection_touched = false;
    app.state.depot_pics_list = None;
    app.state.depot_skip_reasons.clear();
    app.ui.hide("auto-select-note");
    app.ui.set_disabled("btn-auto-select", true);
    app.state.depot_include_dlc = None;
    sync_steam_buttons(app);

    app.state.depot_manifests.clear();
    app.state.depot_custom.clear();
    app.state.depot_latest.clear();
    app.state.depot_status.clear();
    app.state.depot_fetching.clear();
    app.state.depot_empty_note.clear();
    app.state.depot_nokey_removed.clear();
    app.state.depot_tags.clear();
    app.state.depot_name_shown.clear();
    app.state.depot_badge_shown.clear();
    app.state.mark_keyless = data.depots.iter().any(|depot| depot_has_key(app, depot));
    app.state.depot_order = data.depots.iter().map(|depot| depot.depot_id.clone()).collect();

    app.ui.set_value("depotSearch", "");
    app.ui.set_checked("showSelectedOnly", false);

    if app.state.auto_select_all_on_step2 {
        app.state.auto_select_all_on_step2 = false;
        match app.state.auto_select_depot_ids.take().filter(|ids| !ids.is_empty()) {
            Some(ids) => {
                for depot in &data.depots {
                    if ids.contains(&depot.depot_id) {
                        app.state.selected_depots.insert(depot.depot_id.clone());
                    }
                }
            }
            None => select_all(app),
        }
    }

    update_download_button(app);
    offer_steam_keys(app);
    steps::go_to_step(app, 2);
    fetch_depot_metadata_async(app, data.main_app_id.clone());
    fetch_depot_names_from_steam(app, data.main_app_id);
}

fn fetch_depot_metadata_async(app: &mut App, app_id: String) {
    app.tasks.call("fetch_depot_metadata", json!({ "appId": app_id }), |app, depots: Result<Vec<Value>, String>| {
        for info in depots.unwrap_or_default() {
            render_depot_tags(app, &info);
        }
    });
}

fn fetch_depot_names_from_steam(app: &mut App, app_id: String) {
    let asked = app_id.clone();
    app.tasks.call("fetch_depot_metadata_steam", json!({ "appId": app_id }), move |app, depots: Result<Vec<Value>, String>| {
        let depots = match depots {
            Ok(depots) => depots,
            Err(_) => {
                app.state.depot_auto_pending = false;
                return;
            }
        };
        if app.state.parsed_data.as_ref().map(|data| data.main_app_id.as_str()) != Some(asked.as_str()) {
            return;
        }
        app.state.depot_pics_info.clear();
        for meta in &depots {
            let id = str_of(meta, "depotId");
            if id.is_empty() {
                continue;
            }
            app.state.depot_pics_info.insert(id.clone(), meta.clone());
            match meta["name"].as_str().map(str::trim).filter(|name| !name.is_empty()) {
                Some(label) => {
                    app.state.depot_names.insert(id.clone(), label.to_string());
                    app.state.depot_name_shown.insert(id.clone(), label.to_string());
                }
                None => {
                    app.state.depot_name_shown.remove(&id);
                }
            }
            app.state.depot_badge_shown.insert(id.clone());
            mark_empty_manifest(app, meta);
            if meta["role"] == "redistributable" {
                app.state.depot_nokey_removed.insert(id);
            }
        }
        reorder_depot_cards(app);
        if !app.ui.hidden("steam-keys-offer") {
            offer_steam_keys(app);
        }
        app.ui.set_disabled("btn-auto-select", depots.is_empty());
        app.state.depot_pics_list = Some(depots.clone());
        sync_dlc_button(app);
        maybe_fill_latest_manifests(app, depots);
    });
}

fn mark_empty_manifest(app: &mut App, meta: &Value) {
    let gid = str_of(meta, "manifestGid");
    if meta["manifestSize"].as_u64() != Some(0) || gid.is_empty() {
        return;
    }
    let id = str_of(meta, "depotId");
    let matches = app.state.parsed_data.as_ref().and_then(|data| data.depots.iter().find(|d| d.depot_id == id)).is_some_and(|depot| depot.manifest_id == gid);
    if !matches {
        return;
    }
    let note = t!(app, "select.emptyManifest");
    app.state.depot_empty_note.insert(id, note);
}

fn latest_from_pics(depots: &[Value]) -> HashMap<String, String> {
    depots.iter().filter_map(|d| Some((opt_str(d, "depotId")?, opt_str(d, "manifestGid")?))).collect()
}

fn apply_latest_manifests(app: &mut App, latest: &HashMap<String, String>, origin: &str) -> usize {
    let Some(data) = app.state.parsed_data.clone() else { return 0 };
    let mut changed = 0;
    for depot in &data.depots {
        let id = &depot.depot_id;
        let Some(gid) = latest.get(id) else { continue };
        if app.state.depot_manifests.contains_key(id) {
            continue;
        }
        let current = app.state.depot_custom.get(id).map(|v| v.trim().to_string()).unwrap_or_default();
        let was_latest = app.state.depot_latest.contains(id);
        if !current.is_empty() && !was_latest {
            continue;
        }
        if *gid == depot.manifest_id {
            if was_latest {
                app.state.depot_custom.remove(id);
            }
            continue;
        }
        app.state.depot_custom.insert(id.clone(), gid.clone());
        app.state.depot_latest.insert(id.clone());
        changed += 1;
    }
    let note = if changed > 0 { t!(app, "select.latestApplied", count = changed) } else { t!(app, "select.latestNone") };
    app.ui.set_text("latest-manifests-note", note);
    app.ui.show("latest-manifests-note");
    if origin == "button" {
        let text = if changed > 0 { t!(app, "select.latestAppliedShort", count = changed) } else { t!(app, "select.latestNone") };
        ui::show_toast(app, text, if changed > 0 { "success" } else { "info" }, 4000);
    }
    app.emit_event("manifest_tool", json!({ "action": "fetch_latest_all", "ok": true, "origin": origin, "changed": telemetry::count_bucket(changed) }));
    changed
}

fn maybe_fill_latest_manifests(app: &mut App, depots: Vec<Value>) {
    app.tasks.call("get_settings", Value::Null, move |app, settings: Result<Value, String>| {
        let settings = settings.unwrap_or(Value::Null);
        let wanted = app.state.latest_on_select || settings["latest_manifests"].as_bool().unwrap_or(false);
        let origin = if app.state.latest_on_select { "update" } else { "setting" };
        app.state.latest_on_select = false;
        if wanted {
            apply_latest_manifests(app, &latest_from_pics(&depots), origin);
        }
        maybe_auto_select_depots(app, depots, settings);
    });
}

pub fn fetch_latest_manifests_for_all(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    app.ui.set_disabled("btn-latest-manifests", true);
    app.tasks.call("fetch_latest_manifest_ids", json!({ "appId": data.main_app_id }), |app, result: Result<Value, String>| {
        match result {
            Ok(result) => {
                let latest: HashMap<String, String> = result["manifests"]
                    .as_object()
                    .map(|map| map.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|n| n.to_string()))?))).collect())
                    .unwrap_or_default();
                apply_latest_manifests(app, &latest, "button");
            }
            Err(error) => {
                let note = t!(app, "depots.fetchLatestError", message = app.i18n.localize_error(&error));
                app.ui.set_text("latest-manifests-note", note);
                app.ui.show("latest-manifests-note");
                app.emit_event("manifest_tool", json!({ "action": "fetch_latest_all", "ok": false, "key": telemetry::error_key_or_unmatched(&error) }));
            }
        }
        app.ui.set_disabled("btn-latest-manifests", false);
    });
}

fn maybe_auto_select_depots(app: &mut App, depots: Vec<Value>, settings: Value) {
    if app.state.parsed_data.is_none() {
        return;
    }
    let pending = app.state.depot_auto_pending;
    app.state.depot_auto_pending = false;
    app.state.steam_mode = settings["auto_select_depots"].as_bool().unwrap_or(false);
    sync_steam_buttons(app);
    if !pending || !app.state.steam_mode || app.state.depot_selection_touched || depots.is_empty() {
        return;
    }
    let auto_start = settings["auto_start_download"].as_bool().unwrap_or(false);
    apply_recommended_depots(app, depots, move |app, selection| {
        let Some(selection) = selection else { return };
        let known = selection["known"].as_bool().unwrap_or(false);
        let any = selection["selected"].as_array().is_some_and(|list| !list.is_empty());
        if known && any && auto_start && !app.state.depot_selection_touched && !app.state.queue_running {
            schedule_auto_start(app);
        }
    });
}

pub fn apply_recommended_depots(app: &mut App, depots: Vec<Value>, then: impl FnOnce(&mut App, Option<Value>) + Send + 'static) {
    let Some(data) = app.state.parsed_data.clone() else { return then(app, None) };
    let serial = app.state.parsed_serial;
    let args = json!({
        "depots": depots,
        "candidates": data.depots.iter().map(|d| d.depot_id.clone()).collect::<Vec<_>>(),
        "keyed": data.depots.iter().filter(|d| depot_has_key(app, d)).map(|d| d.depot_id.clone()).collect::<Vec<_>>(),
        "uiLanguage": app.i18n.code(),
        "includeDlc": app.state.depot_include_dlc,
    });
    let total = data.depots.len();
    app.tasks.call("recommend_depots", args, move |app, selection: Result<Value, String>| {
        let selection = match selection {
            Ok(selection) if selection["selected"].is_array() && app.state.parsed_serial == serial => selection,
            _ => return then(app, None),
        };
        let id_of = |v: &Value| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string());
        app.state.selected_depots = selection["selected"].as_array().map(|list| list.iter().map(id_of).collect()).unwrap_or_default();
        app.state.depot_skip_reasons.clear();
        let skipped = selection["skipped"].as_array().cloned().unwrap_or_default();
        for entry in &skipped {
            app.state.depot_skip_reasons.insert(str_of(entry, "depotId"), str_of(entry, "reason"));
        }
        reorder_depot_cards(app);
        update_download_button(app);
        render_auto_select_note(app, &selection, total);
        if app.state.depot_include_dlc.is_none() {
            app.state.depot_include_dlc = Some(!skipped.iter().any(|s| s["reason"] == "dlc") && !candidate_dlc_depots(app).is_empty());
        }
        sync_dlc_button(app);
        then(app, Some(selection));
    });
}

fn key_listed(app: &App, depot: &Depot) -> bool {
    if app.state.mode != "search" {
        return false;
    }
    app.state.search_key_vdf_keys.as_ref().is_some_and(|keys| !keys[depot.depot_id.as_str()].is_null() && keys[depot.depot_id.as_str()] != "")
}

fn depot_known_key(app: &App, depot: &Depot) -> bool {
    depot.depot_key.is_some() || key_listed(app, depot)
}

fn is_redist_depot(app: &App, depot_id: &str) -> bool {
    app.state.depot_pics_info.get(depot_id).is_some_and(|info| info["role"] == "redistributable")
}

fn depots_without_key(app: &App) -> Vec<Depot> {
    app.state.parsed_data.as_ref().map(|data| data.depots.iter().filter(|d| !depot_known_key(app, d) && !is_redist_depot(app, &d.depot_id)).cloned().collect()).unwrap_or_default()
}

fn set_steam_keys_offer(app: &mut App, html: String, tone: &str) {
    let classes: &[&'static str] = match tone {
        "success" => &["steam-keys-offer", "steam-keys-offer--success"],
        "warning" => &["steam-keys-offer", "steam-keys-offer--warning"],
        _ => &["steam-keys-offer"],
    };
    app.ui.set_classes("steam-keys-offer", classes);
    app.ui.set_hidden("steam-keys-offer", html.is_empty());
    app.ui.set_html("steam-keys-offer", html);
}

fn offer_steam_keys(app: &mut App) {
    let missing = depots_without_key(app);
    let Some(data) = app.state.parsed_data.as_ref() else { return set_steam_keys_offer(app, String::new(), "") };
    if missing.is_empty() || app.state.steam_keys_asked.as_deref() == Some(data.main_app_id.as_str()) {
        return set_steam_keys_offer(app, String::new(), "");
    }
    let html = format!(
        r#"<span class="steam-keys-offer__text">{}</span><span class="steam-keys-offer__actions"><button class="btn btn--small btn--primary" data-click="steam-keys:look">{}</button><button class="btn btn--small btn--outline" data-click="steam-keys:file">{}</button><button class="btn btn--small btn--outline" data-click="steam-keys:no">{}</button></span>"#,
        escape_html(&t!(app, "select.steamKeysOffer", count = missing.len())),
        escape_html(app.i18n.t("select.steamKeysLook")),
        escape_html(app.i18n.t("select.keysFromFile")),
        escape_html(app.i18n.t("select.steamKeysNo")),
    );
    set_steam_keys_offer(app, html, "");
}

fn apply_found_keys(app: &mut App, missing: &[Depot], keys: &Value) -> usize {
    let mut found = 0;
    for depot in missing {
        let Some(key) = keys[depot.depot_id.as_str()].as_str().filter(|key| !key.is_empty()) else { continue };
        if let Some(data) = app.state.parsed_data.as_mut() {
            if let Some(target) = data.depots.iter_mut().find(|d| d.depot_id == depot.depot_id) {
                target.depot_key = Some(key.to_string());
            }
        }
        found += 1;
        app.state.depot_nokey_removed.insert(depot.depot_id.clone());
    }
    found
}

fn load_keys_from_file(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    let serial = app.state.parsed_serial;
    let filters = vec![FileFilter { name: t!(app, "select.keyFilesFilter"), extensions: ["json", "csv", "vdf", "txt"].map(String::from).to_vec() }];
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(path) = platform.pick_file(None, filters).await else { return };
        let keys = tasks.invoke::<Value>("read_key_file", json!({ "path": path.to_string_lossy() })).await;
        tasks.post(move |app| match keys {
            Ok(keys) => {
                if app.state.parsed_serial != serial {
                    return;
                }
                let missing = depots_without_key(app);
                app.state.steam_keys_asked = Some(data.main_app_id.clone());
                let found = apply_found_keys(app, &missing, &keys);
                app.emit_event("key_file_used", json!({ "found": found, "missing": missing.len() - found, "where": "select" }));
                if found > 0 {
                    let text = escape_html(&t!(app, "select.keyFileFound", found = found, count = missing.len()));
                    set_steam_keys_offer(app, text, "success");
                    update_download_button(app);
                } else {
                    let text = escape_html(app.i18n.t("select.keyFileNone"));
                    set_steam_keys_offer(app, text, "warning");
                }
            }
            Err(error) => {
                let text = escape_html(&app.i18n.localize_error(&error));
                set_steam_keys_offer(app, text, "warning");
            }
        });
    });
}

fn look_for_steam_keys(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    let app_id = data.main_app_id.clone();
    app.state.steam_keys_asked = Some(app_id.clone());
    let missing = depots_without_key(app);
    let ids: Vec<String> = missing.iter().map(|d| d.depot_id.clone()).collect();
    app.tasks.call("local_steam_depot_keys", json!({ "depotIds": ids }), move |app, result: Result<Value, String>| {
        let result = result.unwrap_or_else(|_| json!({ "steamFound": false, "keys": {} }));
        if app.state.parsed_data.as_ref().map(|d| d.main_app_id.as_str()) != Some(app_id.as_str()) {
            return;
        }
        let found = apply_found_keys(app, &missing, &result["keys"]);
        app.emit_event("steam_keys_used", json!({ "found": found, "missing": missing.len() - found }));
        if !result["steamFound"].as_bool().unwrap_or(false) {
            let text = escape_html(app.i18n.t("select.steamKeysNoSteam"));
            set_steam_keys_offer(app, text, "warning");
        } else if found > 0 {
            let text = escape_html(&t!(app, "select.steamKeysFound", found = found, count = missing.len()));
            set_steam_keys_offer(app, text, "success");
            update_download_button(app);
        } else {
            let text = escape_html(app.i18n.t("select.steamKeysNone"));
            set_steam_keys_offer(app, text, "warning");
        }
    });
}

pub fn depot_has_key(app: &App, depot: &Depot) -> bool {
    depot.depot_key.is_some() || app.state.account_mode || key_listed(app, depot)
}

fn candidate_dlc_depots(app: &App) -> Vec<String> {
    let Some(data) = app.state.parsed_data.as_ref() else { return Vec::new() };
    data.depots
        .iter()
        .filter(|d| app.state.depot_pics_info.get(&d.depot_id).is_some_and(|info| info["role"] == "dlc" && !info["optionalDlc"].as_bool().unwrap_or(false)))
        .map(|d| d.depot_id.clone())
        .collect()
}

pub fn sync_steam_buttons(app: &mut App) {
    app.ui.set_hidden("btn-auto-select", !app.state.steam_mode);
    sync_dlc_button(app);
}

fn sync_dlc_button(app: &mut App) {
    let show = app.state.steam_mode && app.state.depot_pics_list.is_some() && !candidate_dlc_depots(app).is_empty();
    app.ui.set_hidden("btn-auto-select-dlc", !show);
    let label = t!(app, if app.state.depot_include_dlc == Some(true) { "select.autoSelectNoDlc" } else { "select.autoSelectDlc" });
    app.ui.set_text("btn-auto-select-dlc", label);
}

pub fn steam_language_name(app: &App, code: &str) -> String {
    let key = format!("steamLanguages.{code}");
    if app.i18n.has(&key) {
        app.i18n.t(&key).to_string()
    } else {
        capitalize(code)
    }
}

fn platform_name(platform: &str) -> String {
    platform
        .split(',')
        .filter(|p| !p.is_empty())
        .map(|p| match p {
            "windows" => "Windows",
            "linux" => "Linux",
            "macos" => "macOS",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn render_auto_select_note(app: &mut App, selection: &Value, total: usize) {
    if !selection["known"].as_bool().unwrap_or(false) {
        return app.ui.hide("auto-select-note");
    }
    let count = selection["selected"].as_array().map(Vec::len).unwrap_or(0);
    let title = t!(app, "select.autoSelectTitle", count = count, total = total);
    app.ui.set_text("auto-select-title", title);
    let mut counts: Vec<(String, usize)> = Vec::new();
    for entry in selection["skipped"].as_array().into_iter().flatten() {
        let reason = str_of(entry, "reason");
        match counts.iter_mut().find(|(r, _)| *r == reason) {
            Some((_, n)) => *n += 1,
            None => counts.push((reason, 1)),
        }
    }
    let platform = platform_name(&str_of(selection, "platform"));
    let mut parts = vec![format!("{} · {}", platform, steam_language_name(app, &str_of(selection, "language")))];
    let skipped: Vec<String> = counts.iter().map(|(reason, n)| format!("{}× {}", n, app.i18n.t(&format!("select.skipReason.{reason}")))).collect();
    if !skipped.is_empty() {
        parts.push(t!(app, "select.autoSelectSkipped", list = skipped.join(", ")));
    }
    if selection["fallback"].as_bool().unwrap_or(false) {
        parts.push(t!(app, "select.platformFallback", platform = platform));
    }
    app.ui.set_text("auto-select-detail", parts.join(" — "));
    app.ui.hide("btn-auto-start-cancel");
    app.ui.show("auto-select-note");
}

fn auto_start_tick(app: &mut App, token: u64, seconds: u32) {
    if app.state.auto_start_timer != Some(token) {
        return;
    }
    if seconds == 0 {
        cancel_auto_start(app);
        if app.state.current_step == 2 && !app.state.selected_depots.is_empty() {
            start_download(app);
        }
        return;
    }
    let label = t!(app, "select.autoStartIn", seconds = seconds);
    app.ui.set_text("btn-auto-start-cancel", label);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(1000).await;
        tasks.post(move |app| auto_start_tick(app, token, seconds - 1));
    });
}

fn schedule_auto_start(app: &mut App) {
    cancel_auto_start(app);
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.auto_start_timer = Some(token);
    app.ui.show("btn-auto-start-cancel");
    auto_start_tick(app, token, 5);
}

pub fn cancel_auto_start(app: &mut App) {
    app.state.auto_start_timer = None;
    app.ui.hide("btn-auto-start-cancel");
}

pub fn mark_depot_selection_touched(app: &mut App) {
    app.state.depot_selection_touched = true;
    app.state.depot_auto_pending = false;
    cancel_auto_start(app);
}

fn host_os() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "windows"
    }
}

fn depot_role_sort_key(app: &App, depot_id: &str) -> u32 {
    let Some(info) = app.state.depot_pics_info.get(depot_id) else { return 100 };
    match info["role"].as_str().unwrap_or("") {
        "shared_content" => 10,
        "platform" => {
            let os = str_of(info, "oslist").to_lowercase();
            if os.contains(host_os()) {
                20
            } else if os.contains("windows") {
                30
            } else if os.contains("linux") {
                31
            } else if os.contains("mac") {
                32
            } else {
                33
            }
        }
        "language" => 50,
        "dlc" => 60,
        "redistributable" => 90,
        _ => 80,
    }
}

fn reorder_depot_cards(app: &mut App) {
    let mut order = std::mem::take(&mut app.state.depot_order);
    let key = |app: &App, id: &str| {
        let skipped = if app.state.depot_skip_reasons.contains_key(id) { 200 } else { 0 };
        (skipped + depot_role_sort_key(app, id), id.parse::<u64>().unwrap_or(u64::MAX))
    };
    order.sort_by_key(|id| key(app, id));
    app.state.depot_order = order;
}

fn depot_role_badge(app: &App, info: &Value) -> Option<(String, String, String)> {
    let role = info["role"].as_str()?;
    let (text, title) = match role {
        "dlc" => (app.i18n.t("depots.roleDlc").to_string(), String::new()),
        "language" => match opt_str(info, "language") {
            Some(language) => (t!(app, "depots.roleLanguageWithName", name = steam_language_name(app, &language.to_lowercase())), String::new()),
            None => (app.i18n.t("depots.roleLanguage").to_string(), String::new()),
        },
        "shared_content" => (app.i18n.t("depots.roleContent").to_string(), String::new()),
        "platform" => (app.i18n.t("depots.rolePlatform").to_string(), String::new()),
        "redistributable" => (app.i18n.t("depots.roleRedist").to_string(), app.i18n.t("depots.roleRedistHint").to_string()),
        _ => return None,
    };
    Some((role.to_string(), text, title))
}

fn render_depot_tags(app: &mut App, info: &Value) {
    let id = str_of(info, "depot_id");
    if id.is_empty() {
        return;
    }
    let mut tags: Vec<(String, String)> = Vec::new();
    let os = str_of(info, "oslist").to_lowercase();
    if os.contains("windows") {
        tags.push(("windows".into(), t!(app, "depotTags.windows")));
    }
    if os.contains("linux") {
        tags.push(("linux".into(), t!(app, "depotTags.linux")));
    }
    if os.contains("mac") {
        tags.push(("mac".into(), t!(app, "depotTags.macos")));
    }
    match info["osarch"].as_str() {
        Some("64") => tags.push(("arch".into(), t!(app, "depotTags.arch64"))),
        Some("32") => tags.push(("arch".into(), t!(app, "depotTags.arch32"))),
        _ => {}
    }
    if let Some(language) = opt_str(info, "language") {
        tags.push(("lang".into(), steam_language_name(app, &language.to_lowercase())));
    }
    app.state.depot_tags.insert(id, tags);
}

fn toggle_depot(app: &mut App, depot_id: &str) {
    mark_depot_selection_touched(app);
    if !app.state.selected_depots.remove(depot_id) {
        app.state.selected_depots.insert(depot_id.to_string());
    }
    update_download_button(app);
}

pub fn select_all(app: &mut App) {
    if let Some(data) = app.state.parsed_data.as_ref() {
        app.state.selected_depots = data.depots.iter().map(|d| d.depot_id.clone()).collect();
    }
    update_download_button(app);
}

pub fn deselect_all(app: &mut App) {
    app.state.selected_depots.clear();
    update_download_button(app);
}

pub fn update_download_button(app: &mut App) {
    let count = app.state.selected_depots.len();
    app.ui.set_disabled("btn-download", count == 0);
    app.ui.set_disabled("btn-queue-add", count == 0);
    let total: u64 = app
        .state
        .parsed_data
        .as_ref()
        .map(|data| data.depots.iter().filter(|d| app.state.selected_depots.contains(&d.depot_id)).filter_map(|d| d.size_bytes).sum())
        .unwrap_or(0);
    let label = if count == 0 {
        t!(app, "select.downloadSelected")
    } else if total > 0 {
        t!(app, "select.downloadCountSize", count = count, size = format_bytes(total as f64).unwrap_or_default())
    } else {
        t!(app, "select.downloadCount", count = count)
    };
    app.ui.set_html("btn-download", format!(r#"{} <svg class="btn__icon" data-icon="download" data-butt></svg>"#, escape_html(&label)));
}

pub fn collect_depot_configs(app: &App, selected: &[Depot], repair: Option<&Value>) -> Vec<Value> {
    selected
        .iter()
        .map(|depot| {
            let custom = app.state.depot_custom.get(&depot.depot_id).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
            let mut result = json!({
                "depotId": depot.depot_id,
                "manifestId": depot.manifest_id,
                "depotKey": depot.depot_key,
                "sizeBytes": depot.size_bytes,
                "customManifestId": custom,
                "displayName": app.state.depot_names.get(&depot.depot_id),
            });
            if let Some(manifest) = repair.and_then(|repair| repair[depot.depot_id.as_str()].as_str()) {
                result["customManifestId"] = json!(manifest);
            }
            if let Some((original, stored)) = app.state.depot_manifests.get(&depot.depot_id) {
                result["uploadedManifestPath"] = json!(stored);
                if result["customManifestId"].is_null() {
                    if let Some(id) = manifest_id_from_name(original) {
                        result["customManifestId"] = json!(id);
                    }
                }
            }
            result
        })
        .collect()
}

fn manifest_id_from_name(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".manifest")?;
    let (depot, manifest) = stem.split_once('_')?;
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    (digits(depot) && digits(manifest)).then(|| manifest.to_string())
}

pub fn build_download_config(app: &App, data: &ParsedData, depots: &[Value], mh_api_key: &str, repair: Option<&Value>) -> Value {
    let download_dir = get_download_dir(app);
    let mut config = json!({
        "mainAppId": data.main_app_id,
        "selectedDepots": depots,
        "manifestHubApiKey": (!mh_api_key.is_empty()).then_some(mh_api_key),
        "downloadDir": (!download_dir.is_empty()).then_some(download_dir),
        "gameName": app.state.game_name,
        "headerImage": app.state.header_image,
        "updateDir": progress::active_update_dir(app, &data.main_app_id),
        "repair": repair.is_some(),
        "allAppIds": data.all_app_ids,
        "includeDlc": app.state.depot_include_dlc,
        "mode": if app.state.mode == "search" { "search" } else { "upload" },
    });
    telemetry::merge(&mut config, account::account_download_fields(app));
    if app.state.mode == "search" {
        if let Some(repo) = &app.state.search_repo {
            config["repo"] = json!(repo);
        }
        if let Some(sha) = &app.state.search_sha {
            config["sha"] = json!(sha);
        }
        if let Some(keys) = &app.state.search_key_vdf_keys {
            config["keyVdfKeys"] = keys.clone();
        }
        if let Some(kind) = app.state.selected_repo.and_then(|index| app.state.search_repos.get(index)).map(|repo| repo.kind.clone()).filter(|kind| !kind.is_empty()) {
            config["sourceType"] = json!(kind);
        }
    }
    config
}

pub fn remember_download_inputs(app: &mut App) -> String {
    let key = app.ui.value("mh-apikey").trim().to_string();
    progress::hide_mh_key_required_hint(app);
    if key.is_empty() {
        app.store.remove(MH_APIKEY_STORAGE_KEY);
    } else {
        app.store.set(MH_APIKEY_STORAGE_KEY, &key);
    }
    save_download_dir(app);
    key
}

pub fn start_download(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    let selected: Vec<Depot> = data.depots.iter().filter(|d| app.state.selected_depots.contains(&d.depot_id)).cloned().collect();
    if selected.is_empty() {
        return;
    }
    if let Some(problem) = account::branch_start_problem(app) {
        let text = t!(app, problem);
        return ui::show_toast(app, text, "error", 6000);
    }
    let update_dir = progress::active_update_dir(app, &data.main_app_id);
    let needed: u64 = selected.iter().filter_map(|d| d.size_bytes).sum();
    download_control::confirm_free_space(app, needed, update_dir.is_some(), move |app, ok| {
        if !ok {
            return;
        }
        let key = remember_download_inputs(app);
        let repair = progress::active_repair(app, &data.main_app_id);
        app.state.repair_running = repair.is_some();
        let depots = collect_depot_configs(app, &selected, repair.as_ref());
        let config = build_download_config(app, &data, &depots, &key, repair.as_ref());
        run_download(app, config, depots, false);
    });
}

fn download_context(app: &App, config: &Value, depots: &[Value], from_queue: bool) -> Value {
    let keys = &config["keyVdfKeys"];
    let selection = if from_queue {
        "queued"
    } else if app.state.depot_selection_touched {
        "manual"
    } else if app.state.steam_mode {
        "like_steam"
    } else {
        "default"
    };
    let keyless = depots.iter().filter(|d| d["depotKey"].is_null() && keys[d["depotId"].as_str().unwrap_or("")].is_null()).count();
    json!({
        "mode": if config["repair"].as_bool().unwrap_or(false) { "repair" } else if !config["updateDir"].is_null() { "update" } else { "new" },
        "selection": selection,
        "source": config["sourceType"].as_str().unwrap_or(if app.state.mode == "search" { "search" } else { "upload" }),
        "queue": from_queue,
        "dlc": config["includeDlc"].as_bool(),
        "keyless": telemetry::count_bucket(keyless),
        "custom_manifest": depots.iter().any(|d| !d["customManifestId"].is_null()),
        "account": config["useAccount"].as_bool().unwrap_or(false),
        "branch": if config["branch"].is_null() || config["branch"] == "" { "public" } else if !config["branchPassword"].is_null() && config["branchPassword"] != "" { "password" } else { "other" },
    })
}

pub fn run_download(app: &mut App, config: Value, depots: Vec<Value>, from_queue: bool) {
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let settings = tasks.invoke::<Value>("get_settings", Value::Null).await;
        let pending = tasks.ask(|app| app.state.pending_history_entry.take()).await.flatten();
        if let Some(entry) = pending {
            let _ = tasks.invoke_raw("record_history_entry", json!({ "entry": entry })).await;
        }
        let start_config = config.clone();
        let started = tasks
            .ask(move |app| {
                let (engine, source_count) = match &settings {
                    Ok(s) => (if s["use_native_downloader"].as_bool() != Some(false) { "native" } else { "ddm" }, s["depot_sources"].as_array().map(Vec::len)),
                    Err(_) => ("native", None),
                };
                app.state.current_engine = Some(engine.to_string());
                app.state.dl_nonce = Some(telemetry::telemetry_nonce());
                app.state.dl_source_count = source_count;
                app.state.dl_had_mh_key = !config["manifestHubApiKey"].is_null();
                app.state.dl_context = Some(download_context(app, &config, &depots, from_queue));
                let props = telemetry::merged([
                    json!({ "job": app.state.dl_nonce, "depot_count": depots.len(), "engine": engine, "source_count": source_count, "had_mh_key": app.state.dl_had_mh_key }),
                    app.state.dl_context.clone().unwrap_or(Value::Null),
                ]);
                app.emit_event("download_started", props);
                app.ui.hide("btn-report-failure");
                steps::go_to_step(app, 3);
                progress::init_progress_ui(app, &depots);
                app.state.download_started_at = Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
                app.state.dl_started_at = Some(std::time::Instant::now());
                app.state.dl_stage = Some("starting".to_string());
                app.state.dl_depot_count = Some(depots.len());
                progress::connect_progress_listener(app);
            })
            .await;
        if started.is_none() {
            return;
        }
        let result = tasks.invoke::<Value>("start_download", json!({ "config": start_config })).await;
        tasks.post(move |app| match result {
            Ok(result) => {
                if app.state.job_id.is_none() {
                    app.state.job_id = opt_str(&result, "jobId");
                }
                app.state.download_dir = opt_str(&result, "downloadDir");
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                let line = t!(app, "progress.errorLine", message = text);
                progress::append_terminal_line(app, line, "error");
                let props = telemetry::merged([
                    json!({ "success": false }),
                    telemetry::job_context(app),
                    json!({
                        "outcome": "failed",
                        "depots_total": app.state.dl_depot_count.unwrap_or(0),
                        "depots_ok": 0,
                        "duration_bucket": telemetry::elapsed_bucket(app),
                        "engine": app.state.current_engine.clone().unwrap_or_else(|| "native".to_string()),
                        "err_key": telemetry::error_key_or_unmatched(&error),
                    }),
                    telemetry::classify_start_failure(&error),
                ]);
                app.emit_event("download_completed", props);
                progress::show_completion(app, false, text);
                queue::queue_after_download(app, "failed");
            }
        });
    });
}

pub fn handle_depot_manifest_file(app: &mut App, depot_id: String) {
    let filters = vec![FileFilter { name: t!(app, "select.manifestFilesFilter"), extensions: vec!["manifest".to_string()] }];
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(path) = platform.pick_file(None, filters).await else { return };
        let path = path.to_string_lossy().to_string();
        tasks.post(move |app| {
            let name = file_name(&path);
            app.state.depot_manifests.insert(depot_id.clone(), (name.clone(), path));
            app.emit_event("manifest_tool", json!({ "action": "upload", "ok": true }));
            app.state.depot_status.insert(depot_id, format!(r#"<span class="manifest-uploaded">{} {}</span>"#, icon(ICON_CHECK), escape_html(&name)));
        });
    });
}

pub fn fetch_latest_manifest_for_depot(app: &mut App, depot_id: String) {
    let Some(app_id) = app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).filter(|id| !id.is_empty()) else { return };
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.depot_status_timer.insert(depot_id.clone(), token);
    app.state.depot_fetching.insert(depot_id.clone());
    app.state.depot_status.insert(depot_id.clone(), format!(r#"<span class="manifest-uploading">{}</span>"#, app.i18n.t("depots.fetchingLatest")));
    app.tasks.call("fetch_latest_manifest_id", json!({ "appId": app_id, "depotId": depot_id.clone() }), move |app, result: Result<Value, String>| {
        match result {
            Ok(result) => {
                let manifest_id = str_of(&result, "manifestId");
                let source = str_of(&result, "source");
                let steam = source == "steam";
                let label = t!(app, if steam { "depots.fetchSourceSteam" } else { "depots.fetchSourceFallback" });
                app.state.depot_custom.insert(depot_id.clone(), manifest_id.clone());
                app.emit_event("manifest_tool", json!({ "action": "fetch_latest", "ok": true, "source": if steam { "steam" } else { "fallback" } }));
                app.state.depot_status.insert(
                    depot_id.clone(),
                    format!(
                        r#"<span class="manifest-uploaded">{} {}</span> <span class="manifest-source manifest-source--{}">{}</span>"#,
                        icon(ICON_CHECK),
                        escape_html(&manifest_id),
                        escape_html(&source),
                        escape_html(&label)
                    ),
                );
            }
            Err(error) => {
                app.emit_event("manifest_tool", json!({ "action": "fetch_latest", "ok": false, "key": telemetry::error_key_or_unmatched(&error) }));
                let message = t!(app, "depots.fetchLatestError", message = app.i18n.localize_error(&error));
                app.state.depot_status.insert(depot_id.clone(), format!(r#"<span class="status-error">{}</span>"#, escape_html(&message)));
            }
        }
        app.state.depot_fetching.remove(&depot_id);
        let tasks = app.tasks.clone();
        app.tasks.spawn(async move {
            tasks.sleep(5000).await;
            tasks.post(move |app| {
                if app.state.depot_status_timer.get(&depot_id) != Some(&token) {
                    return;
                }
                match app.state.depot_manifests.get(&depot_id) {
                    Some((name, _)) => {
                        let html = format!(r#"<span class="manifest-uploaded">{} {}</span>"#, icon(ICON_CHECK), escape_html(name));
                        app.state.depot_status.insert(depot_id.clone(), html);
                    }
                    None => {
                        app.state.depot_status.remove(&depot_id);
                    }
                }
                app.state.depot_status_timer.remove(&depot_id);
            });
        });
    });
}

fn depot_matches_filters(app: &App, depot_id: &str, search: &str, selected_only: bool) -> bool {
    let info = app.state.depot_pics_info.get(depot_id);
    let field = |name: &str| info.map(|info| str_of(info, name)).unwrap_or_default();
    let haystack = [depot_id.to_string(), app.state.depot_names.get(depot_id).cloned().unwrap_or_default(), field("role"), field("oslist"), field("language")]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    (search.is_empty() || haystack.contains(search)) && (!selected_only || app.state.selected_depots.contains(depot_id))
}

fn fill_depot_list(app: &mut App, dom: &mut Dom<'_>) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    let search = app.ui.value("depotSearch").trim().to_lowercase();
    let selected_only = app.ui.checked("showSelectedOnly");
    let order = app.state.depot_order.clone();
    let placeholder = t!(app, "select.customManifestPlaceholder");
    let fetch_title = t!(app, "depots.fetchLatest");
    let upload_title = t!(app, "depots.uploadManifest");
    for depot_id in &order {
        let Some(depot) = data.depots.iter().find(|d| d.depot_id == *depot_id) else { continue };
        let id = depot.depot_id.clone();
        let safe_id = escape_html(&id);
        let visible = depot_matches_filters(app, &id, &search, selected_only);
        let skip_reason = app.state.depot_skip_reasons.get(&id).cloned();
        let mut item = el("div.depot-item").key(&id).class_if("selected", app.state.selected_depots.contains(&id)).class_if("is-skipped", skip_reason.is_some());
        if !visible {
            item = item.css("display", "none");
        }
        let name = app.state.depot_name_shown.get(&id).map(|label| format!(r#"<span class="depot-item__name-label"> — {}</span>"#, escape_html(label))).unwrap_or_default();
        let size = depot.size_bytes.and_then(|bytes| format_bytes(bytes as f64)).map(|size| format!(r#"<span class="depot-item__size">{}</span>"#, escape_html(&size))).unwrap_or_default();
        let badge = app
            .state
            .depot_pics_info
            .get(&id)
            .filter(|_| app.state.depot_badge_shown.contains(&id))
            .and_then(|info| depot_role_badge(app, info))
            .map(|(role, text, title)| format!(r#" <span class="depot-tag depot-tag--{}" title="{}">{}</span>"#, escape_html(&role), escape_html(&title), escape_html(&text)))
            .unwrap_or_default();
        let tags: String = app.state.depot_tags.get(&id).map(|tags| tags.iter().map(|(class, label)| format!(r#"<span class="depot-tag depot-tag--{}">{}</span>"#, class, escape_html(label))).collect()).unwrap_or_default();
        let skipped = skip_reason.map(|reason| format!(r#"<span class="depot-tag depot-tag--skipped">{}</span>"#, escape_html(app.i18n.t(&format!("select.skipReason.{reason}"))))).unwrap_or_default();
        let header = format!(
            r#"<div class="depot-item__header"><span class="depot-item__depot-id">Depot {safe_id}<span class="depot-item__name">{name}</span>{size}{badge}</span><span class="depot-item__tags">{tags}</span>{skipped}</div>"#
        );
        let nokey = if app.state.mark_keyless && !depot_has_key(app, depot) && !app.state.depot_nokey_removed.contains(&id) {
            format!(r#" <span class="depot-tag depot-tag--nokey" title="{}">{}</span>"#, escape_html(app.i18n.t("select.noKeyHint")), escape_html(app.i18n.t("select.skipReason.no_key")))
        } else {
            String::new()
        };
        let manifest_line = format!(r#"<div class="depot-item__manifest-id">Manifest: {}{nokey}</div>"#, escape_html(&depot.manifest_id));
        let status = app.state.depot_status.get(&id).cloned().unwrap_or_default();
        let note = app.state.depot_empty_note.get(&id).cloned().unwrap_or_default();
        let mut custom = app.state.depot_custom.get(&id).cloned().unwrap_or_default();
        let before = custom.clone();
        let note_hidden = note.is_empty() || !custom.trim().is_empty();
        let fetching = app.state.depot_fetching.contains(&id);
        let has_file = app.state.depot_manifests.contains_key(&id);
        let mut action: Option<&'static str> = None;
        let clicked = dom
            .button(item, |dom| {
                dom.empty("div.depot-item__checkbox");
                dom.el("div.depot-item__info", |dom| {
                    dom.markup(&header);
                    dom.markup(&manifest_line);
                    dom.el("div.depot-item__manifest-row", |dom| {
                        dom.input_with(el("input.custom-manifest-input[type=text]"), &mut custom, &placeholder, false);
                        let fetch = el("button.depot-manifest-action.depot-manifest-fetch-btn").title(fetch_title.clone()).class_if("depot-manifest-action--loading", fetching).state(crate::style::selector::DISABLED, fetching);
                        if dom.button(fetch, |dom| dom.icon("svg.btn-icon", crate::icons::by_name(ICON_REFRESH).expect("icon"))).clicked() && !fetching {
                            action = Some("fetch");
                        }
                        let upload = el("button.depot-manifest-action.depot-manifest-btn").title(upload_title.clone()).class_if("depot-manifest-action--active", has_file);
                        if dom.button(upload, |dom| dom.icon("svg.btn-icon", crate::icons::by_name(ICON_UPLOAD).expect("icon"))).clicked() {
                            action = Some("upload");
                        }
                    });
                    dom.el(el("span.depot-manifest-status").state(crate::style::selector::EMPTY, status.is_empty()), |dom| {
                        dom.markup(&status);
                    });
                    dom.el(el("span.depot-empty-note").class_if("hidden", note_hidden), |dom| dom.text(&note));
                });
            })
            .clicked();
        if custom != before {
            app.state.depot_latest.remove(&id);
            app.state.depot_custom.insert(id.clone(), custom);
        }
        match action {
            Some("fetch") => fetch_latest_manifest_for_depot(app, id),
            Some("upload") => handle_depot_manifest_file(app, id),
            _ if clicked => toggle_depot(app, &id),
            _ => {}
        }
    }
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-select-all" => {
            mark_depot_selection_touched(app);
            select_all(app);
        }
        "btn-deselect-all" => {
            mark_depot_selection_touched(app);
            deselect_all(app);
        }
        "btn-latest-manifests" => fetch_latest_manifests_for_all(app),
        "btn-auto-select" => {
            mark_depot_selection_touched(app);
            if let Some(list) = app.state.depot_pics_list.clone() {
                apply_recommended_depots(app, list, |_, _| {});
            }
        }
        "btn-auto-start-cancel" => cancel_auto_start(app),
        "btn-auto-select-dlc" => {
            mark_depot_selection_touched(app);
            app.state.depot_include_dlc = Some(app.state.depot_include_dlc != Some(true));
            if let Some(list) = app.state.depot_pics_list.clone() {
                apply_recommended_depots(app, list, |_, _| {});
            }
        }
        "btn-back" => steps::go_to_step(app, 1),
        "btn-download" => start_download(app),
        "btn-browse-dir" => browse_download_dir(app),
        "steam-keys:look" => look_for_steam_keys(app),
        "steam-keys:file" => load_keys_from_file(app),
        "steam-keys:no" => {
            app.state.steam_keys_asked = app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone());
            set_steam_keys_offer(app, String::new(), "");
        }
        _ => return false,
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    match id {
        "mh-apikey" => {
            if !app.ui.value("mh-apikey").trim().is_empty() {
                progress::hide_mh_key_required_hint(app);
            }
        }
        "depotSearch" | "download-dir" => {}
        _ => return false,
    }
    true
}

pub fn changed(_app: &mut App, id: &str) -> bool {
    id == "showSelectedOnly"
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "depot-list" => fill_depot_list(app, dom),
        _ => return false,
    }
    true
}
