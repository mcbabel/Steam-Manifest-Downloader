use std::collections::BTreeSet;

use serde_json::{json, Value};

use super::util::{escape_html, opt_str, str_of};
use crate::app::App;
use crate::dom::Dom;

const DEFAULT_LINKS: [(&str, &str); 3] = [
    ("achievement_watcher", "https://github.com/Shirowwww/Achievement-Watcher-Next"),
    ("issues", "https://github.com/MCbabel/Steam-Manifest-Downloader/issues"),
    ("releases", "https://github.com/MCbabel/Steam-Manifest-Downloader/releases"),
];
const DISMISSED_KEY: &str = "dismissedNotices_v1";
const OFFLINE_RECHECK_MS: u64 = 20_000;

#[derive(Default)]
pub struct RemoteUi {
    pub links: Vec<(String, String)>,
    pub notices: Vec<Value>,
    pub shown: BTreeSet<String>,
    pub offline: Option<bool>,
    pub offline_reported: bool,
    pub offline_timer: u64,
}

fn apply_remote_links(app: &mut App, links: &Value) {
    app.state.remote.links = DEFAULT_LINKS.iter().filter_map(|(key, url)| opt_str(links, key).filter(|next| next != url).map(|next| (url.to_string(), next))).collect();
}

pub fn resolve_url(app: &App, url: &str) -> String {
    app.state.remote.links.iter().find(|(from, _)| from == url).map(|(_, to)| to.clone()).unwrap_or_else(|| url.to_string())
}

fn dismissed_notices(app: &App) -> BTreeSet<String> {
    app.store.get(DISMISSED_KEY).and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok()).map(|list| list.into_iter().collect()).unwrap_or_default()
}

fn notice_text(app: &App, notice: &Value) -> String {
    let text = &notice["text"];
    opt_str(text, app.i18n.code()).or_else(|| opt_str(text, "en")).or_else(|| text.as_object().and_then(|map| map.values().find_map(|v| v.as_str().map(str::to_string)))).unwrap_or_default()
}

fn notice_props(notice: &Value, action: &str) -> Value {
    json!({ "action": action, "id": notice["id"], "level": if notice["level"] == "warning" { "warning" } else { "info" } })
}

fn render_remote_notices(app: &mut App, notices: Vec<Value>) {
    let hidden = dismissed_notices(app);
    let visible: Vec<Value> = notices.into_iter().filter(|n| !hidden.contains(&str_of(n, "id"))).collect();
    for notice in &visible {
        let id = str_of(notice, "id");
        if !notice_text(app, notice).is_empty() && app.state.remote.shown.insert(id) {
            app.emit_event("remote_notice", notice_props(notice, "shown"));
        }
    }
    app.state.remote.notices = visible;
}

pub fn refresh_remote(app: &mut App) {
    app.tasks.call("remote_refresh", Value::Null, |app, view: Result<Value, String>| {
        let Ok(view) = view else { return };
        apply_remote_links(app, &view["links"]);
        render_remote_notices(app, view["notices"].as_array().cloned().unwrap_or_default());
    });
}

pub fn check_connectivity(app: &mut App, first: bool) {
    app.tasks.call("connectivity_check", Value::Null, move |app, online: Result<bool, String>| {
        let online = online.unwrap_or(true);
        let was_offline = app.state.remote.offline == Some(true);
        app.state.remote.offline = Some(!online);
        app.ui.set_hidden("offline-banner", online);
        if !online && !app.state.remote.offline_reported {
            app.state.remote.offline_reported = true;
            app.emit_event("offline_notice", json!({}));
        }
        app.state.remote.offline_timer += 1;
        if !online {
            let token = app.state.remote.offline_timer;
            let tasks = app.tasks.clone();
            app.tasks.spawn_quiet(async move {
                tasks.sleep(OFFLINE_RECHECK_MS).await;
                tasks.post(move |app| {
                    if app.state.remote.offline_timer == token {
                        check_connectivity(app, false);
                    }
                });
            });
        } else if was_offline || first {
            refresh_remote(app);
        }
    });
}

pub fn init_remote(app: &mut App) {
    check_connectivity(app, true);
}

fn notice_by_id(app: &App, id: &str) -> Option<Value> {
    app.state.remote.notices.iter().find(|n| n["id"] == id).cloned()
}

pub fn click(app: &mut App, id: &str) -> bool {
    if let Some(notice) = id.strip_prefix("notice-open:").and_then(|id| notice_by_id(app, id)) {
        app.emit_event("remote_notice", notice_props(&notice, "opened"));
        app.open_url(&str_of(&notice, "url"));
        return true;
    }
    if let Some(notice) = id.strip_prefix("notice-dismiss:").and_then(|id| notice_by_id(app, id)) {
        let mut set = dismissed_notices(app);
        set.insert(str_of(&notice, "id"));
        let text = serde_json::to_string(&set.into_iter().collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string());
        app.store.set(DISMISSED_KEY, &text);
        app.emit_event("remote_notice", notice_props(&notice, "dismissed"));
        app.state.remote.notices.retain(|n| n["id"] != notice["id"]);
        return true;
    }
    false
}

pub fn is_empty(app: &App, id: &str) -> Option<bool> {
    (id == "remote-notices").then(|| app.state.remote.notices.iter().all(|notice| notice_text(app, notice).is_empty()))
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    if id != "remote-notices" {
        return false;
    }
    let markup: String = app
        .state
        .remote
        .notices
        .iter()
        .filter_map(|notice| {
            let text = notice_text(app, notice);
            if text.is_empty() {
                return None;
            }
            let notice_id = escape_html(&str_of(notice, "id"));
            let link = opt_str(notice, "url").map(|_| format!(r#"<a class="link app-banner__link" data-click="notice-open:{notice_id}">{}</a>"#, escape_html(app.i18n.t("remote.more")))).unwrap_or_default();
            Some(format!(
                r#"<div class="app-banner app-banner--{}" data-key="{notice_id}"><span>{}</span>{link}<button class="app-banner__close" data-click="notice-dismiss:{notice_id}">×</button></div>"#,
                if notice["level"] == "warning" { "warning" } else { "info" },
                escape_html(&text),
            ))
        })
        .collect();
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}
