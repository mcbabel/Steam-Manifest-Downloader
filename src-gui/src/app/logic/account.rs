use serde_json::{json, Value};

use super::picker::{self, Outcome, Picker};
use super::util::{escape_html, opt_str, str_of};
use super::{search, select, settings, ui};
use crate::app::state::{Depot, ParsedData, Repo};
use crate::app::App;
use crate::dom::{el, Dom};
use crate::t;

const BRANCH_PICKER: Picker = Picker { picker: "branch-picker", button: "branch-button", menu: "branch-menu" };

#[derive(Default)]
pub struct AccountUi {
    pub info: Option<Value>,
    pub poll_timer: Option<u64>,
    pub polling: bool,
    pub interval: u64,
    pub branches: Option<Value>,
    pub branch: String,
    pub branch_app_id: Option<String>,
    pub qr: Option<String>,
}

pub fn signed_in(app: &App) -> bool {
    app.state.account.info.is_some()
}

fn account_storage_text(app: &App, info: &Value) -> String {
    match info["storage"].as_str() {
        Some("keychain") => app.i18n.t("account.storedKeychain").to_string(),
        Some("file") => app.i18n.t("account.storedFile").to_string(),
        _ => app.i18n.t("account.storedSession").to_string(),
    }
}

pub fn load_account_status(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    app.tasks.call("account_status", Value::Null, move |app, info: Result<Value, String>| {
        app.state.account.info = info.ok().filter(|info| !info.is_null());
        render_account_settings(app);
        then(app);
    });
}

pub fn render_account_settings(app: &mut App) {
    match app.state.account.info.clone() {
        Some(info) => {
            let status = t!(app, "account.signedInAs", name = str_of(&info, "accountName"));
            app.ui.set_text("account-status", status);
            let detail = account_storage_text(app, &info);
            app.ui.set_text("account-detail", detail);
            app.ui.hide("btn-account-login");
            app.ui.show("btn-account-logout");
        }
        None => {
            let (status, detail) = (t!(app, "account.signedOut"), t!(app, "account.signedOutHint"));
            app.ui.set_text("account-status", status);
            app.ui.set_text("account-detail", detail);
            app.ui.show("btn-account-login");
            app.ui.hide("btn-account-logout");
        }
    }
}

fn set_account_qr(app: &mut App, qr: Option<&Value>) {
    let svg = qr.and_then(|qr| qr["svg"].as_str()).filter(|svg| !svg.is_empty());
    app.state.account.qr = svg.map(|svg| app.register_svg(svg));
    app.ui.set_class("account-qr", "account-qr--empty", app.state.account.qr.is_none());
}

fn set_account_modal_status(app: &mut App, text: String, tone: &str) {
    let classes: &[&'static str] = match tone {
        "warning" => &["account-modal__status", "account-modal__status--warning"],
        "error" => &["account-modal__status", "account-modal__status--error"],
        _ => &["account-modal__status"],
    };
    app.ui.set_classes("account-modal-status", classes);
    app.ui.set_text("account-modal-status", text);
}

fn stop_account_polling(app: &mut App) {
    app.state.account.poll_timer = None;
    app.state.account.polling = false;
}

fn schedule_account_poll(app: &mut App, seconds: u64) {
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.account.poll_timer = Some(token);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(seconds.max(1) * 1000).await;
        tasks.post(move |app| {
            if app.state.account.poll_timer == Some(token) {
                poll_account_login(app);
            }
        });
    });
}

fn login_failed(app: &mut App, error: &str, reason: &str) {
    let text = app.i18n.localize_error(error);
    set_account_modal_status(app, text, "error");
    app.ui.show("btn-account-retry");
    app.emit_event("account_login", json!({ "step": "failed", "reason": reason }));
}

pub fn start_account_login(app: &mut App) {
    stop_account_polling(app);
    set_account_qr(app, None);
    app.ui.hide("btn-account-retry");
    let loading = t!(app, "account.qrLoading");
    set_account_modal_status(app, loading, "");
    app.ui.show("account-modal");
    app.emit_event("account_login", json!({ "step": "start" }));
    app.tasks.call("account_login_start", Value::Null, |app, qr: Result<Value, String>| match qr {
        Ok(qr) => {
            set_account_qr(app, Some(&qr));
            let waiting = t!(app, "account.qrWaiting");
            set_account_modal_status(app, waiting, "");
            app.state.account.polling = true;
            app.state.account.interval = qr["intervalSecs"].as_u64().filter(|secs| *secs > 0).unwrap_or(5);
            let interval = app.state.account.interval;
            schedule_account_poll(app, interval);
        }
        Err(error) => login_failed(app, &error, "start"),
    });
}

fn poll_account_login(app: &mut App) {
    if !app.state.account.polling {
        return;
    }
    app.tasks.call("account_login_poll", Value::Null, |app, result: Result<Value, String>| {
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                stop_account_polling(app);
                return login_failed(app, &error, "poll");
            }
        };
        if !app.state.account.polling {
            return;
        }
        match result["state"].as_str() {
            Some("waiting") => {
                if !result["qr"].is_null() {
                    set_account_qr(app, Some(&result["qr"]));
                    if let Some(interval) = result["qr"]["intervalSecs"].as_u64().filter(|secs| *secs > 0) {
                        app.state.account.interval = interval;
                    }
                }
                let text = t!(app, if result["scanned"].as_bool().unwrap_or(false) { "account.qrScanned" } else { "account.qrWaiting" });
                set_account_modal_status(app, text, "");
                let interval = app.state.account.interval;
                schedule_account_poll(app, interval);
            }
            Some("done") => {
                stop_account_polling(app);
                let account = result["account"].clone();
                app.state.account.info = Some(account.clone());
                render_account_settings(app);
                close_account_modal(app, false);
                let toast = t!(app, "account.signedInToast", name = str_of(&account, "accountName"));
                ui::show_toast(app, toast, "success", 6000);
                app.emit_event("account_login", json!({ "step": "done", "storage": account["storage"] }));
                refresh_branch_section(app);
            }
            Some("expired") => {
                stop_account_polling(app);
                set_account_qr(app, None);
                let text = t!(app, "account.qrExpired");
                set_account_modal_status(app, text, "warning");
                app.ui.show("btn-account-retry");
                app.emit_event("account_login", json!({ "step": "expired" }));
            }
            _ => stop_account_polling(app),
        }
    });
}

fn close_account_modal(app: &mut App, cancel: bool) {
    app.ui.hide("account-modal");
    if cancel && app.state.account.polling {
        app.tasks.fire("account_login_cancel", Value::Null);
        app.emit_event("account_login", json!({ "step": "cancelled" }));
    }
    stop_account_polling(app);
}

fn account_logout(app: &mut App) {
    let name = app.state.account.info.as_ref().map(|info| str_of(info, "accountName")).unwrap_or_default();
    app.tasks.call("account_logout", Value::Null, move |app, result: Result<Value, String>| {
        if let Err(error) = result {
            let text = app.i18n.localize_error(&error);
            return ui::show_toast(app, text, "error", 4000);
        }
        app.state.account.info = None;
        render_account_settings(app);
        refresh_branch_section(app);
        app.emit_event("account_logout", json!({}));
        let toast = t!(app, "account.signedOutToast", name = name);
        ui::show_toast(app, toast, "success", 4000);
    });
}

fn selected_branch(app: &App) -> &str {
    if app.state.account.branch.is_empty() {
        "public"
    } else {
        &app.state.account.branch
    }
}

fn branch_list(app: &App) -> Vec<Value> {
    let list = app.state.account.branches.as_ref().and_then(|b| b["branches"].as_array()).cloned().unwrap_or_default();
    if list.is_empty() {
        vec![json!({ "name": "public", "description": "", "passwordRequired": false, "available": true })]
    } else {
        list
    }
}

fn selected_branch_info(app: &App) -> Option<Value> {
    let name = selected_branch(app);
    app.state.account.branches.as_ref().and_then(|b| b["branches"].as_array()).and_then(|list| list.iter().find(|b| b["name"] == name)).cloned()
}

fn is_public_branch(name: &str) -> bool {
    name.is_empty() || name.eq_ignore_ascii_case("public")
}

pub fn account_download_fields(app: &App) -> Value {
    let branch = selected_branch(app);
    let use_account = app.state.account_mode || !is_public_branch(branch) || (signed_in(app) && app.ui.checked("use-account-toggle"));
    let mut fields = json!({ "useAccount": use_account });
    if !is_public_branch(branch) {
        fields["branch"] = json!(branch);
        let password = app.ui.value("branch-password").trim();
        if !password.is_empty() {
            fields["branchPassword"] = json!(password);
        }
    }
    fields
}

pub fn branch_start_problem(app: &App) -> Option<&'static str> {
    if is_public_branch(selected_branch(app)) {
        return None;
    }
    if !signed_in(app) {
        return Some("branch.needsLogin");
    }
    let info = selected_branch_info(app)?;
    let locked = info["passwordRequired"].as_bool().unwrap_or(false) && !info["available"].as_bool().unwrap_or(false);
    (locked && app.ui.value("branch-password").trim().is_empty()).then_some("branch.needsPassword")
}

fn branch_tag(app: &App, branch: &Value) -> String {
    if branch["passwordRequired"].as_bool().unwrap_or(false) {
        app.i18n.t("branch.passwordTag").to_string()
    } else if !branch["available"].as_bool().unwrap_or(false) {
        app.i18n.t("branch.lockedTag").to_string()
    } else {
        String::new()
    }
}

fn branch_item_html(app: &App, branch: &Value) -> String {
    let tag = branch_tag(app, branch);
    let description = str_of(branch, "description");
    format!(
        r#"<span class="branch-picker__name">{}</span>{}{}"#,
        escape_html(&str_of(branch, "name")),
        if description.is_empty() { String::new() } else { format!(r#"<span class="branch-picker__desc">{}</span>"#, escape_html(&description)) },
        if tag.is_empty() { String::new() } else { format!(r#"<span class="depot-tag branch-picker__tag">{}</span>"#, escape_html(&tag)) },
    )
}

fn render_branch_section(app: &mut App) {
    let names = branch_list(app);
    if !names.iter().any(|b| b["name"] == app.state.account.branch.as_str()) {
        app.state.account.branch = "public".to_string();
    }
    let branch = app.state.account.branch.clone();
    set_branch(app, &branch);
}

fn set_branch(app: &mut App, name: &str) {
    app.state.account.branch = if name.is_empty() { "public".to_string() } else { name.to_string() };
    update_branch_controls(app);
}

fn active_index(app: &App) -> Option<usize> {
    branch_list(app).iter().position(|b| b["name"] == selected_branch(app))
}

fn choose_branch(app: &mut App, name: &str) {
    set_branch(app, name);
    picker::set_open(app, BRANCH_PICKER, false, None);
    app.ui.request_focus("branch-button");
}

fn update_branch_controls(app: &mut App) {
    let branch = selected_branch(app).to_string();
    let info = selected_branch_info(app);
    let signed = signed_in(app);
    let non_public = !is_public_branch(&branch);
    let others = branch_list(app).len() - 1;
    app.ui.set_hidden("branch-section", !signed && others < 1);
    app.ui.set_hidden("branch-label", others < 1 && !non_public);
    app.ui.set_hidden("branch-row", !signed || (others < 1 && !non_public));
    app.ui.set_hidden("branch-login-row", signed || others < 1);
    let more = t!(app, "branch.moreBranches", count = others);
    app.ui.set_text("branch-more", more);
    let needs_password = non_public && info.as_ref().is_some_and(|info| info["passwordRequired"].as_bool().unwrap_or(false) && !info["available"].as_bool().unwrap_or(false));
    app.ui.set_hidden("branch-password", !needs_password);
    app.ui.set_hidden("use-account-row", !signed);
    app.ui.set_disabled("use-account-toggle", non_public || app.state.account_mode);
    if non_public || app.state.account_mode {
        app.ui.set_checked("use-account-toggle", true);
    }
    let hint = if signed && non_public { t!(app, "branch.hintBranch") } else { String::new() };
    app.ui.set_hidden("branch-hint", hint.is_empty());
    app.ui.set_text("branch-hint", hint);
}

pub fn refresh_branch_section(app: &mut App) {
    let Some(app_id) = app.state.parsed_data.as_ref().map(|d| d.main_app_id.clone()).filter(|id| !id.is_empty()) else { return };
    app.state.account.branch_app_id = Some(app_id.clone());
    let list = move |app: &mut App| {
        let asked = app_id.clone();
        app.tasks.call("list_branches", json!({ "appId": app_id }), move |app, list: Result<Value, String>| {
            if let Ok(list) = &list {
                if app.state.account.branch_app_id.as_deref() != Some(asked.as_str()) {
                    return;
                }
                app.state.account.branches = Some(list.clone());
            } else {
                app.state.account.branches = None;
            }
            render_branch_section(app);
        });
    };
    if signed_in(app) {
        list(app);
    } else {
        load_account_status(app, list);
    }
}

pub fn init_branch_section(app: &mut App) {
    app.state.account.branches = None;
    app.state.account.branch = "public".to_string();
    picker::set_open(app, BRANCH_PICKER, false, None);
    app.ui.set_value("branch-password", "");
    let account_mode = app.state.account_mode;
    app.ui.set_checked("use-account-toggle", account_mode);
    render_branch_section(app);
    refresh_branch_section(app);
}

fn load_from_account(app: &mut App) {
    let Some(app_id) = app.state.search_app_id else { return };
    if !signed_in(app) {
        return settings::open_settings(app, Some("steam"));
    }
    app.ui.hide("search-error");
    app.ui.show("manifest-loading");
    let args = json!({ "appId": app_id.to_string(), "branch": null, "branchPassword": null });
    app.tasks.call("account_app_depots", args, move |app, depots: Result<Vec<Value>, String>| {
        app.ui.hide("manifest-loading");
        let depots = match depots {
            Ok(depots) => depots,
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                return search::show_search_error(app, text);
            }
        };
        if depots.is_empty() {
            let text = t!(app, "account.noDepots");
            return search::show_search_error(app, text);
        }
        app.state.account_mode = true;
        app.state.search_repo = None;
        app.state.search_sha = None;
        app.state.search_key_vdf_keys = None;
        app.state.search_repos = vec![Repo { kind: "account".to_string(), ..Default::default() }];
        app.state.selected_repo = Some(0);
        let count = depots.len();
        let depots = depots.iter().map(|d| Depot { depot_id: str_of(d, "depotId"), manifest_id: opt_str(d, "manifestId").unwrap_or_else(|| "N/A".to_string()), depot_key: None, size_bytes: None }).collect();
        app.state.set_parsed(Some(ParsedData { main_app_id: app_id.to_string(), depots, all_app_ids: Vec::new() }));
        app.emit_event("account_depots", json!({ "count": count }));
        select::show_selection_step(app);
    });
}

pub fn init_account(app: &mut App) {
    app.state.account.interval = 5;
    app.state.account.branch = "public".to_string();
    load_account_status(app, |_| {});
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-account-login" | "btn-account-retry" => start_account_login(app),
        "btn-account-logout" => account_logout(app),
        "btn-account-cancel" | "account-modal>modal__backdrop" => close_account_modal(app, true),
        "branch-button" => {
            let open = !picker::is_open(app, BRANCH_PICKER);
            let active = active_index(app);
            picker::set_open(app, BRANCH_PICKER, open, active);
        }
        "btn-branch-login" => settings::open_settings(app, Some("steam")),
        "btn-search-account" => load_from_account(app),
        _ => match id.strip_prefix("branch:") {
            Some(name) => choose_branch(app, name),
            None => return false,
        },
    }
    true
}

pub fn changed(_app: &mut App, id: &str) -> bool {
    id == "use-account-toggle"
}

pub fn input(_app: &mut App, id: &str) -> bool {
    id == "branch-password"
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "branch-button-label" => {
            if let Some(current) = branch_list(app).into_iter().find(|b| b["name"] == selected_branch(app)) {
                dom.markup(branch_item_html(app, &current));
            }
        }
        "branch-menu" => {
            let focused = picker::focused(app, BRANCH_PICKER);
            let markup: String = branch_list(app)
                .iter()
                .enumerate()
                .map(|(index, branch)| {
                    let name = escape_html(&str_of(branch, "name"));
                    let active = if branch["name"] == selected_branch(app) { " is-active" } else { "" };
                    let focus = if focused == Some(index) { " is-focused" } else { "" };
                    format!(r#"<li class="history-sort__option{active}{focus}" data-click="branch:{name}">{}</li>"#, branch_item_html(app, branch))
                })
                .collect();
            if let Some(action) = dom.markup(markup) {
                app.click(&action);
            }
        }
        "account-qr" => {
            if let Some(uri) = app.state.account.qr.clone() {
                dom.image(el("svg"), &uri, false);
            }
        }
        _ => return false,
    }
    true
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    let count = branch_list(app).len();
    let active = active_index(app);
    if let Outcome::Choose(index) = picker::frame(app, ctx, BRANCH_PICKER, count, active) {
        if let Some(name) = branch_list(app).get(index).map(|b| str_of(b, "name")) {
            choose_branch(app, &name);
        }
    }
}
