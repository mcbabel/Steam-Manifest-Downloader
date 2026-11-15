use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::{json, Value};

use super::emulator_extras as extras;
use super::emulator_settings as fields;
use super::util::{escape_html, format_bytes, opt_str, str_of};
use super::{history, progress, select, steps, telemetry, ui};
use crate::app::state::ParsedData;
use crate::app::tasks::Tasks;
use crate::app::App;
use crate::dom::{el, Dom};
use crate::platform::FileFilter;
use crate::style::selector::{CHECKED, HAS_CHECKED};
use crate::t;

pub const SECTIONS: [&str; 9] = ["files", "gamedata", "controller", "emulator", "saves", "profile", "network", "mods", "extras"];
const DOWNLOAD_CONFIRM: &str = "EMU_DOWNLOAD_CONFIRM_REQUIRED:";
const RELEASES: &str = "https://github.com/Detanup01/gbe_fork/releases/";

#[derive(Default)]
pub struct EmuState {
    pub scan: Vec<Value>,
    pub selected_files: BTreeSet<String>,
    pub selection_explicit: bool,
    pub edit_targets: Vec<Value>,
    pub patched_paths: HashSet<String>,
    pub settings_prefill_path: Option<String>,
    pub busy: bool,
    pub apply_complete: bool,
    pub allow_download: bool,
    pub prefer_local: bool,
    pub pending_download: Option<(Vec<Value>, String)>,
    pub lobby_retry: bool,
    pub download_listening: bool,
    pub download_tag: Option<String>,
    pub bypass_initial_state: bool,
    pub release_info: Option<Value>,
    pub drm_targets: Vec<Value>,
    pub drm_shown: Vec<Value>,
    pub drm_success: HashSet<usize>,
    pub drm_backups: Vec<Value>,
    pub dlc_merge_plan: Option<Value>,
    pub controller_info: Option<Value>,
    pub controller_checking: bool,
    pub controller_file: Option<String>,
    pub controller_variant_set: bool,
    pub patch_info: Option<Value>,
    pub settings_group: Option<&'static str>,
    pub reported_sections: HashSet<String>,
    pub search_timer: u64,
    pub copy_timer: u64,
    pub save_folder: Option<Value>,
    pub save_folders: Vec<Value>,
    pub achievements: Vec<Value>,
    pub overlay_assets: Value,
    pub system_fonts: Option<Vec<Value>>,
    pub fonts_loading: bool,
    pub mods: Vec<Value>,
    pub cold_dir: Option<String>,
    pub armed: HashMap<&'static str, u64>,
    pub pictures: HashMap<String, String>,
}

fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false)) && value != "" && value != 0
}

fn paths_of(targets: &[Value]) -> Vec<String> {
    targets.iter().map(|t| str_of(t, "path")).filter(|p| !p.is_empty()).collect()
}

pub fn check_emulator_support(app: &mut App) {
    check_emulator_support_then(app, |_| {});
}

pub fn check_emulator_support_then(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    app.state.emu.selection_explicit = false;
    let Some(dir) = app.state.download_dir.clone() else {
        app.state.emulator_available = false;
        return then(app);
    };
    app.tasks.call("emu_scan_game_dir", json!({ "gameDir": dir }), move |app, scanned: Result<Vec<Value>, String>| {
        app.state.emu.scan = scanned.unwrap_or_default();
        app.state.emu.selected_files.clear();
        app.state.emulator_available = !app.state.emu.scan.is_empty();
        update_next_button_text(app);
        then(app);
    });
}

pub fn update_next_button_text(app: &mut App) {
    let has_next = app.state.shortcut_supported || app.state.steam_library_supported || app.state.emulator_available;
    let label = t!(app, if has_next { "common.next" } else { "emulator.goToHome" });
    app.ui.set_text("btn-next-step", label);
}

pub fn go_to_emulator_step(app: &mut App, prefill: bool) {
    app.state.emu.edit_targets.clear();
    app.state.emu.patched_paths.clear();
    app.state.emu.settings_prefill_path = None;
    app.state.emu.busy = false;
    app.state.emu.bypass_initial_state = false;
    hide_emu_already_patched_notice(app);
    apply_emu_settings_hint(app);
    if app.state.emu_edit_mode {
        set_emu_edit_mode(app, false);
    }
    app.state.emu.apply_complete = false;
    let apply = t!(app, "emulator.apply");
    app.ui.set_text("btn-emu-apply", apply);
    apply_emu_start_over_visibility(app);
    steps::go_to_step(app, 5);
    let enter = move |app: &mut App| {
        render_emu_file_list(app);
        update_emu_action_buttons(app);
        extras::reset_emu_extras_view(app);
        extras::refresh_emu_method(app);
        if prefill {
            let last = fields::load_last(app).unwrap_or_else(|| json!({}));
            fields::populate(app, &last);
        }
        apply_bypass_availability(app);
        scan_for_dlc_merge(app);
        scan_for_drm(app);
        load_emu_controller_info(app);
        refresh_emu_drm_restore(app);
        refresh_emu_denuvo(app);
        load_emu_release_info(app);
    };
    if app.state.emu.scan.is_empty() {
        check_emulator_support_then(app, enter);
    } else {
        enter(app);
    }
}

pub fn start_standalone(app: &mut App, game_dir: String, app_id: String, scanned: Vec<Value>) {
    let unpatched: Vec<String> = scanned.iter().filter(|f| !truthy(&f["is_patched"])).map(|f| str_of(f, "path")).collect();
    app.state.emu.selection_explicit = true;
    app.state.emu_standalone = true;
    app.state.download_dir = Some(game_dir);
    app.state.set_parsed(Some(ParsedData { main_app_id: app_id, ..Default::default() }));
    app.state.emu.scan = scanned.clone();
    app.state.emu.selected_files = unpatched.iter().cloned().collect();
    app.state.emulator_available = true;
    go_to_emulator_step(app, unpatched.len() == scanned.len());
    refresh_emu_patched_state(app, scanned);
    hydrate_emu_patched_context(app);
}

fn scan_for_dlc_merge(app: &mut App) {
    app.ui.hide("emu-dlc-merge-section");
    app.ui.hide("emu-dlc-merge-status");
    let Some(dir) = app.state.download_dir.clone().filter(|_| !app.state.emu_standalone) else { return };
    let app_id = current_app_id_for_emu(app);
    app.tasks.call("emu_scan_for_dlc_merge", json!({ "gameDir": dir, "appId": app_id }), |app, plan: Result<Value, String>| {
        let Ok(plan) = plan else { return };
        if plan["toMerge"].as_array().is_none_or(|list| list.is_empty()) {
            return;
        }
        let hint = render_merge_plan_hint(app, &plan);
        app.state.emu.dlc_merge_plan = Some(plan);
        app.ui.show("emu-dlc-merge-section");
        app.ui.set_html("emu-dlc-merge-hint", hint);
        app.ui.set_disabled("btn-emu-merge-dlcs", false);
    });
}

fn role_label(app: &App, role: &str) -> String {
    let key = format!("depots.role_{role}");
    if app.i18n.has(&key) {
        app.i18n.t(&key).to_string()
    } else {
        role.to_string()
    }
}

fn render_merge_plan_hint(app: &App, plan: &Value) -> String {
    let main_id = str_of(plan, "mainDepotId");
    let main = match opt_str(plan, "mainLabel") {
        Some(label) => format!("{label} ({main_id})"),
        None => main_id,
    };
    let row = |d: &Value, skipped: bool| {
        let label = opt_str(d, "label").map(|l| format!("— {}", escape_html(&l))).unwrap_or_default();
        let role = str_of(d, "role");
        let (class, text) = if skipped {
            ("skipped".to_string(), format!("{} ({})", escape_html(&role_label(app, &role)), escape_html(app.i18n.t("emulator.dlcMergeSkippedTag"))))
        } else {
            (escape_html(&role), escape_html(&role_label(app, &role)))
        };
        format!(r#"<li><strong>{}</strong> {label} <span class="depot-role-pill depot-role-pill--{class}">{text}</span></li>"#, escape_html(&str_of(d, "depotId")))
    };
    let to_merge: String = plan["toMerge"].as_array().into_iter().flatten().map(|d| row(d, false)).collect();
    let skipped: String = plan["skipped"].as_array().into_iter().flatten().map(|d| row(d, true)).collect();
    let count = plan["toMerge"].as_array().map(Vec::len).unwrap_or(0);
    let intro = t!(app, "emulator.dlcMergeHintDetail", count = count, main = escape_html(&main));
    let skipped_block =
        if skipped.is_empty() { String::new() } else { format!(r#"<p class="dd-path__hint">{}</p><ul class="emu-dlc-merge-list">{skipped}</ul>"#, app.i18n.t("emulator.dlcMergeSkippedNote")) };
    format!(r#"{intro}<ul class="emu-dlc-merge-list">{to_merge}</ul>{skipped_block}"#)
}

fn perform_dlc_merge(app: &mut App) {
    let Some(plan) = app.state.emu.dlc_merge_plan.clone() else { return };
    let busy = t!(app, "emulator.dlcMergeBusy");
    app.ui.set_disabled("btn-emu-merge-dlcs", true);
    app.ui.set_text("btn-emu-merge-dlcs", busy.clone());
    app.ui.show("emu-dlc-merge-status");
    app.ui.set_text("emu-dlc-merge-status", busy);
    let count = plan["dlcDepotDirs"].as_array().map(Vec::len).unwrap_or(0);
    let args = json!({ "mainDepotDir": plan["mainDepotDir"], "dlcDepotDirs": plan["dlcDepotDirs"] });
    app.tasks.call("emu_merge_dlc_depots", args, move |app, result: Result<Value, String>| match result {
        Ok(_) => {
            app.emit_event("dlc_merged", json!({ "ok": true, "depots": telemetry::count_bucket(count) }));
            app.state.emu.dlc_merge_plan = None;
            let done = t!(app, "emulator.dlcMergeDone", count = count);
            app.ui.set_text("emu-dlc-merge-status", done);
            let short = t!(app, "emulator.dlcMergeDoneShort");
            app.ui.set_text("btn-emu-merge-dlcs", short);
            let tasks = app.tasks.clone();
            app.tasks.spawn(async move {
                tasks.sleep(4000).await;
                tasks.post(|app| app.ui.hide("emu-dlc-merge-section"));
            });
        }
        Err(error) => {
            let text = t!(app, "emulator.dlcMergeError", message = app.i18n.localize_error(&error));
            app.ui.set_text("emu-dlc-merge-status", text);
            app.ui.set_disabled("btn-emu-merge-dlcs", false);
            let label = t!(app, "emulator.dlcMergeButton");
            app.ui.set_text("btn-emu-merge-dlcs", label);
        }
    });
}

fn apply_bypass_availability(app: &mut App) {
    let windows = app.state.emu.scan.iter().any(|t| t["platform"] == "windows");
    app.ui.set_class("emu-bypass-section", "emu-bypass-section--disabled", !windows);
    if !windows {
        app.ui.set_checked("emu-bypass-toggle", false);
    }
    app.ui.set_disabled("emu-bypass-toggle", !windows);
    let hint = t!(app, if windows { "emulator.bypassHint" } else { "emulator.bypassLinuxNote" });
    app.ui.set_html("emu-bypass-hint", hint);
}

pub fn scan_for_drm(app: &mut App) {
    let Some(dir) = app.state.download_dir.clone() else { return };
    app.tasks.call("steamless_scan", json!({ "gameDir": dir }), |app, entries: Result<Vec<Value>, String>| match entries {
        Ok(entries) => {
            app.state.emu.drm_targets = entries;
            render_drm_section(app);
        }
        Err(_) => {
            app.state.emu.drm_targets.clear();
            app.ui.hide("emu-drm-section");
        }
    });
}

fn render_drm_section(app: &mut App) {
    if app.state.emu.drm_targets.is_empty() {
        return app.ui.hide("emu-drm-section");
    }
    set_drm_done(app, false);
    app.ui.show("emu-drm-section");
    app.state.emu.drm_shown = app.state.emu.drm_targets.clone();
    app.state.emu.drm_success.clear();
    app.ui.hide("emu-drm-status-wrap");
    app.ui.set_disabled("btn-emu-drm-remove", false);
    let label = t!(app, "emulator.drmRemove");
    app.ui.set_text("btn-emu-drm-remove", label);
}

fn set_drm_done(app: &mut App, done: bool) {
    app.ui.set_class("emu-drm-section", "emu-drm-section--done", done);
    let (title, description) = (t!(app, if done { "emulator.drmDoneTitle" } else { "emulator.drmTitle" }), t!(app, if done { "emulator.drmDoneDescription" } else { "emulator.drmDescription" }));
    app.ui.set_text("emu-drm-title", title);
    app.ui.set_text("emu-drm-description", description);
}

fn set_drm_status(app: &mut App, kind: &str, text: String) {
    let classes: &[&'static str] = match kind {
        "busy" => &["emu-drm-status", "emu-drm-status--busy"],
        "success" => &["emu-drm-status", "emu-drm-status--success"],
        "error" => &["emu-drm-status", "emu-drm-status--error"],
        _ => &["emu-drm-status"],
    };
    app.ui.set_classes("emu-drm-status", classes);
    app.ui.set_text("emu-drm-status", text);
    app.ui.show("emu-drm-status-wrap");
}

fn copy_drm_log(app: &mut App) {
    let text = app.ui.text("emu-drm-status").unwrap_or("").to_string();
    app.ctx.copy_text(text);
    app.ui.add_class("btn-emu-drm-copy", "emu-drm-copy-btn--copied");
    app.ui.set_html("btn-emu-drm-copy", r#"<svg data-icon="check" data-stroke="2.5" width="14" height="14"></svg>"#);
    let copied = t!(app, "emulator.drmCopyLogCopied");
    app.ui.set_attr("btn-emu-drm-copy", "title", copied);
    app.state.emu.copy_timer += 1;
    let token = app.state.emu.copy_timer;
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(1500).await;
        tasks.post(move |app| {
            if app.state.emu.copy_timer != token {
                return;
            }
            app.ui.remove_class("btn-emu-drm-copy", "emu-drm-copy-btn--copied");
            app.ui.clear_content("btn-emu-drm-copy");
            app.ui.remove_attr("btn-emu-drm-copy", "title");
        });
    });
}

fn mono_hint(app: &App, error: &str) -> String {
    let lower = error.to_lowercase();
    let missing = ["command not found", "no such file", "cannot run", "exec format"].iter().any(|needle| lower.contains(needle));
    if missing && lower.contains("mono") {
        format!("\n\n{}", app.i18n.t("emulator.drmMonoHint"))
    } else {
        String::new()
    }
}

fn remove_drm(app: &mut App) {
    if app.state.emu.drm_targets.is_empty() {
        return;
    }
    let paths = paths_of(&app.state.emu.drm_targets);
    app.ui.set_disabled("btn-emu-drm-remove", true);
    let removing = t!(app, "emulator.drmRemoving");
    set_drm_status(app, "busy", removing);
    app.tasks.call("steamless_unpack", json!({ "targets": paths }), |app, results: Result<Vec<Value>, String>| {
        app.ui.set_disabled("btn-emu-drm-remove", false);
        match results {
            Ok(results) => {
                let success = results.iter().filter(|r| truthy(&r["success"])).count();
                let failed = results.len() - success;
                let outcome = if failed == 0 { "complete" } else if success > 0 { "partial" } else { "failed" };
                app.emit_event("steamless_used", json!({ "outcome": outcome, "targets": telemetry::count_bucket(results.len()) }));
                if failed == 0 {
                    let text = t!(app, "emulator.drmRemoveSuccess", count = success);
                    set_drm_status(app, "success", text);
                    app.state.emu.drm_success = (0..results.len()).collect();
                    app.state.emu.drm_targets.clear();
                    set_drm_done(app, true);
                } else {
                    let error = results.iter().find(|r| !truthy(&r["success"])).and_then(|r| opt_str(r, "error")).unwrap_or_else(|| "unknown error".to_string());
                    let summary = t!(app, "emulator.drmRemovePartial", success = success, failed = failed);
                    let text = telemetry::with_emu_hint(app, &format!("{summary}\n\n{error}{}", mono_hint(app, &error)), &[&error]);
                    set_drm_status(app, "error", text);
                }
            }
            Err(error) => {
                let base = t!(app, "emulator.drmRemoveError", message = app.i18n.localize_error(&error));
                let text = telemetry::with_emu_hint(app, &format!("{base}{}", mono_hint(app, &error)), &[&error]);
                set_drm_status(app, "error", text);
            }
        }
    });
}

fn extract_depot_folder(path: &str) -> String {
    let normal = path.replace('\\', "/");
    let Some(index) = normal.find("/depots/") else { return String::new() };
    let rest = &normal[index + "/depots/".len()..];
    rest.split('/').next().unwrap_or("").to_string()
}

fn pick_default_emu_depot(files: &[Value]) -> Option<String> {
    let host_linux = cfg!(target_os = "linux");
    let mut groups: Vec<(String, Vec<&Value>)> = Vec::new();
    for file in files {
        let folder = extract_depot_folder(&str_of(file, "path"));
        let key = if folder.is_empty() { "__root__".to_string() } else { folder };
        match groups.iter_mut().find(|(name, _)| *name == key) {
            Some((_, group)) => group.push(file),
            None => groups.push((key, vec![file])),
        }
    }
    let mut best: Option<(i64, String)> = None;
    for (key, group) in groups {
        let mut score = group.len() as i64;
        if group.iter().any(|f| (f["platform"] == "linux") == host_linux) {
            score += 1000;
        }
        if group.iter().any(|f| f["arch"] == "x64") {
            score += 100;
        }
        if best.as_ref().is_none_or(|(top, _)| score > *top) {
            best = Some((score, key));
        }
    }
    best.map(|(_, key)| key)
}

fn apply_emu_settings_hint(app: &mut App) {
    let key = if !app.state.emu.edit_targets.is_empty() {
        "emulator.settingsHintPatched"
    } else if app.state.emu_standalone {
        "emulator.settingsHintStandalone"
    } else {
        "emulator.settingsHint"
    };
    let text = t!(app, key);
    app.ui.set_text("emu-settings-hint", text);
}

fn update_emu_patched_notice(app: &mut App) {
    let patched = app.state.emu.edit_targets.len();
    let total = app.state.emu.scan.len();
    if app.state.emu_edit_mode || patched == 0 {
        return hide_emu_already_patched_notice(app);
    }
    let text = if patched == total { t!(app, "emulator.patchedNoticeAll") } else { t!(app, "emulator.patchedNoticeSome", patched = patched, total = total) };
    app.ui.set_text("emu-already-patched", text);
    app.ui.show("emu-already-patched");
}

fn hide_emu_already_patched_notice(app: &mut App) {
    app.ui.hide("emu-already-patched");
}

pub fn render_emu_file_list(app: &mut App) {
    let files = app.state.emu.scan.clone();
    if files.is_empty() {
        app.ui.show("emu-file-empty");
        app.state.emu.selected_files.clear();
        return update_emu_action_buttons(app);
    }
    app.ui.hide("emu-file-empty");
    if !app.state.emu.selection_explicit && app.state.emu.selected_files.is_empty() {
        let folders: HashSet<String> = files.iter().map(|f| extract_depot_folder(&str_of(f, "path"))).filter(|f| !f.is_empty()).collect();
        let default_depot = if folders.len() > 1 { pick_default_emu_depot(&files) } else { None };
        app.state.emu.selected_files = files
            .iter()
            .filter(|f| default_depot.as_ref().is_none_or(|depot| extract_depot_folder(&str_of(f, "path")) == *depot))
            .map(|f| str_of(f, "path"))
            .collect();
    }
    update_emu_action_buttons(app);
}

pub fn set_emu_busy(app: &mut App, on: bool) {
    app.state.emu.busy = on;
    update_emu_action_buttons(app);
}

pub fn update_emu_action_buttons(app: &mut App) {
    let selected = get_selected_emu_targets(app);
    let patched = app.state.emu.edit_targets.len();
    let busy = app.state.emu.busy;
    app.ui.set_hidden("btn-emu-apply", app.state.emu_edit_mode);
    app.ui.set_disabled("btn-emu-apply", busy || selected.is_empty());
    if !app.state.emu.apply_complete {
        let all_patched = !selected.is_empty() && selected.iter().all(|t| app.state.emu.patched_paths.contains(&str_of(t, "path")));
        let label = t!(app, if all_patched { "emulator.reapply" } else { "emulator.apply" });
        app.ui.set_text("btn-emu-apply", label);
    }
    app.ui.set_hidden("btn-emu-save-settings", patched == 0);
    app.ui.set_disabled("btn-emu-save-settings", busy || app.state.emu.settings_prefill_path.is_none());
    let primary = app.state.emu_edit_mode || selected.is_empty();
    app.ui.set_class("btn-emu-save-settings", "btn--primary", primary);
    app.ui.set_class("btn-emu-save-settings", "btn--outline", !primary);
    app.ui.set_hidden("btn-emu-revert", patched == 0);
    app.ui.set_disabled("btn-emu-revert", busy);
    update_emu_overview(app);
}

pub fn select_emu_tab(app: &mut App, section: &str) {
    for name in SECTIONS {
        app.ui.set_class(&format!("emu-tab-{name}"), "is-active", name == section);
        app.ui.set_class(&format!("emu-panel-{name}"), "is-active", name == section);
    }
    extras::refresh_emu_extra_tab(app, section);
}

pub fn set_emu_badge(app: &mut App, section: &str, text: &str, tone: &str) {
    let key = format!("emu-badge-{section}");
    app.ui.set_text(&key, text);
    app.ui.set_hidden(&key, text.is_empty());
    app.ui.set_class(&key, "emu-nav__badge--on", tone == "on");
    app.ui.set_class(&key, "emu-nav__badge--warn", tone == "warn");
}

pub fn update_emu_overview(app: &mut App) {
    let total = app.state.emu.scan.len();
    let selected = get_selected_emu_targets(app).len();
    let drm_visible = !app.ui.hidden("emu-drm-section");
    let drm_done = drm_visible && app.ui.has_class("emu-drm-section", "emu-drm-section--done");
    let drm_shown = drm_visible && !drm_done;
    let merge_shown = !app.ui.hidden("emu-dlc-merge-section");
    let game_data_on = app.ui.checked("emu-gamedata-toggle");
    let bypass_on = app.ui.checked("emu-bypass-toggle");
    let controller_on = emu_controller_on(app);
    let variant = selected_emu_variant(app);
    let custom = fields::custom_count(app);
    let (on, off) = (t!(app, "emulator.summary.on"), t!(app, "emulator.summary.off"));

    let files = if drm_shown { "!".to_string() } else if total > 0 { format!("{selected}/{total}") } else { String::new() };
    set_emu_badge(app, "files", &files, if drm_shown { "warn" } else { "" });
    set_emu_badge(app, "gamedata", if game_data_on { &on } else { &off }, if game_data_on { "on" } else { "" });
    set_emu_badge(app, "controller", if controller_on { &on } else { &off }, if controller_on { "on" } else { "" });
    set_emu_badge(app, "emulator", &if custom > 0 { custom.to_string() } else { String::new() }, "");
    let extras_text = if merge_shown { "!" } else if bypass_on { on.as_str() } else { "" };
    set_emu_badge(app, "extras", extras_text, if merge_shown { "warn" } else if bypass_on { "on" } else { "" });

    let mut chips = String::new();
    let mut chip = |text: String, tone: &str| {
        let class = if tone.is_empty() { String::new() } else { format!(" emu-summary__chip--{tone}") };
        chips.push_str(&format!(r#"<span class="emu-summary__chip{class}">{text}</span>"#));
    };
    if let Some(name) = &app.state.game_name {
        chip(format!("<strong>{}</strong>", escape_html(name)), "");
    }
    if total > 0 {
        chip(escape_html(&t!(app, "emulator.summary.files", selected = selected, total = total)), "");
    }
    chip(escape_html(app.i18n.t(if variant == "experimental" { "emulator.variantExperimental" } else { "emulator.variantRegular" })), "");
    chip(escape_html(app.i18n.t("emulator.tabs.gameData")), if game_data_on { "on" } else { "off" });
    chip(escape_html(app.i18n.t("emulator.summary.controller")), if controller_on { "on" } else { "off" });
    if bypass_on {
        chip(escape_html(app.i18n.t("emulator.bypassLabel")), "on");
    }
    if custom > 0 {
        chip(escape_html(&t!(app, "emulator.summary.custom", count = custom)), "");
    }
    if drm_shown {
        chip(escape_html(app.i18n.t("emulator.drmTitle")), "warn");
    }
    if drm_done {
        chip(escape_html(app.i18n.t("emulator.drmDoneTitle")), "on");
    }
    if merge_shown {
        chip(escape_html(app.i18n.t("emulator.dlcMergeLabel")), "warn");
    }
    app.ui.set_html("emu-summary", chips);
}

fn report_emu_section(app: &mut App, section: &str) {
    if app.state.emu.reported_sections.insert(section.to_string()) {
        app.emit_event("emu_section_viewed", json!({ "section": section }));
    }
}

pub fn refresh_emu_patched_state(app: &mut App, scanned: Vec<Value>) {
    app.state.emu.edit_targets = scanned.iter().filter(|f| truthy(&f["is_patched"])).cloned().collect();
    app.state.emu.patched_paths = app.state.emu.edit_targets.iter().map(|f| str_of(f, "path")).collect();
    app.state.emu.scan = scanned;
}

pub fn hydrate_emu_patched_context(app: &mut App) {
    hydrate_then(app, |_| {});
}

fn finish_hydrate(app: &mut App) {
    apply_emu_settings_hint(app);
    update_emu_patched_notice(app);
    render_emu_file_list(app);
}

fn hydrate_then(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    let Some(first) = app.state.emu.edit_targets.first().map(|t| str_of(t, "path")) else {
        app.state.emu.settings_prefill_path = None;
        app.state.emu.bypass_initial_state = false;
        if !app.ui.disabled("emu-bypass-toggle") {
            app.ui.set_checked("emu-bypass-toggle", false);
        }
        finish_hydrate(app);
        return then(app);
    };
    let targets = paths_of(&app.state.emu.edit_targets);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let initial = tasks.invoke::<Value>("emu_read_emu_settings", json!({ "targetPath": first })).await;
        let installed = tasks.invoke::<bool>("steam_api_bypass_status", json!({ "targets": targets })).await.unwrap_or(false);
        tasks.post(move |app| {
            match initial {
                Ok(initial) => {
                    fields::populate(app, &initial);
                    app.state.emu.settings_prefill_path = Some(first);
                }
                Err(error) => {
                    app.state.emu.settings_prefill_path = None;
                    let text = t!(app, "emulator.settingsReadError", message = app.i18n.localize_error(&error));
                    set_emu_apply_status(app, "error", text, None);
                }
            }
            app.state.emu.bypass_initial_state = installed;
            if !app.ui.disabled("emu-bypass-toggle") {
                app.ui.set_checked("emu-bypass-toggle", installed);
            }
            finish_hydrate(app);
            then(app);
        });
    });
}

async fn refresh_emu_scan_in_place(tasks: &Tasks) {
    let Some(Some(dir)) = tasks.ask(|app| app.state.download_dir.clone()).await else { return };
    let Ok(scanned) = tasks.invoke::<Vec<Value>>("emu_scan_game_dir", json!({ "gameDir": dir })).await else { return };
    tasks
        .ask(move |app| {
            refresh_emu_patched_state(app, scanned);
            app.state.emu.settings_prefill_path = app.state.emu.edit_targets.first().map(|t| str_of(t, "path"));
            finish_hydrate(app);
        })
        .await;
}

pub fn get_selected_emu_targets(app: &App) -> Vec<Value> {
    let emu = &app.state.emu;
    if !emu.selection_explicit && emu.selected_files.is_empty() {
        return emu.scan.clone();
    }
    emu.scan.iter().filter(|f| emu.selected_files.contains(&str_of(f, "path"))).cloned().collect()
}

pub fn relativize_emu_path(app: &App, path: &str) -> String {
    match app.state.download_dir.as_deref().and_then(|dir| path.strip_prefix(dir)) {
        Some(rest) => rest.trim_start_matches(['\\', '/']).to_string(),
        None => path.to_string(),
    }
}

fn load_emu_release_info(app: &mut App) {
    app.ui.remove_class("emu-release-status", "emu-release-status--ready");
    app.ui.add_class("emu-release-status", "emu-release-status--busy");
    let loading = t!(app, "emulator.releaseLoading");
    app.ui.set_text("emu-release-status", loading);
    app.tasks.call("emu_release_info", Value::Null, |app, info: Result<Value, String>| {
        app.ui.remove_class("emu-release-status", "emu-release-status--busy");
        match info {
            Ok(info) => {
                app.ui.add_class("emu-release-status", "emu-release-status--ready");
                let (tag, local) = (str_of(&info, "tag"), str_of(&info, "local_tag"));
                let text = if !local.is_empty() && local != tag { t!(app, "emulator.releaseNewer", tag = tag, local = local) } else { t!(app, "emulator.releaseReady", tag = tag) };
                app.ui.set_text("emu-release-status", text);
                app.state.emu.release_info = Some(info);
                refresh_emu_patch_version(app);
            }
            Err(error) => {
                app.ui.remove_class("emu-release-status", "emu-release-status--ready");
                let text = app.i18n.localize_error(&error);
                app.ui.set_text("emu-release-status", text);
            }
        }
    });
}

pub fn current_app_id_for_emu(app: &App) -> String {
    app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).filter(|id| !id.is_empty()).or_else(|| app.state.search_app_id.map(|id| id.to_string())).unwrap_or_default()
}

pub fn selected_emu_variant(app: &App) -> &'static str {
    if app.ui.checked("emu-variant-experimental") {
        "experimental"
    } else {
        "regular"
    }
}

fn set_variant(app: &mut App, variant: &str) {
    let experimental = variant == "experimental";
    if app.ui.checked("emu-variant-experimental") != experimental || app.ui.checked("emu-variant-regular") == experimental {
        app.ui.set_checked("emu-variant-experimental", experimental);
        app.ui.set_checked("emu-variant-regular", !experimental);
        render_emu_controller_state(app);
    }
}

async fn sync_emu_bypass(tasks: &Tasks, targets: Vec<Value>) -> String {
    let Some(Some((want, initial))) = tasks.ask(|app| (!app.ui.disabled("emu-bypass-toggle")).then(|| (app.ui.checked("emu-bypass-toggle"), app.state.emu.bypass_initial_state))).await else {
        return String::new();
    };
    if want == initial {
        return String::new();
    }
    let (ok, message, event) = if want {
        let windows: Vec<Value> = targets.iter().filter(|t| t["platform"] == "windows").cloned().collect();
        if windows.is_empty() {
            (true, String::new(), None)
        } else {
            match tasks.invoke::<Vec<Value>>("steam_api_bypass_apply", json!({ "targets": windows })).await {
                Ok(results) => {
                    let success = results.iter().filter(|r| truthy(&r["success"])).count();
                    let failed = results.len() - success;
                    let event = json!({ "action": "apply", "ok": failed == 0, "targets": telemetry::count_bucket(results.len()) });
                    let detail = results.iter().find(|r| !truthy(&r["success"])).and_then(|r| opt_str(r, "error")).map(|e| format!("\n{e}")).unwrap_or_default();
                    (failed == 0, format!("{success}|{failed}|{detail}"), Some(event))
                }
                Err(error) => (false, format!("!{error}"), None),
            }
        }
    } else {
        match tasks.invoke_raw("steam_api_bypass_revert", json!({ "targets": paths_of(&targets) })).await {
            Ok(_) => (true, String::new(), Some(json!({ "action": "revert", "ok": true }))),
            Err(error) => (false, format!("!{error}"), None),
        }
    };
    tasks
        .ask(move |app| {
            if let Some(event) = event {
                app.emit_event("api_bypass", event);
            }
            if ok {
                app.state.emu.bypass_initial_state = want;
            }
            if let Some(error) = message.strip_prefix('!') {
                return t!(app, "emulator.bypassError", message = app.i18n.localize_error(error));
            }
            let mut parts = message.splitn(3, '|');
            match (parts.next().and_then(|v| v.parse::<usize>().ok()), parts.next().and_then(|v| v.parse::<usize>().ok())) {
                (Some(success), Some(0)) => t!(app, "emulator.bypassSuccess", count = success),
                (Some(success), Some(failed)) => format!("{}{}", t!(app, "emulator.bypassPartial", success = success, failed = failed), parts.next().unwrap_or("")),
                _ => String::new(),
            }
        })
        .await
        .unwrap_or_default()
}

pub fn sync_emu_game_data_key(app: &mut App) {
    app.tasks.call("get_settings", Value::Null, |app, settings: Result<Value, String>| {
        let Ok(settings) = settings else { return };
        app.ui.set_value("emu-webapi-key", str_of(&settings, "steam_web_api_key"));
        let media = opt_str(&settings, "game_data_media").unwrap_or_else(|| "off".to_string());
        set_emu_game_media(app, &media);
    });
}

async fn save_setting(tasks: &Tasks, key: &'static str, value: String) -> Result<(), String> {
    let mut settings = tasks.invoke::<Value>("get_settings", Value::Null).await?;
    let current = match &settings[key] {
        Value::String(text) => text.clone(),
        _ if key == "game_data_media" => "off".to_string(),
        _ => String::new(),
    };
    if current == value {
        return Ok(());
    }
    settings[key] = json!(value);
    tasks.invoke_raw("save_settings", json!({ "settings": settings })).await?;
    tasks.invoke_raw("emit_telemetry_event", json!({ "kind": "settings_saved", "props": { "keys": [key], "from": "emulator" } })).await.ok();
    Ok(())
}

fn set_emu_game_data_status(app: &mut App, kind: &str, text: String) {
    app.ui.set_hidden("emu-gamedata-status", text.is_empty());
    app.ui.set_class("emu-gamedata-status", "dd-path__hint--error", kind == "error");
    app.ui.set_text("emu-gamedata-status", text);
}

fn game_data_summary(app: &App, r: &Value) -> String {
    let count = |key: &str| r[key].as_u64().unwrap_or(0);
    let label = |key: &str, n: u64| app.i18n.tp(&format!("emulator.gameData.{key}"), &[("count", &n)]);
    let mut parts = Vec::new();
    for (field, key) in [("languages", "languages"), ("depots", "depots"), ("branches", "branches")] {
        if count(field) > 0 {
            parts.push(label(key, count(field)));
        }
    }
    if count("achievements") > 0 {
        let (n, languages, icons) = (count("achievements"), count("achievement_languages"), count("icons"));
        parts.push(if languages > 1 {
            app.i18n.tp("emulator.gameData.achievementLanguages", &[("count", &n), ("languages", &languages), ("icons", &icons)])
        } else {
            app.i18n.tp("emulator.gameData.achievements", &[("count", &n), ("icons", &icons)])
        });
    }
    for (field, key) in [("stats", "stats"), ("leaderboards", "leaderboards"), ("items", "items"), ("cloud_dirs", "cloud"), ("watcher_schemas", "watcher"), ("media_files", "mediaFiles")] {
        if count(field) > 0 {
            parts.push(label(key, count(field)));
        }
    }
    let mut lines = vec![if parts.is_empty() { app.i18n.t("emulator.gameData.nothing").to_string() } else { app.i18n.tp("emulator.gameData.written", &[("list", &parts.join(", "))]) }];
    for note in r["notes"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        lines.push(app.i18n.t(&format!("emulator.{note}")).to_string());
    }
    lines.join("\n")
}

async fn write_emu_game_data(tasks: &Tasks, targets: Vec<Value>, app_id: String, language: Option<String>) -> String {
    let paths = paths_of(&targets);
    if paths.is_empty() || app_id.is_empty() {
        return String::new();
    }
    tasks
        .ask(|app| {
            let busy = t!(app, "emulator.gameData.busy");
            set_emu_game_data_status(app, "busy", busy);
        })
        .await;
    let result = tasks.invoke::<Value>("emu_generate_game_data", json!({ "targets": paths, "appId": app_id, "language": language })).await;
    tasks
        .ask(move |app| match result {
            Ok(r) => {
                let bucket = |key: &str| telemetry::count_bucket(r[key].as_u64().unwrap_or(0) as usize);
                let notes: Vec<String> = r["notes"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|n| n.replace("gameData.", "")).collect();
                let achievements = r["achievements"].as_u64().unwrap_or(0);
                app.emit_event(
                    "game_data_written",
                    json!({
                        "source": r["source"],
                        "achievements": bucket("achievements"),
                        "achievement_languages": bucket("achievement_languages"),
                        "stats": bucket("stats"),
                        "languages": bucket("languages"),
                        "depots": bucket("depots"),
                        "branches": bucket("branches"),
                        "leaderboards": bucket("leaderboards"),
                        "items": bucket("items"),
                        "cloud_dirs": bucket("cloud_dirs"),
                        "watcher": r["watcher_schemas"].as_u64().unwrap_or(0) > 0,
                        "media": current_emu_game_media(app),
                        "icons": (achievements > 0).then(|| r["icons"].as_u64().unwrap_or(0) >= achievements),
                        "notes": if notes.is_empty() { vec!["none".to_string()] } else { notes },
                    }),
                );
                let text = game_data_summary(app, &r);
                let ok = achievements > 0 || r["languages"].as_u64().unwrap_or(0) > 0;
                set_emu_game_data_status(app, if ok { "ok" } else { "error" }, text.clone());
                text
            }
            Err(error) => {
                let text = t!(app, "emulator.gameData.failed", message = app.i18n.localize_error(&error));
                telemetry::report_error(app, "game_data", Some(&error), None);
                set_emu_game_data_status(app, "error", text.clone());
                text
            }
        })
        .await
        .unwrap_or_default()
}

fn current_emu_game_media(app: &App) -> &'static str {
    ["images", "all"].into_iter().find(|value| app.ui.has_class(&format!("emu-gamedata-media-{value}"), "is-active")).unwrap_or("off")
}

fn set_emu_game_media(app: &mut App, value: &str) {
    for option in ["off", "images", "all"] {
        app.ui.set_class(&format!("emu-gamedata-media-{option}"), "is-active", option == value);
    }
}

fn init_emu_game_data(app: &mut App) {
    if app.store.get("emuGameData").as_deref() == Some("0") {
        app.ui.set_checked("emu-gamedata-toggle", false);
    }
    let off = !app.ui.checked("emu-gamedata-toggle");
    app.ui.set_class("emu-gamedata-section", "is-off", off);
}

fn toast_result(app: &mut App, result: Result<(), String>, success_key: Option<&str>) {
    match result {
        Ok(()) => {
            if let Some(key) = success_key {
                let text = t!(app, key);
                ui::show_toast(app, text, "success", 4000);
            }
        }
        Err(error) => {
            let text = app.i18n.localize_error(&error);
            ui::show_toast(app, text, "error", 4000);
        }
    }
}

fn run_game_data(app: &mut App) {
    let targets = get_selected_emu_targets(app);
    let app_id = current_app_id_for_emu(app);
    if targets.is_empty() || app_id.is_empty() {
        let text = t!(app, "emulator.applyNoSelection");
        return set_emu_game_data_status(app, "error", text);
    }
    let key = app.ui.value("emu-webapi-key").trim().to_string();
    let language = fields::gather(app).and_then(|g| opt_str(&g, "language"));
    app.ui.set_disabled("btn-emu-gamedata", true);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if !key.is_empty() {
            let _ = save_setting(&tasks, "steam_web_api_key", key).await;
        }
        write_emu_game_data(&tasks, targets, app_id, language).await;
        tasks.post(|app| app.ui.set_disabled("btn-emu-gamedata", false));
    });
}

fn emu_has_windows_targets(app: &App) -> bool {
    app.state.emu.scan.iter().any(|f| str_of(f, "path").to_lowercase().ends_with(".dll"))
}

pub fn emu_controller_on(app: &App) -> bool {
    app.ui.checked("emu-controller-toggle")
}

fn emu_controller_needs_experimental(app: &App) -> bool {
    if !emu_controller_on(app) || !emu_has_windows_targets(app) {
        return false;
    }
    app.state.emu.controller_file.is_some() || app.state.emu.controller_info.as_ref().is_none_or(|info| truthy(&info["layout"]))
}

fn render_emu_controller_state(app: &mut App) {
    let on = emu_controller_on(app);
    app.ui.set_class("emu-controller-section", "is-off", !on);
    let key = match &app.state.emu.controller_info {
        _ if app.state.emu.controller_checking => "checking",
        Some(info) if truthy(&info["layout"]) => "statusLayout",
        Some(info) if matches!(info["support"].as_str(), Some("full" | "partial")) => "statusSupport",
        Some(_) => "statusNone",
        None => "statusUnknown",
    };
    let status = t!(app, &format!("emulator.controller.{key}"));
    app.ui.set_text("emu-controller-status", status);
    let warning = t!(app, "emulator.controller.regularWarning");
    let text = if emu_controller_needs_experimental(app) {
        if selected_emu_variant(app) == "experimental" { t!(app, "emulator.controller.experimentalNote") } else { warning.clone() }
    } else {
        String::new()
    };
    app.ui.set_hidden("emu-controller-build", text.is_empty());
    app.ui.set_class("emu-controller-build", "dd-path__hint--error", text == warning);
    app.ui.set_text("emu-controller-build", text);
    let file = app.state.emu.controller_file.clone().unwrap_or_default();
    app.ui.set_hidden("btn-emu-controller-clear", file.is_empty());
    app.ui.set_value("emu-controller-file", file);
}

fn pick_emu_experimental_if_needed(app: &mut App) {
    if !emu_controller_needs_experimental(app) || app.state.emu.controller_variant_set {
        return;
    }
    set_variant(app, "experimental");
    app.state.emu.controller_variant_set = true;
}

pub fn load_emu_controller_info(app: &mut App) {
    app.state.emu.controller_info = None;
    app.state.emu.controller_variant_set = false;
    let app_id = current_app_id_for_emu(app);
    if app_id.is_empty() {
        return render_emu_controller_state(app);
    }
    app.state.emu.controller_checking = true;
    render_emu_controller_state(app);
    let asked = app_id.clone();
    app.tasks.call("emu_controller_info", json!({ "appId": app_id }), move |app, info: Result<Value, String>| {
        if let Ok(info) = info {
            if current_app_id_for_emu(app) == asked {
                app.state.emu.controller_info = Some(info);
            }
        }
        app.state.emu.controller_checking = false;
        pick_emu_experimental_if_needed(app);
        render_emu_controller_state(app);
    });
}

fn set_emu_controller_result(app: &mut App, kind: &str, text: String) {
    app.ui.set_hidden("emu-controller-result", text.is_empty());
    app.ui.set_class("emu-controller-result", "dd-path__hint--error", kind == "error");
    app.ui.set_text("emu-controller-result", text);
}

fn controller_summary(app: &App, r: &Value) -> String {
    let (sets, glyphs) = (r["sets"].as_u64().unwrap_or(0), r["glyphs"].as_u64().unwrap_or(0));
    let line = if sets == 0 {
        app.i18n.t("emulator.controller.nothing").to_string()
    } else if r["source"] == "file" {
        app.i18n.tp("emulator.controller.writtenFile", &[("count", &sets)])
    } else if glyphs > 0 {
        app.i18n.tp("emulator.controller.written", &[("count", &sets), ("glyphs", &glyphs)])
    } else {
        app.i18n.tp("emulator.controller.writtenNoGlyphs", &[("count", &sets)])
    };
    let mut lines = vec![line];
    for note in r["notes"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        lines.push(app.i18n.t(&format!("emulator.{note}")).to_string());
    }
    lines.join("\n")
}

async fn write_emu_controller(tasks: &Tasks, targets: Vec<Value>) -> String {
    let paths = paths_of(&targets);
    let Some((app_id, file)) = tasks
        .ask(|app| {
            let busy = t!(app, "emulator.controller.busy");
            set_emu_controller_result(app, "busy", busy);
            (current_app_id_for_emu(app), app.state.emu.controller_file.clone())
        })
        .await
    else {
        return String::new();
    };
    if paths.is_empty() || app_id.is_empty() {
        return String::new();
    }
    let result = tasks.invoke::<Value>("emu_write_controller", json!({ "targets": paths, "appId": app_id, "layoutFile": file })).await;
    tasks
        .ask(move |app| match result {
            Ok(r) => {
                let notes: Vec<String> = r["notes"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|n| n.split_once('.').map(|(_, rest)| rest.to_string()).unwrap_or_else(|| n.to_string())).collect();
                app.emit_event(
                    "controller_applied",
                    json!({
                        "source": r["source"],
                        "sets": telemetry::count_bucket(r["sets"].as_u64().unwrap_or(0) as usize),
                        "glyphs": r["glyphs"].as_u64().unwrap_or(0) > 0,
                        "variant": selected_emu_variant(app),
                        "steam_input": app.ui.checked("emu-set-disable_steamoverlaygameid_env_var"),
                        "notes": if notes.is_empty() { vec!["none".to_string()] } else { notes },
                    }),
                );
                let text = controller_summary(app, &r);
                set_emu_controller_result(app, "success", text.clone());
                text
            }
            Err(error) => {
                let text = t!(app, "emulator.controller.failed", message = app.i18n.localize_error(&error));
                set_emu_controller_result(app, "error", text.clone());
                text
            }
        })
        .await
        .unwrap_or_default()
}

fn choose_controller_file(app: &mut App) {
    let filters = vec![FileFilter { name: t!(app, "emulator.controller.filterName"), extensions: vec!["vdf".to_string(), "txt".to_string()] }];
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let Some(path) = platform.pick_file(None, filters).await else { return };
        let path = path.to_string_lossy().to_string();
        tasks.post(move |app| {
            app.state.emu.controller_file = Some(path);
            app.state.emu.controller_variant_set = false;
            pick_emu_experimental_if_needed(app);
            render_emu_controller_state(app);
        });
    });
}

pub fn refresh_emu_patch_version(app: &mut App) {
    let Some(target) = app.state.emu.edit_targets.first().map(|t| str_of(t, "path")) else { return app.ui.hide("emu-patch-version") };
    app.tasks.call("emu_patch_info", json!({ "target": target }), |app, info: Result<Value, String>| {
        let info = info.ok().filter(|i| i.is_object()).unwrap_or_else(|| json!({}));
        if let Some(variant) = info["variant"].as_str().filter(|v| matches!(*v, "regular" | "experimental")) {
            set_variant(app, variant);
        }
        let variant = match info["variant"].as_str() {
            Some("experimental") => t!(app, "emulator.variantExperimental"),
            Some("regular") => t!(app, "emulator.variantRegular"),
            _ => String::new(),
        };
        let tag = str_of(&info, "tag");
        let text = if tag.is_empty() {
            t!(app, "emulator.patchedWithUnknown")
        } else {
            t!(app, "emulator.patchedWith", tag = tag, variant = if variant.is_empty() { String::new() } else { format!(" ({variant})") })
        };
        app.ui.set_text("emu-patch-version-text", text);
        let latest = app.state.emu.release_info.as_ref().map(|i| str_of(i, "tag")).unwrap_or_default();
        let label = t!(app, "emulator.updateGbe", tag = latest);
        app.ui.set_text("btn-emu-update-gbe", label);
        app.ui.set_hidden("btn-emu-update-gbe", latest.is_empty() || latest == tag);
        app.ui.show("emu-patch-version");
        app.state.emu.patch_info = Some(info);
    });
}

fn update_emu_in_place(app: &mut App) {
    if app.state.emu.edit_targets.is_empty() || app.state.emu.busy {
        return;
    }
    app.state.emu.selected_files = app.state.emu.edit_targets.iter().map(|t| str_of(t, "path")).collect();
    app.state.emu.selection_explicit = true;
    let known = app.state.emu.patch_info.as_ref().is_some_and(|i| !str_of(i, "tag").is_empty());
    app.emit_event("emu_gbe_updated", json!({ "from": if known { "known" } else { "unknown" } }));
    apply_emu_replacement(app);
}

pub fn refresh_emu_drm_restore(app: &mut App) {
    let Some(dir) = app.state.download_dir.clone() else { return };
    app.tasks.call("emu_drm_backups", json!({ "gameDir": dir }), |app, backups: Result<Vec<Value>, String>| {
        app.state.emu.drm_backups = backups.unwrap_or_default();
        app.ui.set_hidden("emu-drm-restore", app.state.emu.drm_backups.is_empty());
    });
}

fn restore_one_drm(app: &mut App, path: String) {
    app.tasks.call("emu_restore_drm", json!({ "exe": path }), |app, result: Result<Value, String>| {
        match result {
            Ok(_) => {
                app.emit_event("drm_restored", json!({ "where": "single" }));
                let done = t!(app, "emulator.drmRestore.done");
                ui::show_toast(app, done, "success", 4000);
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                ui::show_toast(app, text, "error", 4000);
            }
        }
        refresh_emu_drm_restore(app);
        scan_for_drm(app);
    });
}

async fn restore_all_drm(tasks: &Tasks, dir: Option<String>) {
    let Some(dir) = dir else { return };
    let backups = tasks.invoke::<Vec<Value>>("emu_drm_backups", json!({ "gameDir": dir })).await.unwrap_or_default();
    let mut restored = 0;
    for backup in backups {
        if tasks.invoke_raw("emu_restore_drm", json!({ "exe": backup["path"] })).await.is_ok() {
            restored += 1;
        }
    }
    if restored > 0 {
        tasks.invoke_raw("emit_telemetry_event", json!({ "kind": "drm_restored", "props": { "where": "revert" } })).await.ok();
    }
}

fn collect_installed_app_ids(app: &App, main: &str) -> Vec<String> {
    app.state.parsed_data.as_ref().map(|d| d.all_app_ids.iter().filter(|id| !id.is_empty() && id.as_str() != main).cloned().collect()).unwrap_or_default()
}

fn parse_emu_download_confirm(error: &str) -> Option<Value> {
    let info: Value = serde_json::from_str(error.strip_prefix(DOWNLOAD_CONFIRM)?).ok()?;
    info["platforms"].as_array().is_some_and(|list| !list.is_empty()).then_some(info)
}

fn patch_event(app: &App, outcome: &str, variant: &str, targets: &[Value], failures: usize, fail_class: Value) -> Value {
    json!({
        "entry": telemetry::emu_entry_point(app),
        "outcome": outcome,
        "variant": variant,
        "platforms": telemetry::emu_platform_mix(targets),
        "targets": telemetry::count_bucket(targets.len()),
        "failures": telemetry::count_bucket(failures),
        "fail_class": fail_class,
    })
}

pub fn apply_emu_replacement(app: &mut App) {
    if app.state.emu.apply_complete {
        return progress::reset_app(app);
    }
    let cold_exe = extras::emu_cold_client_exe(app);
    let selected: Vec<Value> = get_selected_emu_targets(app).into_iter().filter(|t| cold_exe.is_none() || t["platform"] != "windows" || truthy(&t["loader"])).collect();
    if selected.is_empty() && cold_exe.is_none() {
        let text = t!(app, "emulator.applyNoSelection");
        return set_emu_apply_status(app, "error", text, None);
    }
    let app_id = current_app_id_for_emu(app);
    if app_id.is_empty() {
        let text = t!(app, "emulator.applyError", message = "missing app id");
        return set_emu_apply_status(app, "error", text, None);
    }
    set_emu_busy(app, true);
    let applying = t!(app, "emulator.applying");
    set_emu_apply_status(app, "busy", applying, None);
    let variant = selected_emu_variant(app);
    let gathered = fields::gather(app);
    let args = json!({
        "targets": selected,
        "variant": variant,
        "appId": app_id,
        "installedAppIds": collect_installed_app_ids(app, &app_id),
        "emuSettings": gathered.clone().unwrap_or_else(|| json!({})),
        "allowDownload": app.state.emu.allow_download,
        "preferLocal": app.state.emu.prefer_local,
        "coldclientExe": cold_exe,
    });
    let cold = cold_exe.is_some();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let outcome = tasks.invoke::<Vec<Value>>("emu_apply_replacement", args).await;
        match outcome {
            Ok(results) => {
                let total = results.len();
                let success = results.iter().filter(|r| truthy(&r["success"])).count();
                let failed = total - success;
                let done: Vec<Value> = results
                    .iter()
                    .filter(|r| truthy(&r["success"]))
                    .map(|r| selected.iter().find(|t| t["path"] == r["path"]).cloned().unwrap_or_else(|| json!({ "path": r["path"], "platform": "windows", "arch": "x64", "loader": true })))
                    .collect();
                let noted = results.clone();
                let picked = selected.clone();
                tasks
                    .ask(move |app| {
                        app.state.emu.allow_download = false;
                        app.state.emu.prefer_local = false;
                        if cold || noted.iter().any(|r| truthy(&r["shortcut"])) {
                            extras::note_emu_cold_client(app, &noted, cold);
                        }
                        let class = telemetry::emu_fail_class(&noted).map(Value::from).unwrap_or(Value::Null);
                        let event = patch_event(app, telemetry::emu_outcome(success, total), variant, &picked, failed, class);
                        app.emit_event("patch_applied", event);
                    })
                    .await;
                if failed == 0 {
                    let plain: Vec<Value> = done.iter().filter(|t| !truthy(&t["loader"])).cloned().collect();
                    let bypass = sync_emu_bypass(&tasks, plain).await;
                    let (key, game_data, controller) = tasks.ask(|app| (app.ui.value("emu-webapi-key").trim().to_string(), app.ui.checked("emu-gamedata-toggle"), emu_controller_on(app))).await.unwrap_or_default();
                    if !key.is_empty() {
                        let _ = save_setting(&tasks, "steam_web_api_key", key).await;
                    }
                    let language = gathered.as_ref().and_then(|g| opt_str(g, "language"));
                    let game_note = if game_data { format!("\n\n{}", write_emu_game_data(&tasks, done.clone(), app_id.clone(), language).await) } else { String::new() };
                    let controller_note = if controller { format!("\n\n{}", write_emu_controller(&tasks, done.clone()).await) } else { String::new() };
                    let dlc = results.iter().find_map(|r| r["dlcCount"].as_u64());
                    let standalone = tasks
                        .ask(move |app| {
                            let dlc_note = match dlc {
                                Some(0) => format!("\n{}", app.i18n.t("emulator.dlcNone")),
                                Some(count) => format!("\n{}", t!(app, "emulator.dlcActivated", count = count)),
                                None => String::new(),
                            };
                            let extra = if bypass.is_empty() { String::new() } else { format!("\n\n{bypass}") };
                            let text = format!("{}{dlc_note}{extra}{game_note}{controller_note}", t!(app, "emulator.applySuccess", count = success, total = total));
                            set_emu_apply_status(app, "success", text, None);
                            fields::save_last(app, gathered.as_ref());
                            if app.state.emu_edit_mode {
                                refresh_emu_patch_version(app);
                            } else if !app.state.emu_standalone {
                                app.state.emu.apply_complete = true;
                                let home = t!(app, "emulator.goBackHome");
                                app.ui.set_text("btn-emu-apply", home);
                            }
                            app.state.emu_standalone && !app.state.emu_edit_mode
                        })
                        .await
                        .unwrap_or(false);
                    if standalone {
                        refresh_emu_scan_in_place(&tasks).await;
                    }
                    tasks.ask(extras::sync_emu_after_patch).await;
                } else {
                    let standalone = tasks
                        .ask(move |app| {
                            let failures: Vec<&Value> = results.iter().filter(|r| !truthy(&r["success"])).collect();
                            let details = failures
                                .iter()
                                .map(|r| format!("{}\n    {}", emu_file_label(&str_of(r, "path")), opt_str(r, "error").unwrap_or_else(|| app.i18n.t("emulator.applyReasonUnknown").to_string())))
                                .collect::<Vec<_>>()
                                .join("\n");
                            let errors: Vec<String> = failures.iter().map(|r| str_of(r, "error")).collect();
                            let sources: Vec<&str> = errors.iter().map(String::as_str).collect();
                            let summary = telemetry::with_emu_hint(app, &t!(app, "emulator.applyPartial", success = success, failed = failed), &sources);
                            set_emu_apply_status(app, "error", summary, Some(details));
                            app.state.emu_standalone
                        })
                        .await
                        .unwrap_or(false);
                    if standalone {
                        refresh_emu_scan_in_place(&tasks).await;
                    }
                }
            }
            Err(error) => {
                tasks
                    .ask(move |app| {
                        if let Some(info) = parse_emu_download_confirm(&error) {
                            app.state.emu.pending_download = Some((selected, variant.to_string()));
                            return show_emu_download_confirm(app, &info);
                        }
                        app.state.emu.allow_download = false;
                        let event = patch_event(app, "failed", selected_emu_variant(app), &selected, selected.len(), json!(telemetry::classify_emu_command_error(&error)));
                        app.emit_event("patch_applied", event);
                        if error.contains("AV_BLOCKED") {
                            set_emu_apply_antivirus_blocked(app);
                        } else {
                            let text = telemetry::with_emu_hint(app, &t!(app, "emulator.applyError", message = app.i18n.localize_error(&error)), &[&error]);
                            set_emu_apply_status(app, "error", text, None);
                        }
                    })
                    .await;
            }
        }
        tasks.post(|app| {
            set_emu_busy(app, false);
            app.state.emu.download_listening = false;
        });
    });
}

fn set_emu_apply_antivirus_blocked(app: &mut App) {
    app.ui.set_classes("emu-apply-status", &["completion-message", "completion-message--error"]);
    let html = format!(
        r#"<div class="av-blocked"><div class="av-blocked__title">{}</div><p class="av-blocked__hint">{}</p><button id="btn-av-retry" class="btn btn--primary av-blocked__retry" data-click="btn-av-retry">{}</button></div>"#,
        escape_html(app.i18n.t("emulator.avBlockedTitle")),
        escape_html(app.i18n.t("emulator.avBlockedHint")),
        escape_html(app.i18n.t("emulator.avBlockedRetry")),
    );
    app.ui.set_html("emu-apply-status", html);
}

fn emu_settings_dir_of(path: &str) -> &str {
    path.rfind(['\\', '/']).map(|index| &path[..index]).unwrap_or(path)
}

async fn folder_gone(tasks: &Tasks, dir: Option<String>) -> bool {
    match dir {
        Some(dir) => tasks.invoke_raw("emu_scan_game_dir", json!({ "gameDir": dir })).await.is_err(),
        None => false,
    }
}

fn leave_for_missing_folder(app: &mut App) {
    set_emu_busy(app, false);
    progress::reset_app(app);
    history::show_folder_missing(app);
}

fn save_emu_settings(app: &mut App) {
    if app.state.emu.edit_targets.is_empty() || app.state.emu.busy {
        return;
    }
    set_emu_busy(app, true);
    let dir = app.state.download_dir.clone();
    let targets = app.state.emu.edit_targets.clone();
    let settings = fields::gather(app).unwrap_or_else(|| json!({}));
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if folder_gone(&tasks, dir).await {
            return tasks.post(leave_for_missing_folder);
        }
        tasks
            .ask(|app| {
                let saving = t!(app, "emulator.savingSettings");
                set_emu_apply_status(app, "busy", saving, None);
            })
            .await;
        let mut seen = HashSet::new();
        let (mut success, mut failed) = (0usize, 0usize);
        for target in &targets {
            let path = str_of(target, "path");
            if !seen.insert(emu_settings_dir_of(&path).to_string()) {
                continue;
            }
            match tasks.invoke_raw("emu_write_emu_settings", json!({ "targetPath": path, "settings": settings })).await {
                Ok(_) => success += 1,
                Err(_) => failed += 1,
            }
        }
        let listed = targets.clone();
        let proceed = tasks
            .ask(move |app| {
                app.emit_event(
                    "patch_settings_saved",
                    json!({
                        "entry": telemetry::emu_entry_point(app),
                        "outcome": telemetry::emu_outcome(success, success + failed),
                        "platforms": telemetry::emu_platform_mix(&listed),
                        "targets": telemetry::count_bucket(success + failed),
                        "failures": telemetry::count_bucket(failed),
                    }),
                );
                if failed > 0 {
                    set_emu_busy(app, false);
                    let text = t!(app, "emulator.savePartial", success = success, failed = failed);
                    set_emu_apply_status(app, "error", text, None);
                }
                (failed == 0).then(|| emu_controller_on(app) && !current_app_id_for_emu(app).is_empty())
            })
            .await
            .flatten();
        let Some(controller) = proceed else { return };
        let bypass = sync_emu_bypass(&tasks, targets.clone()).await;
        let controller_note = if controller { write_emu_controller(&tasks, targets).await } else { String::new() };
        tasks.post(move |app| {
            set_emu_busy(app, false);
            if !app.state.emu_standalone {
                return progress::reset_app(app);
            }
            let extra: String = [bypass, controller_note].iter().filter(|m| !m.is_empty()).map(|m| format!("\n\n{m}")).collect();
            let text = format!("{}{extra}", t!(app, "emulator.saveSuccess", count = success));
            set_emu_apply_status(app, "success", text, None);
        });
    });
}

fn platform_display_name(platform: &str) -> &str {
    match platform {
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    }
}

pub fn show_emu_download_confirm(app: &mut App, info: &Value) {
    let items: Vec<String> = info["platforms"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let size = p["size"].as_f64().and_then(format_bytes).map(|size| format!(" (~{size})")).unwrap_or_default();
            format!("{}{size}", platform_display_name(&str_of(p, "platform")))
        })
        .collect();
    let tag = str_of(info, "tag");
    let mut scope = t!(app, "modals.emuDownload.scope", tag = tag, items = items.join(", "));
    if app.ui.checked("emu-method-coldclient") {
        scope.push(' ');
        scope.push_str(app.i18n.t("modals.emuDownload.coldNote"));
    }
    app.ui.set_text("emu-download-scope", scope);
    let local = app.state.emu.release_info.as_ref().map(|i| str_of(i, "local_tag")).unwrap_or_default();
    let no = if !local.is_empty() && local != tag { t!(app, "modals.emuDownload.keepLocal", tag = local) } else { t!(app, "modals.emuDownload.no") };
    app.ui.set_text("btn-emu-download-no", no);
    app.state.emu.download_tag = (!tag.is_empty()).then_some(tag);
    app.ui.show("emu-download-modal");
}

pub fn resolve_url(app: &App, url: &str) -> String {
    match (&app.state.emu.download_tag, url.strip_prefix(RELEASES)) {
        (Some(tag), Some(_)) if !app.ui.hidden("emu-download-modal") => format!("{RELEASES}tag/{tag}"),
        _ => url.to_string(),
    }
}

fn confirm_emu_download(app: &mut App) {
    app.ui.hide("emu-download-modal");
    app.state.emu.allow_download = true;
    app.state.emu.pending_download = None;
    app.state.emu.download_listening = true;
    if std::mem::take(&mut app.state.emu.lobby_retry) {
        return extras::start_emu_lobby_connect(app, true);
    }
    let text = t!(app, "emulator.emuDownloading", progress = "0%");
    set_emu_apply_status(app, "busy", text, None);
    apply_emu_replacement(app);
}

pub fn show_emu_download_progress(app: &mut App, info: &Value) {
    if !app.state.emu.download_listening {
        return;
    }
    let label = platform_display_name(&str_of(info, "platform")).to_string();
    let (downloaded, total) = (info["downloaded"].as_f64().unwrap_or(0.0), info["total"].as_f64().unwrap_or(0.0));
    if total > 0.0 && downloaded >= total {
        let text = t!(app, "emulator.emuExtracting", platform = if label.is_empty() { String::new() } else { format!(" ({label})") });
        return set_emu_apply_status(app, "busy", text, None);
    }
    let done = format_bytes(downloaded).unwrap_or_default();
    let progress = if total > 0.0 { format!("{}% · {done}/{}", (downloaded / total * 100.0).floor(), format_bytes(total).unwrap_or_default()) } else { done };
    let text = t!(app, "emulator.emuDownloading", progress = if label.is_empty() { progress } else { format!("{label} {progress}") });
    set_emu_apply_status(app, "busy", text, None);
}

fn decline_emu_download(app: &mut App) {
    let local = app.state.emu.release_info.as_ref().map(|i| str_of(i, "local_tag")).unwrap_or_default();
    if app.state.emu.lobby_retry || local.is_empty() || app.state.emu.pending_download.is_none() {
        return cancel_emu_download(app);
    }
    app.ui.hide("emu-download-modal");
    app.state.emu.pending_download = None;
    app.state.emu.prefer_local = true;
    apply_emu_replacement(app);
}

fn cancel_emu_download(app: &mut App) {
    app.ui.hide("emu-download-modal");
    app.state.emu.download_listening = false;
    app.state.emu.lobby_retry = false;
    let Some((targets, variant)) = app.state.emu.pending_download.take() else { return };
    let event = patch_event(app, "cancelled", &variant, &targets, 0, Value::Null);
    app.emit_event("patch_applied", event);
    let text = t!(app, "emulator.applyCancelled");
    set_emu_apply_status(app, "error", text, None);
}

fn show_emu_revert_confirm(app: &mut App) {
    let loader = app.state.emu.edit_targets.iter().any(|t| truthy(&t["loader"]));
    let mut scope = t!(app, "modals.emuRevert.scope", count = app.state.emu.edit_targets.len());
    if loader {
        scope.push(' ');
        scope.push_str(app.i18n.t("modals.emuRevert.loader"));
    }
    app.ui.set_text("emu-revert-scope", scope);
    app.ui.show("emu-revert-modal");
}

fn confirm_emu_revert(app: &mut App) {
    app.ui.hide("emu-revert-modal");
    if app.state.emu.edit_targets.is_empty() || app.state.emu.busy {
        return;
    }
    set_emu_busy(app, true);
    let dir = app.state.download_dir.clone();
    let targets = app.state.emu.edit_targets.clone();
    let entry = telemetry::emu_entry_point(app);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if folder_gone(&tasks, dir.clone()).await {
            return tasks.post(leave_for_missing_folder);
        }
        tasks
            .ask(|app| {
                let reverting = t!(app, "emulator.reverting");
                set_emu_apply_status(app, "busy", reverting, None);
            })
            .await;
        let results = match tasks.invoke::<Vec<Value>>("emu_revert_replacement", json!({ "targets": paths_of(&targets), "closeSteam": true })).await {
            Ok(results) => results,
            Err(error) => {
                return tasks.post(move |app| {
                    let event = json!({ "entry": entry, "outcome": "failed", "platforms": telemetry::emu_platform_mix(&targets), "targets": telemetry::count_bucket(targets.len()), "failures": telemetry::count_bucket(targets.len()), "fail_class": telemetry::classify_emu_command_error(&error) });
                    app.emit_event("patch_reverted", event);
                    set_emu_busy(app, false);
                    let text = t!(app, "emulator.revertError", message = app.i18n.localize_error(&error));
                    set_emu_apply_status(app, "error", text, None);
                });
            }
        };
        let plain: Vec<String> = targets.iter().filter(|t| !truthy(&t["loader"])).map(|t| str_of(t, "path")).collect();
        if !plain.is_empty() {
            tasks.invoke_raw("steam_api_bypass_revert", json!({ "targets": plain })).await.ok();
        }
        restore_all_drm(&tasks, dir).await;
        let success = results.iter().filter(|r| truthy(&r["success"])).count();
        let failed = results.len() - success;
        let standalone = tasks
            .ask(move |app| {
                let details = results
                    .iter()
                    .filter(|r| !truthy(&r["success"]))
                    .map(|r| {
                        let reason = if r["failClass"] == "steam_running" { app.i18n.t("emulator.method.steamRunning").to_string() } else { opt_str(r, "error").unwrap_or_else(|| app.i18n.t("emulator.applyReasonUnknown").to_string()) };
                        format!("{}\n    {reason}", emu_file_label(&str_of(r, "path")))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if targets.iter().any(|t| truthy(&t["loader"])) {
                    let links: u64 = results.iter().map(|r| r["links"].as_u64().unwrap_or(0)).sum();
                    if links > 0 {
                        let text = t!(app, "emulator.method.linksBack", count = links);
                        ui::show_toast(app, text, "success", 4000);
                    }
                    app.emit_event("coldclient_used", json!({ "action": "removed", "ok": failed == 0, "links": links > 0 }));
                }
                let class = telemetry::emu_fail_class(&results).map(Value::from).unwrap_or(Value::Null);
                app.emit_event(
                    "patch_reverted",
                    json!({ "entry": entry, "outcome": telemetry::emu_outcome(success, results.len()), "platforms": telemetry::emu_platform_mix(&targets), "targets": telemetry::count_bucket(results.len()), "failures": telemetry::count_bucket(failed), "fail_class": class }),
                );
                if !app.state.emu_standalone {
                    set_emu_busy(app, false);
                    if failed > 0 {
                        let text = t!(app, "emulator.revertPartial", success = success, failed = failed);
                        set_emu_apply_status(app, "error", text, Some(details));
                    } else {
                        progress::reset_app(app);
                    }
                    return None;
                }
                app.state.emu.bypass_initial_state = false;
                if !app.ui.disabled("emu-bypass-toggle") {
                    app.ui.set_checked("emu-bypass-toggle", false);
                }
                Some(details)
            })
            .await
            .flatten();
        let Some(details) = standalone else { return };
        refresh_emu_scan_in_place(&tasks).await;
        tasks.post(move |app| {
            app.state.emu.selected_files = app.state.emu.scan.iter().filter(|f| !truthy(&f["is_patched"])).map(|f| str_of(f, "path")).collect();
            app.state.emu.settings_prefill_path = app.state.emu.edit_targets.first().map(|t| str_of(t, "path"));
            render_emu_file_list(app);
            set_emu_busy(app, false);
            if failed > 0 {
                let text = t!(app, "emulator.revertPartial", success = success, failed = failed);
                set_emu_apply_status(app, "error", text, Some(details));
            } else {
                let text = t!(app, "emulator.revertSuccess", count = success);
                set_emu_apply_status(app, "success", text, None);
            }
        });
    });
}

fn apply_emu_start_over_visibility(app: &mut App) {
    let hidden = app.state.emu_edit_mode || app.state.emu_standalone;
    app.ui.set_hidden("btn-emu-start-over", hidden);
}

fn refresh_emu_denuvo(app: &mut App) {
    app.ui.hide("emu-denuvo-note");
    let app_id = current_app_id_for_emu(app);
    if app_id.is_empty() {
        return;
    }
    let asked = app_id.clone();
    app.tasks.call("get_steam_app_info", json!({ "appId": app_id }), move |app, info: Result<Value, String>| {
        let denuvo = info.is_ok_and(|info| truthy(&info["denuvo"]));
        if current_app_id_for_emu(app) != asked {
            return;
        }
        app.ui.set_hidden("emu-denuvo-note", !denuvo);
        if denuvo {
            select::report_denuvo(app, "emulator", &asked);
        }
    });
}

pub fn set_emu_edit_mode(app: &mut App, on: bool) {
    app.state.emu_edit_mode = on;
    app.ui.set_hidden("emu-release-status", on);
    if on {
        app.ui.hide("emu-drm-section");
    }
    apply_emu_start_over_visibility(app);
    let (title, description, home) = (
        t!(app, if on { "emulator.editTitle" } else { "emulator.title" }),
        t!(app, if on { "emulator.editDescription" } else { "emulator.description" }),
        t!(app, if on { "emulator.backToHome" } else { "emulator.goToHome" }),
    );
    app.ui.set_text("emu-header", title);
    app.ui.set_text("emu-description", description);
    app.ui.set_text("btn-emu-new", home);
    update_emu_action_buttons(app);
}

pub fn open_emu_edit_from_history(app: &mut App, entry: Value) {
    let Some(dir) = opt_str(&entry, "download_dir") else { return };
    app.state.emu_standalone = false;
    app.state.download_dir = Some(dir.clone());
    app.state.game_name = opt_str(&entry, "game_name");
    app.state.header_image = opt_str(&entry, "header_image");
    let app_id = str_of(&entry, "app_id");
    match app.state.parsed_data.as_mut() {
        Some(data) => data.main_app_id = app_id,
        None => app.state.set_parsed(Some(ParsedData { main_app_id: app_id, ..Default::default() })),
    }
    app.tasks.call("emu_scan_game_dir", json!({ "gameDir": dir }), |app, scanned: Result<Vec<Value>, String>| {
        let Ok(scanned) = scanned else { return history::show_folder_missing(app) };
        let patched: Vec<Value> = scanned.into_iter().filter(|f| truthy(&f["is_patched"])).collect();
        if patched.is_empty() {
            let text = t!(app, "emulator.editNoPatches");
            return history::show_history_banner(app, text, 6000, "error");
        }
        app.state.emulator_available = true;
        app.state.emu.apply_complete = false;
        app.state.emu.selection_explicit = true;
        app.state.emu.selected_files.clear();
        history::close_history(app);
        refresh_emu_patched_state(app, patched);
        hide_emu_already_patched_notice(app);
        set_emu_edit_mode(app, true);
        steps::go_to_step(app, 5);
        app.ui.hide("emu-apply-status");
        hydrate_then(app, |app| {
            load_emu_controller_info(app);
            refresh_emu_drm_restore(app);
            extras::reset_emu_extras_view(app);
            extras::refresh_emu_method(app);
            refresh_emu_denuvo(app);
            load_emu_release_info(app);
        });
    });
}

pub fn set_emu_apply_status(app: &mut App, kind: &str, text: String, details: Option<String>) {
    let mut classes: Vec<&'static str> = vec!["completion-message"];
    match kind {
        "success" => classes.push("completion-message--success"),
        "error" => classes.push("completion-message--error"),
        _ => {}
    }
    let details = details.filter(|d| !d.is_empty());
    if details.is_some() {
        classes.push("completion-message--details");
    }
    app.ui.set_classes("emu-apply-status", &classes);
    app.ui.set_text("emu-apply-status", match details {
        Some(details) => format!("{text}\n\n{details}"),
        None => text,
    });
}

fn emu_file_label(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return path.to_string();
    }
    parts[parts.len().saturating_sub(2)..].join("/")
}

pub fn reset_state(app: &mut App) {
    let emu = &mut app.state.emu;
    emu.scan.clear();
    emu.selected_files.clear();
    emu.edit_targets.clear();
    emu.patched_paths.clear();
    emu.settings_prefill_path = None;
    emu.busy = false;
    emu.apply_complete = false;
    emu.selection_explicit = false;
    emu.bypass_initial_state = false;
    emu.drm_targets.clear();
    app.state.emu_standalone = false;
    app.ui.hide("emu-drm-section");
    app.ui.hide("emu-drm-status-wrap");
    app.ui.set_checked("emu-bypass-toggle", false);
    set_emu_edit_mode(app, false);
    app.ui.hide("emu-apply-status");
    fields::populate(app, &Value::Null);
}

pub fn init(app: &mut App) {
    init_emu_game_data(app);
    render_emu_controller_state(app);
}

pub fn event(app: &mut App, channel: &str, payload: &Value) -> bool {
    if channel == "emu-download-progress" {
        show_emu_download_progress(app, payload);
        return true;
    }
    false
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-emu-apply" => apply_emu_replacement(app),
        "btn-emu-save-settings" => save_emu_settings(app),
        "btn-emu-merge-dlcs" => perform_dlc_merge(app),
        "btn-emu-drm-remove" => remove_drm(app),
        "btn-emu-drm-copy" => copy_drm_log(app),
        "btn-emu-new" => progress::reset_app(app),
        "btn-emu-start-over" => steps::go_to_step(app, 2),
        "btn-emu-revert" => show_emu_revert_confirm(app),
        "btn-emu-revert-yes" => confirm_emu_revert(app),
        "btn-emu-revert-no" | "emu-revert-modal>modal__backdrop" => app.ui.hide("emu-revert-modal"),
        "btn-emu-download-yes" => confirm_emu_download(app),
        "btn-emu-download-no" => decline_emu_download(app),
        "emu-download-modal>modal__backdrop" => cancel_emu_download(app),
        "btn-emu-update-gbe" => update_emu_in_place(app),
        "btn-av-retry" => {
            let applying = t!(app, "emulator.applying");
            set_emu_apply_status(app, "busy", applying, None);
            apply_emu_replacement(app);
        }
        "btn-emu-webapi-save" => {
            let key = app.ui.value("emu-webapi-key").trim().to_string();
            let tasks = app.tasks.clone();
            app.tasks.spawn(async move {
                let result = save_setting(&tasks, "steam_web_api_key", key).await;
                tasks.post(move |app| toast_result(app, result, Some("emulator.gameData.keySaved")));
            });
        }
        "btn-emu-gamedata" => run_game_data(app),
        "btn-emu-controller-file" => choose_controller_file(app),
        "btn-emu-controller-clear" => {
            app.state.emu.controller_file = None;
            render_emu_controller_state(app);
        }
        _ => {
            if let Some(section) = id.strip_prefix("emu-tab-") {
                select_emu_tab(app, section);
                report_emu_section(app, section);
            } else if let Some(value) = id.strip_prefix("emu-gamedata-media:") {
                set_emu_game_media(app, value);
                let value = value.to_string();
                let tasks = app.tasks.clone();
                app.tasks.spawn(async move {
                    let result = save_setting(&tasks, "game_data_media", value).await;
                    tasks.post(move |app| toast_result(app, result, None));
                });
            } else if let Some(path) = id.strip_prefix("drm-restore:") {
                restore_one_drm(app, path.to_string());
            } else {
                return fields::click(app, id) || extras::click(app, id);
            }
        }
    }
    true
}

pub fn changed(app: &mut App, id: &str) -> bool {
    match id {
        "emu-gamedata-toggle" => {
            let on = app.ui.checked(id);
            app.ui.set_class("emu-gamedata-section", "is-off", !on);
            app.store.set("emuGameData", if on { "1" } else { "0" });
        }
        "emu-controller-toggle" => {
            if app.ui.checked(id) {
                pick_emu_experimental_if_needed(app);
            }
            render_emu_controller_state(app);
        }
        "emu-variant-regular" | "emu-variant-experimental" => render_emu_controller_state(app),
        "emu-bypass-toggle" => {}
        _ => return fields::changed(app, id) || extras::changed(app, id),
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    matches!(id, "emu-webapi-key" | "emu-controller-file") || fields::input(app, id) || extras::input(app, id)
}

fn fill_file_list(app: &mut App, dom: &mut Dom<'_>) {
    let files = app.state.emu.scan.clone();
    let mut toggled: Option<String> = None;
    for file in &files {
        let path = str_of(file, "path");
        let loader = opt_str(file, "loader");
        let shown = match &loader {
            Some(exe) => t!(app, "emulator.method.loaderRow", exe = relativize_emu_path(app, exe)),
            None => relativize_emu_path(app, &path),
        };
        let linux = file["platform"] == "linux";
        let mut tags = format!(
            r#"<span class="emu-file-tag {}">{}</span><span class="emu-file-tag">{}</span>"#,
            if linux { "emu-file-tag--linux" } else { "emu-file-tag--windows" },
            escape_html(app.i18n.t(if linux { "emulator.platformLinux" } else { "emulator.platformWindows" })),
            escape_html(app.i18n.t(if file["arch"] == "x64" { "emulator.archX64" } else { "emulator.archX32" })),
        );
        if loader.is_some() {
            tags.push_str(&format!(r#"<span class="emu-file-tag">{}</span>"#, escape_html(app.i18n.t("emulator.method.coldclient"))));
        }
        if app.state.emu.patched_paths.contains(&path) {
            tags.push_str(&format!(r#"<span class="emu-file-tag emu-file-tag--patched">{}</span>"#, escape_html(app.i18n.t("emulator.patchedBadge"))));
        }
        let markup = format!(r#"<span class="emu-file-item__path" title="{}">{}</span><span class="emu-file-item__tags">{tags}</span>"#, escape_html(&path), escape_html(&shown));
        let checked = app.state.emu.selected_files.contains(&path);
        let clicked = dom
            .button(el("label.emu-file-item").state(HAS_CHECKED, checked).key(&path), |dom| {
                dom.empty(el("input.emu-file-item__check[type=checkbox]").state(CHECKED, checked));
                dom.markup(&markup);
            })
            .clicked();
        if clicked {
            toggled = Some(path);
        }
    }
    if let Some(path) = toggled {
        app.state.emu.selection_explicit = true;
        if !app.state.emu.selected_files.remove(&path) {
            app.state.emu.selected_files.insert(path);
        }
        update_emu_action_buttons(app);
    }
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    let markup: String = match id {
        "emu-file-list" => {
            fill_file_list(app, dom);
            return true;
        }
        "emu-drm-list" => app
            .state
            .emu
            .drm_shown
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let path = str_of(target, "path");
                format!(
                    r#"<div class="emu-drm-item{}"><span class="emu-drm-item__path" title="{}">{}</span><span class="emu-drm-item__size">{}</span></div>"#,
                    if app.state.emu.drm_success.contains(&index) { " emu-drm-item--success" } else { "" },
                    escape_html(&path),
                    escape_html(&relativize_emu_path(app, &path)),
                    escape_html(&target["size_bytes"].as_f64().and_then(format_bytes).unwrap_or_default()),
                )
            })
            .collect(),
        "emu-drm-restore-list" => app
            .state
            .emu
            .drm_backups
            .iter()
            .map(|backup| {
                let path = str_of(backup, "path");
                format!(
                    r#"<div class="emu-drm-restore__row"><span class="emu-drm-restore__path">{}</span><button class="btn btn--small btn--outline" data-click="drm-restore:{}">{}</button></div>"#,
                    escape_html(&relativize_emu_path(app, &path)),
                    escape_html(&path),
                    escape_html(app.i18n.t("emulator.drmRestore.restore")),
                )
            })
            .collect(),
        _ => return fields::fill(app, id, dom) || extras::fill(app, id, dom),
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}

pub fn frame(app: &mut App, _ctx: &egui::Context) {
    if app.state.current_step == 5 {
        update_emu_overview(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_paths_and_download_requests() {
        assert_eq!(extract_depot_folder("C:\\dl\\depots\\481\\bin\\steam_api.dll"), "481");
        assert_eq!(extract_depot_folder("/dl/game/steam_api.dll"), "");
        assert_eq!(emu_file_label("/a/b/c/steam_api64.dll"), "c/steam_api64.dll");
        assert_eq!(emu_settings_dir_of("/a/b/steam_api.dll"), "/a/b");
        let info = parse_emu_download_confirm(r#"EMU_DOWNLOAD_CONFIRM_REQUIRED:{"tag":"v1","platforms":[{"platform":"windows","size":10}]}"#).expect("confirm");
        assert_eq!(info["tag"], "v1");
        assert!(parse_emu_download_confirm("other error").is_none());
        let files = vec![json!({ "path": "/d/depots/1/a.dll", "platform": "windows", "arch": "x64" }), json!({ "path": "/d/depots/2/b.so", "platform": "linux", "arch": "x64" })];
        assert_eq!(pick_default_emu_depot(&files).as_deref(), Some(if cfg!(target_os = "linux") { "2" } else { "1" }));
    }
}
