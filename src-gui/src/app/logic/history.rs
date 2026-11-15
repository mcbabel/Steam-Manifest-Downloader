use std::collections::HashMap;
use std::time::Instant;

use serde_json::{json, Value};

use super::picker::{self, Outcome, Picker};
use super::shortcuts::exe_platform_badge;
use super::util::{date_time_medium_short, escape_html, format_bytes, icon, opt_str, parse_date, str_of, ICON_FOLDER_OPEN, ICON_GAMEPAD, ICON_PLAY, ICON_REFRESH, ICON_SETTINGS, ICON_SHIELD_CHECK, ICON_TRASH, ICON_UPDATE};
use super::{download_control, emulator, progress, search, select, steps, telemetry, ui};
use crate::app::state::ParsedData;
use crate::app::App;
use crate::dom::{el, Dom};
use crate::images::Status;
use crate::style::selector::{CHECKED, HAS_CHECKED};
use crate::t;

const VIEW_KEY: &str = "smd.historyView";
const KINDS: [&str; 5] = ["complete", "partial", "resumable", "cancelled", "failed"];
const SORTS: [&str; 4] = ["newest", "oldest", "name", "size"];
const UPDATE_CHECK_TTL_SECS: u64 = 600;
const SORT_PICKER: Picker = Picker { picker: "history-sort", button: "history-sort-button", menu: "history-sort-menu" };
const COVER_PLACEHOLDER: &str = r#"<div class="history-entry__image history-entry__image--placeholder"><svg data-icon="download" data-stroke="1.5" data-butt width="24" height="24" style="opacity:0.4"></svg></div>"#;

pub struct HistoryUi {
    pub query: String,
    pub status: String,
    pub sort: String,
    pub cached: Vec<Value>,
    pub loading: Option<bool>,
    pub use_native: bool,
    pub checks: HashMap<String, Value>,
    pub checks_key: String,
    pub checks_at: Option<Instant>,
    pub checks_pending: Option<String>,
    pub covers: HashMap<String, Option<String>>,
    pub remove_id: Option<String>,
    pub remove_resumable: bool,
    pub launch: Vec<Value>,
    pub launch_dir: String,
    pub launch_chosen: String,
    pub launch_entry: Option<Value>,
}

impl Default for HistoryUi {
    fn default() -> Self {
        Self {
            query: String::new(),
            status: "all".to_string(),
            sort: "newest".to_string(),
            cached: Vec::new(),
            loading: None,
            use_native: true,
            checks: HashMap::new(),
            checks_key: String::new(),
            checks_at: None,
            checks_pending: None,
            covers: HashMap::new(),
            remove_id: None,
            remove_resumable: false,
            launch: Vec::new(),
            launch_dir: String::new(),
            launch_chosen: String::new(),
            launch_entry: None,
        }
    }
}

pub fn open_history(app: &mut App) {
    hide_history_banner(app);
    app.emit_event("history_action", json!({ "action": "opened" }));
    app.state.history.query.clear();
    app.ui.show("history-modal");
    load_history(app);
}

pub fn close_history(app: &mut App) {
    app.ui.hide("history-modal");
}

fn load_history(app: &mut App) {
    app.state.history.loading = Some(true);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let native = tasks.invoke::<Value>("get_settings", Value::Null).await.map(|s| s["use_native_downloader"].as_bool() != Some(false)).unwrap_or(true);
        let entries = tasks.invoke::<Vec<Value>>("get_history", Value::Null).await;
        tasks.post(move |app| {
            app.state.history.use_native = native;
            match entries {
                Ok(entries) => {
                    app.state.history.loading = None;
                    render_history(app, entries);
                }
                Err(_) => app.state.history.loading = Some(false),
            }
        });
    });
}

fn load_history_view(app: &mut App) {
    let saved: Value = app.store.get(VIEW_KEY).and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or(Value::Null);
    if let Some(status) = saved["status"].as_str().filter(|s| *s == "all" || KINDS.contains(s)) {
        app.state.history.status = status.to_string();
    }
    if let Some(sort) = saved["sort"].as_str().filter(|s| SORTS.contains(s)) {
        app.state.history.sort = sort.to_string();
    }
}

fn save_history_view(app: &mut App) {
    let text = json!({ "status": app.state.history.status, "sort": app.state.history.sort }).to_string();
    app.store.set(VIEW_KEY, &text);
}

fn relative_to_dir(path: &str, dir: &str) -> String {
    let normal = |p: &str| p.replace('\\', "/").trim_end_matches('/').to_string();
    let (base, full) = (normal(dir), normal(path));
    if !base.is_empty() && full.to_lowercase().starts_with(&format!("{}/", base.to_lowercase())) {
        full[base.len() + 1..].to_string()
    } else {
        full
    }
}

fn choose_launch_exe(app: &mut App, entry: Value, candidates: Vec<Value>, dir: String) {
    let chosen = candidates.iter().find(|c| c["recommended"] == true).unwrap_or(&candidates[0]);
    app.state.history.launch_chosen = str_of(chosen, "path");
    app.state.history.launch = candidates;
    app.state.history.launch_dir = dir;
    app.state.history.launch_entry = Some(entry);
    app.ui.show("launch-modal");
}

fn play_failed(app: &mut App, error: &str) {
    if error.contains("WINDOWS_GAME_NEEDS_PROTON") || error.contains("WINDOWS_GAME_NEEDS_WINE_MAC") {
        let key = if error.contains("WINDOWS_GAME_NEEDS_WINE_MAC") { "history.playNeedsWineMac" } else { "history.playNeedsProton" };
        telemetry::report_error(app, "play", None, Some(key));
        let text = t!(app, key);
        show_history_banner(app, text, 12000, "error");
    } else {
        telemetry::report_error(app, "play", Some(error), None);
        let text = t!(app, "history.playFailed", message = app.i18n.localize_error(error));
        show_history_banner(app, text, 6000, "error");
    }
}

fn launch(app: &mut App, entry: Value, exe: String, chosen: bool) {
    let dir = str_of(&entry, "download_dir");
    app.tasks.call("launch_game", json!({ "dir": dir, "exe": exe }), move |app, method: Result<String, String>| match method {
        Ok(method) => {
            let kind = if ["steam", "wine", "native"].contains(&method.as_str()) { method.as_str() } else { "direct" };
            app.emit_event("game_launched", json!({ "method": kind, "chosen": chosen, "partial": entry["status"] == "partial" }));
            let key = match method.as_str() {
                "steam" => "history.playStartedSteam",
                "wine" => "history.playStartedWine",
                _ => "history.playStarted",
            };
            let name = opt_str(&entry, "game_name").unwrap_or_else(|| format!("App {}", str_of(&entry, "app_id")));
            let text = t!(app, key, name = name);
            show_history_banner(app, text, 5000, "success");
        }
        Err(error) => play_failed(app, &error),
    });
}

fn play_game(app: &mut App, entry: Value, choose: bool) {
    let dir = str_of(&entry, "download_dir");
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let known = if choose { None } else { tasks.invoke::<Value>("get_launch_exe", json!({ "dir": dir })).await.ok().and_then(|exe| exe.as_str().map(str::to_string)).filter(|exe| !exe.is_empty()) };
        if let Some(exe) = known {
            return tasks.post(move |app| launch(app, entry, exe, choose));
        }
        let detected = tasks.invoke::<Value>("detect_executables", json!({ "downloadDir": dir })).await;
        tasks.post(move |app| {
            let candidates = match detected {
                Ok(detected) => detected["executables"].as_array().cloned().unwrap_or_default(),
                Err(error) => return play_failed(app, &error),
            };
            if candidates.is_empty() {
                let text = t!(app, if entry["status"] == "partial" { "history.playNoExePartial" } else { "history.playNoExe" });
                return show_history_banner(app, text, 9000, "error");
            }
            if candidates.len() == 1 && !choose {
                let exe = str_of(&candidates[0], "path");
                launch(app, entry, exe, choose);
            } else {
                choose_launch_exe(app, entry, candidates, dir);
            }
        });
    });
}

fn start_history_redownload(app: &mut App, app_id: String, depot_ids: Vec<String>, target: Option<(Option<String>, Option<Value>)>) {
    let (dir, manifests) = target.clone().unwrap_or((None, None));
    app.state.update_dir = dir;
    app.state.update_app_id = target.as_ref().map(|_| app_id.clone());
    app.state.repair_manifests = manifests.clone();
    app.state.latest_on_select = target.is_some() && manifests.is_none();
    close_history(app);
    app.state.emu_standalone = false;
    emulator::set_emu_edit_mode(app, false);
    app.state.set_parsed(None);
    app.state.selected_depots.clear();
    app.state.job_id = None;
    app.state.depot_manifests.clear();
    app.state.search_repos.clear();
    app.state.selected_repo = None;
    app.state.search_app_id = None;
    app.state.search_sha = None;
    app.state.search_key_vdf_keys = None;
    progress::cleanup_progress_listener(app);
    for id in ["game-info-banner", "search-results", "search-next-row", "search-error", "search-game-banner", "manifest-loading"] {
        app.ui.hide(id);
    }
    steps::reset_upload(app);
    app.state.auto_redownload_pending = true;
    app.state.auto_select_all_on_step2 = true;
    app.state.auto_select_depot_ids = (!depot_ids.is_empty()).then_some(depot_ids);
    steps::switch_tab(app, "search");
    app.ui.set_value("search-appid-input", app_id);
    steps::go_to_step(app, 1);
    search::perform_search(app);
}

fn entry_kind(entry: &Value) -> &'static str {
    let resumable = !entry["resume_payload"].is_null();
    match entry["status"].as_str() {
        Some("complete") => "complete",
        Some("partial") => "partial",
        Some("cancelled_resumable") if resumable => "resumable",
        Some("cancelled" | "cancelled_resumable") => "cancelled",
        _ => "failed",
    }
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "complete" => "history.statusComplete",
        "partial" => "history.statusPartial",
        "resumable" => "history.statusResumable",
        "cancelled" => "history.statusCancelled",
        _ => "history.statusFailed",
    }
}

fn entry_time(entry: &Value) -> i64 {
    let stamp = opt_str(entry, "completed_at").or_else(|| opt_str(entry, "started_at")).unwrap_or_default();
    parse_date(&stamp).map(|date| date.timestamp_millis()).unwrap_or(0)
}

fn visible_history(app: &App) -> Vec<Value> {
    let view = &app.state.history;
    let query = view.query.trim().to_lowercase();
    let mut list: Vec<Value> = view
        .cached
        .iter()
        .filter(|entry| view.status == "all" || entry_kind(entry) == view.status)
        .filter(|entry| query.is_empty() || str_of(entry, "game_name").to_lowercase().contains(&query) || str_of(entry, "app_id").contains(&query))
        .cloned()
        .collect();
    let name = |e: &Value| opt_str(e, "game_name").unwrap_or_else(|| format!("App {}", str_of(e, "app_id"))).to_lowercase();
    match view.sort.as_str() {
        "oldest" => list.sort_by_key(entry_time),
        "name" => list.sort_by_key(name),
        "size" => list.sort_by_key(|e| std::cmp::Reverse(e["size_bytes"].as_u64().unwrap_or(0))),
        _ => list.sort_by_key(|e| std::cmp::Reverse(entry_time(e))),
    }
    list
}

fn kind_counts(app: &App) -> Vec<(&'static str, usize)> {
    KINDS.into_iter().map(|kind| (kind, app.state.history.cached.iter().filter(|e| entry_kind(e) == kind).count())).filter(|(_, count)| *count > 0).collect()
}

fn sync_history_sort(app: &mut App) {
    let sort = app.state.history.sort.clone();
    for value in SORTS {
        app.ui.set_class(&format!("history-sort-{value}"), "is-active", value == sort);
    }
    let label = t!(app, &format!("history.sort{}", super::util::capitalize(&sort)));
    app.ui.set_text("history-sort-label", label);
}

fn choose_history_sort(app: &mut App, value: &str) {
    app.state.history.sort = value.to_string();
    save_history_view(app);
    sync_history_sort(app);
    picker::set_open(app, SORT_PICKER, false, None);
    app.ui.request_focus("history-sort-button");
}

fn sync_history_toolbar(app: &mut App) {
    let empty = app.state.history.cached.is_empty();
    app.ui.set_hidden("history-toolbar", empty);
    let query = app.state.history.query.clone();
    app.ui.set_value("history-search", query);
    let counts = kind_counts(app);
    if app.state.history.status != "all" && !counts.iter().any(|(kind, _)| *kind == app.state.history.status) {
        app.state.history.status = "all".to_string();
    }
    sync_history_sort(app);
}

pub fn init_history_toolbar(app: &mut App) {
    load_history_view(app);
    sync_history_sort(app);
}

fn update_check_key(entries: &[Value]) -> String {
    let mut ids: Vec<String> = entries.iter().filter(|e| e["status"] == "complete" && opt_str(e, "download_dir").is_some()).map(|e| str_of(e, "id")).collect();
    ids.sort();
    ids.join(",")
}

fn refresh_update_checks(app: &mut App) {
    let key = update_check_key(&app.state.history.cached);
    if key.is_empty() {
        return;
    }
    let fresh = app.state.history.checks_key == key && app.state.history.checks_at.is_some_and(|at| at.elapsed().as_secs() < UPDATE_CHECK_TTL_SECS);
    if fresh || app.state.history.checks_pending.as_deref() == Some(key.as_str()) {
        return;
    }
    app.state.history.checks_pending = Some(key.clone());
    app.tasks.call("check_game_updates", Value::Null, move |app, list: Result<Vec<Value>, String>| {
        app.state.history.checks_key = key;
        app.state.history.checks_at = Some(Instant::now());
        app.state.history.checks_pending = None;
        if let Ok(list) = list {
            let available = list.iter().filter(|c| c["update_available"] == true).count();
            app.emit_event("updates_found", json!({ "count": telemetry::count_bucket(available) }));
            app.state.history.checks = list.into_iter().map(|c| (str_of(&c, "entry_id"), c)).collect();
        }
    });
}

fn refresh_covers(app: &mut App) {
    let mut ids: Vec<String> = app.state.history.cached.iter().map(|e| str_of(e, "app_id")).filter(|id| !app.state.history.covers.contains_key(id)).collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return;
    }
    for id in &ids {
        app.state.history.covers.insert(id.clone(), None);
    }
    app.tasks.call("get_covers", json!({ "appIds": ids.clone() }), move |app, found: Result<Value, String>| match found {
        Ok(found) => {
            for (id, url) in found.as_object().into_iter().flatten() {
                app.state.history.covers.insert(id.clone(), url.as_str().map(str::to_string));
            }
        }
        Err(_) => {
            for id in &ids {
                app.state.history.covers.remove(id);
            }
        }
    });
}

fn render_history(app: &mut App, entries: Vec<Value>) {
    app.state.history.cached = entries;
    sync_history_toolbar(app);
    if app.state.history.cached.is_empty() {
        app.ui.set_style("btn-history-clear", "display", "none");
    } else {
        app.ui.clear_style("btn-history-clear", "display");
    }
    refresh_update_checks(app);
    refresh_covers(app);
}

fn resume_entry(app: &mut App, entry: Value, mode: &'static str) {
    close_history(app);
    app.state.emu_standalone = false;
    emulator::set_emu_edit_mode(app, false);
    progress::cleanup_progress_listener(app);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let settings = tasks.invoke::<Value>("get_settings", Value::Null).await;
        let payload = entry["resume_payload"].clone();
        let prepared = entry.clone();
        let config = tasks
            .ask(move |app| {
                let settings = settings.ok()?;
                let engine = if settings["use_native_downloader"].as_bool() != Some(false) { "native" } else { "ddm" };
                let depots = payload["selectedDepots"].as_array().cloned().unwrap_or_default();
                let mut config = payload.clone();
                if opt_str(&config, "manifestHubApiKey").is_none() {
                    config["manifestHubApiKey"] = json!(app.store.get(select::MH_APIKEY_STORAGE_KEY).filter(|key| !key.is_empty()));
                }
                config["resumeMode"] = json!(mode);
                app.state.current_engine = Some(engine.to_string());
                app.state.dl_nonce = Some(telemetry::telemetry_nonce());
                app.state.dl_started_at = Some(Instant::now());
                app.state.dl_stage = Some("starting".to_string());
                app.state.dl_depot_count = Some(depots.len());
                app.state.dl_source_count = settings["depot_sources"].as_array().map(Vec::len);
                app.state.dl_had_mh_key = !config["manifestHubApiKey"].is_null();
                app.state.dl_context = Some(json!({ "mode": "resume", "selection": "resume", "resume_mode": if mode == "fast" { "fast" } else { "full" } }));
                let props = telemetry::merged([
                    json!({ "job": app.state.dl_nonce, "depot_count": depots.len(), "engine": engine, "source_count": app.state.dl_source_count, "had_mh_key": app.state.dl_had_mh_key, "resumed": true }),
                    app.state.dl_context.clone().unwrap_or(Value::Null),
                ]);
                app.emit_event("download_started", props);
                app.state.repair_running = false;
                app.ui.hide("btn-report-failure");
                app.state.set_parsed(Some(ParsedData { main_app_id: str_of(&prepared, "app_id"), ..Default::default() }));
                app.state.game_name = opt_str(&prepared, "game_name");
                app.state.header_image = opt_str(&prepared, "header_image");
                steps::go_to_step(app, 3);
                let shown: Vec<Value> = depots.iter().map(|d| json!({ "depotId": str_of(d, "depotId"), "manifestId": str_of(d, "manifestId"), "sizeBytes": null })).collect();
                progress::init_progress_ui(app, &shown);
                progress::connect_progress_listener(app);
                Some(config)
            })
            .await
            .flatten();
        let result = match config {
            Some(config) => tasks.invoke::<Value>("start_download", json!({ "config": config })).await,
            None => Err("settings unavailable".to_string()),
        };
        match result {
            Ok(result) => {
                tasks.invoke_raw("remove_history_entry", json!({ "entryId": entry["id"] })).await.ok();
                tasks.post(move |app| {
                    if app.state.job_id.is_none() {
                        app.state.job_id = opt_str(&result, "jobId");
                    }
                    app.state.download_dir = opt_str(&result, "downloadDir").or_else(|| opt_str(&entry, "download_dir"));
                });
            }
            Err(error) => tasks.post(move |app| {
                let text = t!(app, "history.resumeError", message = app.i18n.localize_error(&error));
                ui::show_toast(app, text, "error", 8000);
                open_history(app);
            }),
        }
    });
}

fn repair_entry(app: &mut App, entry: Value) {
    let Some(dir) = opt_str(&entry, "download_dir") else { return };
    app.tasks.call("get_installed_depots", json!({ "dir": dir.clone() }), move |app, installed: Result<Vec<Value>, String>| {
        let installed = installed.unwrap_or_default();
        let manifests: serde_json::Map<String, Value> = installed.iter().map(|d| (str_of(d, "depot_id"), json!(str_of(d, "manifest_id")))).collect();
        let depot_ids: Vec<String> = if installed.is_empty() { list_of(&entry, "depot_ids") } else { manifests.keys().cloned().collect() };
        app.emit_event("history_action", json!({ "action": "repair" }));
        start_history_redownload(app, str_of(&entry, "app_id"), depot_ids, Some((Some(dir), Some(Value::Object(manifests)))));
    });
}

fn list_of(entry: &Value, key: &str) -> Vec<String> {
    entry[key].as_array().map(|list| list.iter().map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())).collect()).unwrap_or_default()
}

fn after_change(app: &mut App, result: Result<Value, String>, event: Value) {
    match result {
        Ok(_) => app.emit_event("history_action", event),
        Err(error) => {
            let text = app.i18n.localize_error(&error);
            ui::show_toast(app, text, "error", 8000);
        }
    }
    load_history(app);
}

fn show_history_clear_confirm(app: &mut App) {
    let resumable = app.state.history.cached.iter().filter(|e| entry_kind(e) == "resumable").count();
    app.ui.set_hidden("history-clear-resumable-warning", resumable == 0);
    app.ui.set_checked("history-clear-delete-files", false);
    if resumable > 0 {
        let warning = t!(app, "modals.historyClear.resumableWarning", count = resumable);
        app.ui.set_html("history-clear-resumable-text", warning);
    }
    let body = t!(app, if resumable > 0 { "modals.historyClear.bodyShort" } else { "modals.historyClear.body" });
    app.ui.set_html("history-clear-body", body);
    app.ui.show("history-clear-modal");
}

fn confirm_history_clear(app: &mut App) {
    let delete = app.ui.checked("history-clear-delete-files");
    app.ui.hide("history-clear-modal");
    app.tasks.call("clear_history", json!({ "deleteResumableFiles": delete }), move |app, result: Result<Value, String>| after_change(app, result, json!({ "action": "clear", "delete_files": delete })));
}

fn show_history_remove_confirm(app: &mut App, id: String, resumable: bool) {
    app.state.history.remove_id = Some(id);
    app.state.history.remove_resumable = resumable;
    let suffix = if resumable { "Resumable" } else { "" };
    let (title, body, yes) = (t!(app, &format!("modals.historyRemove.title{suffix}")), t!(app, &format!("modals.historyRemove.body{suffix}")), t!(app, &format!("modals.historyRemove.yes{suffix}")));
    app.ui.set_html("history-remove-modal-title", title);
    app.ui.set_html("history-remove-body", body);
    app.ui.set_text("btn-history-remove-yes", yes);
    app.ui.show("history-remove-modal");
}

fn confirm_history_remove(app: &mut App) {
    let delete = std::mem::take(&mut app.state.history.remove_resumable);
    app.ui.hide("history-remove-modal");
    let Some(id) = app.state.history.remove_id.take() else { return };
    app.tasks.call("remove_history_entry", json!({ "entryId": id, "deleteFiles": delete }), move |app, result: Result<Value, String>| after_change(app, result, json!({ "action": "remove", "delete_files": delete })));
}

pub fn show_history_banner(app: &mut App, text: String, duration_ms: u64, kind: &str) {
    app.ui.set_text("history-banner", text);
    app.ui.show("history-banner");
    app.ui.set_class("history-banner", "history-banner--success", kind == "success");
    app.state.history_banner_timer += 1;
    let token = app.state.history_banner_timer;
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(duration_ms).await;
        tasks.post(move |app| {
            if app.state.history_banner_timer == token {
                app.ui.hide("history-banner");
            }
        });
    });
}

pub fn hide_history_banner(app: &mut App) {
    app.ui.hide("history-banner");
    app.state.history_banner_timer += 1;
}

pub fn show_folder_missing(app: &mut App) {
    if app.ui.hidden("history-modal") {
        open_history(app);
    }
    let text = t!(app, "modals.folderMissing.body");
    show_history_banner(app, text, 6000, "error");
}

fn entry_by_id(app: &App, id: &str) -> Option<Value> {
    app.state.history.cached.iter().find(|e| e["id"] == id).cloned()
}

fn finish_launch(app: &mut App, start: bool) {
    app.ui.hide("launch-modal");
    let Some(entry) = app.state.history.launch_entry.take() else { return };
    if start {
        let exe = app.state.history.launch_chosen.clone();
        launch(app, entry, exe, true);
    }
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-history" => open_history(app),
        "btn-history-close" | "history-modal>modal__backdrop" => close_history(app),
        "btn-history-clear" => show_history_clear_confirm(app),
        "btn-history-clear-yes" => confirm_history_clear(app),
        "btn-history-clear-no" | "history-clear-modal>modal__backdrop" => app.ui.hide("history-clear-modal"),
        "btn-history-remove-yes" => confirm_history_remove(app),
        "btn-history-remove-no" | "history-remove-modal>modal__backdrop" => app.ui.hide("history-remove-modal"),
        "btn-launch-start" => finish_launch(app, true),
        "btn-launch-cancel" | "launch-modal>modal__backdrop" => finish_launch(app, false),
        "history-sort-button" => {
            let open = !picker::is_open(app, SORT_PICKER);
            let active = SORTS.iter().position(|s| *s == app.state.history.sort);
            picker::set_open(app, SORT_PICKER, open, active);
        }
        _ => {
            let Some((action, rest)) = id.split_once(':') else { return false };
            match action {
                "history-sort" => choose_history_sort(app, rest),
                "history-chip" => {
                    app.state.history.status = rest.to_string();
                    save_history_view(app);
                }
                "history-resume" | "history-play" | "history-play-choose" | "history-redownload" | "history-update" | "history-repair" | "history-folder" | "history-edit" | "history-remove" => {
                    let Some(entry) = entry_by_id(app, rest) else { return true };
                    match action {
                        "history-resume" => download_control::ask_resume_mode(app, move |app, mode| {
                            if let Some(mode) = mode {
                                resume_entry(app, entry, mode);
                            }
                        }),
                        "history-play" => play_game(app, entry, false),
                        "history-play-choose" => play_game(app, entry, true),
                        "history-redownload" | "history-update" => {
                            let update = action == "history-update";
                            app.emit_event("history_action", json!({ "action": if update { "update" } else { "redownload" } }));
                            let target = update.then(|| (opt_str(&entry, "download_dir"), None));
                            start_history_redownload(app, str_of(&entry, "app_id"), list_of(&entry, "depot_ids"), target);
                        }
                        "history-repair" => repair_entry(app, entry),
                        "history-folder" => app.tasks.call("open_folder", json!({ "path": entry["download_dir"] }), |app, result: Result<Value, String>| match result {
                            Ok(_) => app.emit_event("history_action", json!({ "action": "open_folder" })),
                            Err(_) => show_folder_missing(app),
                        }),
                        "history-edit" => {
                            app.emit_event("history_action", json!({ "action": "edit_emu" }));
                            emulator::open_emu_edit_from_history(app, entry);
                        }
                        _ => {
                            let resumable = entry_kind(&entry) == "resumable";
                            show_history_remove_confirm(app, rest.to_string(), resumable);
                        }
                    }
                }
                _ => return false,
            }
        }
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    if id == "history-search" {
        app.state.history.query = app.ui.value(id).to_string();
        return true;
    }
    false
}

pub fn changed(_app: &mut App, id: &str) -> bool {
    id == "history-clear-delete-files"
}

fn cover_markup(app: &App, entry: &Value) -> String {
    let cover = app.state.history.covers.get(&str_of(entry, "app_id")).cloned().flatten();
    let header = opt_str(entry, "header_image");
    let source = [cover, header].into_iter().flatten().find(|url| app.images.status(url) != Status::Failed);
    match source {
        Some(url) => format!(r#"<img class="history-entry__image" src="{}">"#, escape_html(&url)),
        None => COVER_PLACEHOLDER.to_string(),
    }
}

fn entry_markup(app: &App, entry: &Value) -> String {
    let id = escape_html(&str_of(entry, "id"));
    let app_id = escape_html(&str_of(entry, "app_id"));
    let stamp = opt_str(entry, "completed_at").or_else(|| opt_str(entry, "started_at")).unwrap_or_default();
    let kind = entry_kind(entry);
    let has_dir = opt_str(entry, "download_dir").is_some();
    let can_resume = kind == "resumable" && app.state.history.use_native;
    let can_update = entry["status"] == "complete" && has_dir;
    let can_play = (entry["status"] == "complete" || entry["status"] == "partial") && has_dir;
    let has_update = can_update && app.state.history.checks.get(&str_of(entry, "id")).is_some_and(|check| check["update_available"] == true);
    let name = opt_str(entry, "game_name").map(|name| escape_html(&name)).unwrap_or_else(|| format!("App {app_id}"));
    let size = entry["size_bytes"].as_f64().and_then(format_bytes).map(|size| format!(r#"<span class="history-entry__size">{}</span>"#, escape_html(&size))).unwrap_or_default();
    let tip = |key: &str| escape_html(app.i18n.t(key));
    let button = |class: &str, action: &str, key: &str, glyph: &str, disabled: bool, extra: &str| {
        format!(r#"<button class="btn btn--small {class}" data-click="{action}:{id}" title="{}"{}{extra}>{}</button>"#, tip(key), if disabled { " disabled" } else { "" }, icon(glyph))
    };
    let update_tip = if !can_update { "history.updateUnavailable" } else if has_update { "history.updateAvailableTooltip" } else { "history.updateTooltip" };
    let cancelled = entry["status"] == "cancelled";
    let mut actions = String::new();
    if can_resume {
        actions.push_str(&button("btn--primary history-action-resume", "history-resume", "history.resumeTooltip", ICON_PLAY, false, ""));
    }
    if can_play {
        actions.push_str(&button("btn--outline history-action-play", "history-play", "history.playTooltip", ICON_GAMEPAD, false, &format!(r#" data-context="history-play-choose:{id}""#)));
    }
    actions.push_str(&button("btn--outline history-action-redownload", "history-redownload", "history.redownloadTooltip", ICON_REFRESH, false, ""));
    actions.push_str(&button(if has_update { "btn--primary history-action-update" } else { "btn--outline history-action-update" }, "history-update", update_tip, ICON_UPDATE, !can_update, ""));
    actions.push_str(&button("btn--outline history-action-repair", "history-repair", if can_update { "history.repairTooltip" } else { "history.repairUnavailable" }, ICON_SHIELD_CHECK, !can_update, ""));
    actions.push_str(&button("btn--outline history-action-folder", "history-folder", "history.openFolderTooltip", ICON_FOLDER_OPEN, cancelled, ""));
    actions.push_str(&button("btn--outline history-action-edit-emu", "history-edit", "emulator.history.editTooltip", ICON_SETTINGS, cancelled || !has_dir, ""));
    actions.push_str(&button("btn--outline history-action-remove", "history-remove", "history.removeTooltip", ICON_TRASH, false, ""));
    format!(
        r#"<div class="history-entry" data-key="{id}">{cover}<div class="history-entry__info"><div class="history-entry__name">{name}</div><div class="history-entry__meta"><span class="history-entry__appid">App {app_id}</span><span class="history-entry__date">{date}</span>{size}</div><div class="history-entry__depots">{depots}</div><div class="history-entry__status"><span class="history-entry__badge history-entry__badge--{kind}">{status}</span>{update}</div></div><div class="history-entry__actions">{actions}</div></div>"#,
        cover = cover_markup(app, entry),
        date = escape_html(&date_time_medium_short(app.i18n.code(), &stamp)),
        depots = t!(app, "history.depotsDownloaded", done = str_of(entry, "depots_downloaded"), total = str_of(entry, "depot_count")),
        status = escape_html(app.i18n.t(kind_label(kind))),
        update = if has_update { format!(r#"<span class="history-entry__badge history-entry__badge--update">{}</span>"#, tip("history.updateAvailable")) } else { String::new() },
    )
}

fn fill_launch_list(app: &mut App, dom: &mut Dom<'_>) {
    let candidates = app.state.history.launch.clone();
    let dir = app.state.history.launch_dir.clone();
    let mut picked = None;
    for candidate in &candidates {
        let path = str_of(candidate, "path");
        let checked = path == app.state.history.launch_chosen;
        let markup = format!(
            r#"<span class="launch-option__name">{}{}</span><span class="launch-option__path" title="{}">{}</span><span class="launch-option__size">{}</span>"#,
            escape_html(&str_of(candidate, "name")),
            exe_platform_badge(app, candidate, &candidates),
            escape_html(&path),
            escape_html(&relative_to_dir(&path, &dir)),
            escape_html(&candidate["size"].as_f64().and_then(format_bytes).unwrap_or_default()),
        );
        let clicked = dom
            .button(el("label.launch-option").state(HAS_CHECKED, checked).key(&path), |dom| {
                dom.empty(el("input[type=radio]").state(CHECKED, checked));
                dom.markup(&markup);
            })
            .clicked();
        if clicked {
            picked = Some(path);
        }
    }
    if let Some(path) = picked {
        app.state.history.launch_chosen = path;
    }
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    let markup: String = match id {
        "history-list" => match app.state.history.loading {
            Some(true) => format!(r#"<div class="history-loading"><div class="spinner"></div><span>{}</span></div>"#, escape_html(app.i18n.t("history.loading"))),
            Some(false) => format!(r#"<div class="history-empty">{}</div>"#, escape_html(app.i18n.t("history.loadFailed"))),
            None if app.state.history.cached.is_empty() => format!(r#"<div class="history-empty">{}</div>"#, escape_html(app.i18n.t("history.empty"))),
            None => {
                let entries = visible_history(app);
                if entries.is_empty() {
                    format!(r#"<div class="history-empty">{}</div>"#, escape_html(app.i18n.t("history.noMatch")))
                } else {
                    entries.iter().map(|entry| entry_markup(app, entry)).collect()
                }
            }
        },
        "history-filters" => {
            let view = app.state.history.status.clone();
            let mut chips = vec![("all", app.i18n.t("history.filterAll").to_string(), app.state.history.cached.len())];
            chips.extend(kind_counts(app).into_iter().map(|(kind, count)| (kind, app.i18n.t(kind_label(kind)).to_string(), count)));
            chips
                .iter()
                .map(|(kind, label, count)| {
                    format!(r#"<button class="history-chip history-chip--{kind}{}" data-click="history-chip:{kind}">{}<span class="history-chip__count">{count}</span></button>"#, if *kind == view { " is-active" } else { "" }, escape_html(label))
                })
                .collect()
        }
        "launch-list" => {
            fill_launch_list(app, dom);
            return true;
        }
        _ => return false,
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    if app.ui.hidden("history-modal") {
        return;
    }
    let active = SORTS.iter().position(|s| *s == app.state.history.sort);
    if let Outcome::Choose(index) = picker::frame(app, ctx, SORT_PICKER, SORTS.len(), active) {
        choose_history_sort(app, SORTS[index]);
    }
    let focused = picker::focused(app, SORT_PICKER);
    for (index, value) in SORTS.iter().enumerate() {
        app.ui.set_class(&format!("history-sort-{value}"), "is-focused", focused == Some(index));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_entries_into_kinds_and_shortens_paths() {
        assert_eq!(entry_kind(&json!({ "status": "complete" })), "complete");
        assert_eq!(entry_kind(&json!({ "status": "cancelled_resumable", "resume_payload": {} })), "resumable");
        assert_eq!(entry_kind(&json!({ "status": "cancelled_resumable" })), "cancelled");
        assert_eq!(entry_kind(&json!({ "status": "error" })), "failed");
        assert_eq!(relative_to_dir("C:\\Games\\X\\bin\\x.exe", "c:/games/x/"), "bin/x.exe");
        assert_eq!(relative_to_dir("/other/x", "/games"), "/other/x");
        assert_eq!(update_check_key(&[json!({ "id": "b", "status": "complete", "download_dir": "/x" }), json!({ "id": "a", "status": "complete", "download_dir": "/y" })]), "a,b");
    }
}
