use serde_json::{json, Value};

use super::util::{date_time_medium_short, escape_html, opt_str, str_of};
use super::{account, select, settings, steps, telemetry, ui};
use crate::app::state::{Depot, ParsedData, Repo};
use crate::app::App;
use crate::dom::Dom;
use crate::t;

pub fn perform_search(app: &mut App) {
    steps::hide_autocomplete(app, "search-autocomplete");
    let text = app.ui.value("search-appid-input").trim().to_string();
    if text.is_empty() {
        return;
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    let app_id = match digits.parse::<u64>() {
        Ok(value) if value > 0 => value,
        _ => {
            let message = t!(app, "search.invalidAppId");
            return show_search_error(app, message);
        }
    };
    app.ui.hide("search-error");
    ui::hide_next_steps(app, "search-next-steps");
    app.ui.hide("search-results");
    app.ui.hide("search-next-row");
    app.ui.hide("search-game-banner");
    app.state.selected_repo = None;
    app.state.search_repos.clear();
    app.state.search_app_id = Some(app_id);
    app.state.account_mode = false;
    app.ui.hide("search-account-row");
    hide_playtests(app);
    app.ui.show("search-loading");
    app.ui.set_disabled("btn-search", true);
    fetch_search_game_info(app, app_id);
    show_playtests(app, app_id);
    app.tasks.call("search_repos", json!({ "appId": app_id.to_string() }), move |app, raw: Result<Value, String>| {
        app.ui.hide("search-loading");
        app.ui.set_disabled("btn-search", false);
        match raw {
            Ok(raw) => {
                let repos: Vec<Repo> = raw["repos"]
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .map(|r| {
                                let kind = opt_str(r, "type").unwrap_or_else(|| "unknown".to_string());
                                Repo { name: str_of(r, "repo"), date: opt_str(r, "date"), sha: opt_str(r, "sha"), source: opt_str(r, "source").unwrap_or_else(|| kind.clone()), kind }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let count = repos.len();
                app.state.search_repos = repos;
                emit_search_outcome(app, count, raw["sourceProbe"].as_str());
                if account::signed_in(app) {
                    app.ui.show("search-account-row");
                }
                if count == 0 {
                    show_search_no_results(app, app_id);
                    telemetry::report_error(app, "search", None, Some("search.noResults"));
                    return;
                }
                render_repo_list(app);
                app.ui.show("search-results");
            }
            Err(error) => {
                emit_search_outcome(app, 0, Some("error"));
                let message = app.i18n.localize_error(&error);
                show_search_error(app, message);
                telemetry::report_error(app, "search", Some(&error), None);
            }
        }
    });
}

fn emit_search_outcome(app: &mut App, repo_count: usize, probe: Option<&str>) {
    let mut parts = probe.unwrap_or("").split(':');
    let source = parts.next().filter(|p| !p.is_empty());
    let class = parts.next().filter(|p| !p.is_empty());
    app.emit_event("search_performed", json!({ "found": repo_count > 0, "repo_count": repo_count, "source_probe": source, "probe_class": class }));
}

fn show_search_no_results(app: &mut App, app_id: u64) {
    let title = t!(app, "search.noResultsTitle", appId = app_id);
    let text = t!(app, "search.noResultsText");
    let actions = vec![
        (
            t!(app, "search.manageSources"),
            ui::action(|app| {
                app.emit_event("advice_action", json!({ "advice": "search_no_results", "action": "sources" }));
                settings::open_settings(app, Some("sources"));
            }),
        ),
        (
            t!(app, "search.useUpload"),
            ui::action(|app| {
                app.emit_event("advice_action", json!({ "advice": "search_no_results", "action": "upload" }));
                steps::switch_tab(app, "upload");
            }),
        ),
    ];
    ui::render_next_steps(app, "search-next-steps", title, text, false, actions);
    app.ui.show("search-next-steps");
    app.emit_event("advice_shown", json!({ "advice": "search_no_results" }));
}

pub fn show_search_error(app: &mut App, message: String) {
    app.ui.set_text("search-error", message);
    app.ui.show("search-error");
}

fn hide_playtests(app: &mut App) {
    app.ui.hide("search-playtest-row");
    app.state.playtests.clear();
}

fn show_playtests(app: &mut App, app_id: u64) {
    app.tasks.call("find_playtests", json!({ "appId": app_id.to_string() }), move |app, found: Result<Vec<Value>, String>| {
        let found = found.unwrap_or_default();
        if app.state.search_app_id != Some(app_id) || found.is_empty() {
            return;
        }
        app.state.playtests = found.into_iter().take(3).collect();
        app.ui.show("search-playtest-row");
    });
}

fn fetch_search_game_info(app: &mut App, app_id: u64) {
    app.ui.hide("search-game-banner");
    app.tasks.call("get_steam_app_info", json!({ "appId": app_id.to_string() }), move |app, info: Result<Value, String>| {
        let Ok(info) = info else { return };
        if info.is_null() {
            return;
        }
        app.state.game_denuvo = info["denuvo"].as_bool().unwrap_or(false);
        if let Some(image) = opt_str(&info, "headerImage") {
            app.ui.set_attr("search-game-image", "src", image.clone());
            app.state.header_image = Some(image);
        }
        if let Some(name) = opt_str(&info, "name") {
            app.ui.set_text("search-game-name", name.clone());
            app.state.game_name = Some(name);
        }
        if let Some(description) = opt_str(&info, "shortDescription") {
            app.ui.set_text("search-game-description", description);
        }
        app.ui.show("search-game-banner");
    });
}

fn render_repo_list(app: &mut App) {
    let count = app.state.search_repos.len();
    if count == 1 {
        select_repo(app, 0);
    }
    if app.state.auto_redownload_pending && count > 0 {
        app.state.auto_redownload_pending = false;
        select_repo(app, 0);
        proceed_from_search(app);
    }
}

fn repo_display_name(app: &App, repo: &Repo) -> String {
    match repo.kind.as_str() {
        "hubcap" => app.i18n.t("search.repoNameHubcap").to_string(),
        "remote" => app.i18n.t("search.repoNameRemote").to_string(),
        _ if !repo.name.is_empty() => repo.name.clone(),
        _ if !repo.kind.is_empty() => repo.kind.clone(),
        _ => "Unknown".to_string(),
    }
}

fn repo_badge_text(app: &App, repo: &Repo) -> String {
    match repo.kind.as_str() {
        "hubcap" => app.i18n.t("search.repoBadgeHubcap").to_string(),
        "remote" => app.i18n.t("search.repoBadgeRemote").to_string(),
        _ if !repo.source.is_empty() => repo.source.clone(),
        _ if !repo.kind.is_empty() => repo.kind.clone(),
        _ => app.i18n.t("search.sourceFallback").to_string(),
    }
}

pub fn select_repo(app: &mut App, index: usize) {
    if index >= app.state.search_repos.len() {
        return;
    }
    app.state.selected_repo = Some(index);
    app.ui.show("search-next-row");
}

pub fn proceed_from_search(app: &mut App) {
    let Some(repo) = app.state.selected_repo.and_then(|index| app.state.search_repos.get(index)).cloned() else { return };
    let Some(app_id) = app.state.search_app_id else { return };
    app.ui.hide("search-next-row");
    app.ui.show("manifest-loading");
    app.ui.hide("search-error");
    let args = json!({ "appId": app_id.to_string(), "repo": repo.name, "sha": repo.sha });
    app.tasks.call("get_repo_manifests", args, move |app, raw: Result<Value, String>| match raw {
        Ok(raw) => {
            let depots: Vec<Depot> = raw["manifests"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|m| Depot {
                            depot_id: str_of(m, "depot_id"),
                            manifest_id: opt_str(m, "manifest_id").unwrap_or_else(|| "N/A".to_string()),
                            depot_key: opt_str(m, "depot_key"),
                            size_bytes: m["size_bytes"].as_u64().filter(|size| *size > 0),
                        })
                        .collect()
                })
                .unwrap_or_default();
            app.state.account_mode = false;
            app.state.search_repo = Some(repo.name.clone());
            app.state.search_sha = repo.sha.clone();
            app.state.search_key_vdf_keys = (!raw["depot_keys"].is_null()).then(|| raw["depot_keys"].clone());
            app.ui.hide("manifest-loading");
            if depots.is_empty() {
                let message = t!(app, "errors.noManifests");
                show_search_error(app, message);
                telemetry::report_error(app, "manifests", None, Some("errors.noManifests"));
                app.ui.show("search-next-row");
                return;
            }
            app.state.set_parsed(Some(ParsedData { main_app_id: app_id.to_string(), depots, all_app_ids: Vec::new() }));
            select::show_selection_step(app);
        }
        Err(error) => {
            app.ui.hide("manifest-loading");
            let message = app.i18n.localize_error(&error);
            show_search_error(app, message);
            telemetry::report_error(app, "manifests", Some(&error), None);
            app.ui.show("search-next-row");
        }
    });
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-search" => perform_search(app),
        "btn-search-next" => proceed_from_search(app),
        _ => {
            if let Some(index) = id.strip_prefix("repo:").and_then(|i| i.parse::<usize>().ok()) {
                select_repo(app, index);
            } else if let Some(app_id) = id.strip_prefix("playtest:") {
                app.emit_event("playtest_opened", json!({}));
                app.ui.set_value("search-appid-input", app_id);
                perform_search(app);
            } else {
                return false;
            }
        }
    }
    true
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    let markup: String = match id {
        "repo-list" => app
            .state
            .search_repos
            .iter()
            .enumerate()
            .map(|(index, repo)| {
                let date = repo
                    .date
                    .as_ref()
                    .map(|date| format!(r#"<div class="repo-card__date">{}: {}</div>"#, app.i18n.t("search.repoUpdated"), escape_html(&date_time_medium_short(app.i18n.code(), date))))
                    .unwrap_or_default();
                let selected = if app.state.selected_repo == Some(index) { " selected" } else { "" };
                format!(
                    r#"<div class="repo-card{selected}" data-click="repo:{index}" data-key="repo-{index}"><div class="repo-card__radio"></div><div class="repo-card__info"><div class="repo-card__name">{}</div>{date}</div><span class="repo-card__badge repo-card__badge--archive">{}</span></div>"#,
                    escape_html(&repo_display_name(app, repo)),
                    escape_html(&repo_badge_text(app, repo)),
                )
            })
            .collect(),
        "search-playtest-row" => app
            .state
            .playtests
            .iter()
            .map(|p| {
                let app_id = str_of(p, "appId");
                format!(
                    r#"<span class="search-account-row__text">{}</span><button class="btn btn--small btn--outline" data-click="playtest:{}">{}</button>"#,
                    escape_html(&t!(app, "search.playtestFound", name = str_of(p, "name"), appId = app_id)),
                    escape_html(&app_id),
                    escape_html(app.i18n.t("search.openPlaytest")),
                )
            })
            .collect(),
        _ => return false,
    };
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}
