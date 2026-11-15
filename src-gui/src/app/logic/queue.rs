use serde_json::{json, Value};

use super::util::{escape_html, format_bytes, icon, opt_str, str_of, ICON_TRASH};
use super::{account, download_control, emulator, history, progress, select, telemetry, ui};
use crate::app::state::{Depot, ParsedData};
use crate::app::App;
use crate::dom::Dom;
use crate::t;

const QUEUE_NEXT_DELAY_SECONDS: u64 = 5;

fn update_queue_badge(app: &mut App, queue: Option<Vec<Value>>) {
    if let Some(queue) = queue {
        app.state.queue = queue;
    }
    let count = app.state.queue.len();
    app.ui.set_text("queue-badge", count.to_string());
    app.ui.set_hidden("queue-badge", count == 0);
    let label = t!(app, "queue.title");
    app.ui.set_attr("btn-queue", "title", if count > 0 { format!("{label} ({count})") } else { label });
}

pub fn refresh_queue(app: &mut App) {
    refresh_queue_then(app, |_| {});
}

fn refresh_queue_then(app: &mut App, then: impl FnOnce(&mut App) + Send + 'static) {
    app.tasks.call("queue_list", Value::Null, move |app, queue: Result<Vec<Value>, String>| {
        update_queue_badge(app, queue.ok());
        if !app.ui.hidden("queue-modal") {
            render_queue(app);
        }
        then(app);
    });
}

pub fn add_to_queue(app: &mut App) {
    let Some(data) = app.state.parsed_data.clone() else { return };
    let selected: Vec<Depot> = data.depots.iter().filter(|d| app.state.selected_depots.contains(&d.depot_id)).cloned().collect();
    if selected.is_empty() {
        return;
    }
    if let Some(problem) = account::branch_start_problem(app) {
        let text = t!(app, problem);
        return ui::show_toast(app, text, "error", 6000);
    }
    if account::account_download_fields(app)["branchPassword"].as_str().is_some_and(|password| !password.is_empty()) {
        let text = t!(app, "branch.noQueueWithPassword");
        return ui::show_toast(app, text, "error", 6000);
    }
    let size: u64 = selected.iter().filter_map(|d| d.size_bytes).sum();
    let updating = progress::active_update_dir(app, &data.main_app_id).is_some();
    download_control::confirm_free_space(app, size, updating, move |app, ok| {
        if !ok {
            return;
        }
        let key = select::remember_download_inputs(app);
        let repair = progress::active_repair(app, &data.main_app_id);
        let depots = select::collect_depot_configs(app, &selected, repair.as_ref());
        let item = json!({
            "id": "",
            "app_id": data.main_app_id,
            "game_name": app.state.game_name,
            "header_image": app.state.header_image,
            "depot_count": depots.len(),
            "size_bytes": (size > 0).then_some(size),
            "depots": depots,
            "config": select::build_download_config(app, &data, &depots, &key, repair.as_ref()),
            "added_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        });
        let name = opt_str(&item, "game_name").unwrap_or_else(|| format!("App {}", data.main_app_id));
        app.tasks.call("queue_add", json!({ "item": item }), move |app, queue: Result<Vec<Value>, String>| match queue {
            Ok(queue) => {
                update_queue_badge(app, Some(queue));
                app.emit_event("queue_action", json!({ "action": "added", "size": telemetry::count_bucket(app.state.queue.len()) }));
                let text = t!(app, "queue.added", name = name, count = app.state.queue.len());
                ui::show_toast(app, text, "success", 4000);
                progress::reset_app(app);
            }
            Err(error) => {
                let text = app.i18n.localize_error(&error);
                ui::show_toast(app, text, "error", 6000);
            }
        });
    });
}

fn render_queue(app: &mut App) {
    let current = app.state.queue_current.clone().filter(|_| app.state.queue_running);
    app.ui.set_hidden("queue-running", current.is_none());
    if let Some(current) = current {
        let name = opt_str(&current, "game_name").unwrap_or_else(|| format!("App {}", str_of(&current, "app_id")));
        let text = t!(app, "queue.running", name = name);
        app.ui.set_text("queue-running", text);
    }
    let busy = app.state.job_id.is_some() || app.state.queue_running;
    let empty = app.state.queue.is_empty();
    app.ui.set_disabled("btn-queue-start", (empty || busy) && !app.state.queue_running);
    let label = t!(app, if app.state.queue_running { "queue.stop" } else { "queue.start" });
    app.ui.set_text("btn-queue-start", label);
    app.ui.set_disabled("btn-queue-clear", empty);
}

pub fn open_queue(app: &mut App) {
    refresh_queue_then(app, |app| {
        render_queue(app);
        app.ui.show("queue-modal");
    });
}

pub fn close_queue(app: &mut App) {
    app.ui.hide("queue-modal");
}

fn start_queue(app: &mut App) {
    if app.state.job_id.is_some() || app.state.queue_running {
        return;
    }
    app.state.queue_running = true;
    app.emit_event("queue_action", json!({ "action": "started", "size": telemetry::count_bucket(app.state.queue.len()) }));
    close_queue(app);
    run_next_queued(app);
}

fn stop_queue(app: &mut App) {
    if app.state.queue_running {
        app.emit_event("queue_action", json!({ "action": "stopped", "size": telemetry::count_bucket(app.state.queue.len()) }));
    }
    app.state.queue_running = false;
    app.state.queue_timer = None;
    app.state.queue_current = None;
    let text = t!(app, "queue.stopped");
    ui::show_toast(app, text, "info", 4000);
    render_queue(app);
}

fn schedule_next(app: &mut App, millis: u64) {
    app.state.timer_serial += 1;
    let token = app.state.timer_serial;
    app.state.queue_timer = Some(token);
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        tasks.sleep(millis).await;
        tasks.post(move |app| {
            if app.state.queue_timer == Some(token) {
                run_next_queued(app);
            }
        });
    });
}

fn depot_of(config: &Value) -> Depot {
    Depot {
        depot_id: str_of(config, "depotId"),
        manifest_id: opt_str(config, "manifestId").unwrap_or_else(|| "N/A".to_string()),
        depot_key: opt_str(config, "depotKey"),
        size_bytes: config["sizeBytes"].as_u64().filter(|size| *size > 0),
    }
}

fn run_next_queued(app: &mut App) {
    app.state.queue_timer = None;
    if !app.state.queue_running {
        return;
    }
    if app.state.job_id.is_some() {
        return schedule_next(app, 2000);
    }
    app.tasks.call("queue_take_next", Value::Null, |app, next: Result<Value, String>| {
        let next = next.ok().filter(|next| !next.is_null());
        refresh_queue_then(app, move |app| {
            let Some(mut next) = next else {
                app.state.queue_running = false;
                app.state.queue_current = None;
                return;
            };
            app.state.queue_current = Some(next.clone());
            history::close_history(app);
            app.state.emu_standalone = false;
            emulator::set_emu_edit_mode(app, false);
            progress::cleanup_progress_listener(app);
            app.state.job_id = None;
            let depots: Vec<Value> = next["depots"].as_array().cloned().unwrap_or_default();
            let app_id = str_of(&next, "app_id");
            app.state.set_parsed(Some(ParsedData { main_app_id: app_id.clone(), depots: depots.iter().map(depot_of).collect(), all_app_ids: Vec::new() }));
            app.state.selected_depots = depots.iter().map(|d| str_of(d, "depotId")).collect();
            app.state.game_name = opt_str(&next, "game_name");
            app.state.header_image = opt_str(&next, "header_image");
            app.state.update_dir = opt_str(&next["config"], "updateDir");
            app.state.update_app_id = app.state.update_dir.as_ref().map(|_| app_id);
            app.state.repair_manifests = None;
            app.state.repair_running = next["config"]["repair"].as_bool().unwrap_or(false);
            if opt_str(&next["config"], "manifestHubApiKey").is_none() {
                next["config"]["manifestHubApiKey"] = json!(app.store.get(select::MH_APIKEY_STORAGE_KEY).filter(|key| !key.is_empty()));
            }
            select::run_download(app, next["config"].clone(), depots, true);
        });
    });
}

pub fn queue_after_download(app: &mut App, outcome: &'static str) {
    if !app.state.queue_running {
        return;
    }
    if outcome == "cancelled" {
        return stop_queue(app);
    }
    refresh_queue_then(app, move |app| {
        if app.state.queue.is_empty() {
            app.state.queue_running = false;
            app.state.queue_current = None;
            let text = t!(app, "queue.finished");
            ui::show_toast(app, text, "success", 6000);
            return download_control::maybe_schedule_shutdown(app);
        }
        progress::commit_pending_history(app);
        if outcome != "failed" {
            download_control::save_pending_followup(app);
        }
        let line = t!(app, "queue.nextIn", seconds = QUEUE_NEXT_DELAY_SECONDS);
        progress::append_terminal_line(app, line, "info");
        schedule_next(app, QUEUE_NEXT_DELAY_SECONDS * 1000);
    });
}

fn act(app: &mut App, command: &'static str, args: Value) {
    app.tasks.call(command, args, |app, queue: Result<Vec<Value>, String>| {
        update_queue_badge(app, queue.ok());
        render_queue(app);
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-queue-add" => add_to_queue(app),
        "btn-queue" => open_queue(app),
        "btn-queue-close" | "queue-modal>modal__backdrop" => close_queue(app),
        "btn-queue-start" => {
            if app.state.queue_running {
                stop_queue(app);
            } else {
                start_queue(app);
            }
        }
        "btn-queue-clear" => {
            let size = telemetry::count_bucket(app.state.queue.len());
            app.tasks.call("queue_clear", Value::Null, move |app, result: Result<Value, String>| {
                if result.is_ok() {
                    app.emit_event("queue_action", json!({ "action": "cleared", "size": size }));
                }
                refresh_queue(app);
            });
        }
        _ => {
            if let Some(item) = id.strip_prefix("queue-up:") {
                act(app, "queue_move", json!({ "id": item, "offset": -1 }));
            } else if let Some(item) = id.strip_prefix("queue-down:") {
                act(app, "queue_move", json!({ "id": item, "offset": 1 }));
            } else if let Some(item) = id.strip_prefix("queue-remove:") {
                act(app, "queue_remove", json!({ "id": item }));
            } else {
                return false;
            }
        }
    }
    true
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    if id != "queue-list" {
        return false;
    }
    let queue = &app.state.queue;
    let markup: String = if queue.is_empty() {
        format!(r#"<div class="history-empty">{}</div>"#, escape_html(app.i18n.t("queue.empty")))
    } else {
        let (up, down, remove) = (escape_html(app.i18n.t("queue.up")), escape_html(app.i18n.t("queue.down")), escape_html(app.i18n.t("queue.remove")));
        queue
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let item_id = escape_html(&str_of(item, "id"));
                let app_id = str_of(item, "app_id");
                let name = opt_str(item, "game_name").unwrap_or_else(|| format!("App {app_id}"));
                let size = item["size_bytes"].as_f64().and_then(format_bytes).map(|size| format!(" · {}", escape_html(&size))).unwrap_or_default();
                let kind = if opt_str(&item["config"], "updateDir").is_some() {
                    format!(" · {}", escape_html(app.i18n.t(if item["config"]["repair"].as_bool().unwrap_or(false) { "queue.kindRepair" } else { "queue.kindUpdate" })))
                } else {
                    String::new()
                };
                format!(
                    r#"<div class="queue-item" data-key="{item_id}"><span class="queue-item__pos">{pos}</span><div class="queue-item__info"><div class="queue-item__name">{name}</div><div class="queue-item__meta">App {app_id} · {depots}{size}{kind}</div></div><div class="queue-item__actions"><button class="btn btn--small btn--outline queue-up" title="{up}" data-click="queue-up:{item_id}"{first}>↑</button><button class="btn btn--small btn--outline queue-down" title="{down}" data-click="queue-down:{item_id}"{last}>↓</button><button class="btn btn--small btn--outline queue-remove" title="{remove}" data-click="queue-remove:{item_id}">{trash}</button></div></div>"#,
                    pos = index + 1,
                    name = escape_html(&name),
                    app_id = escape_html(&app_id),
                    depots = escape_html(&t!(app, "queue.depots", count = str_of(item, "depot_count"))),
                    first = if index == 0 { " disabled" } else { "" },
                    last = if index + 1 == queue.len() { " disabled" } else { "" },
                    trash = icon(ICON_TRASH),
                )
            })
            .collect()
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}
