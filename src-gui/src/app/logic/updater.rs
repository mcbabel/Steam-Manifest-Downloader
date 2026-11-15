use std::sync::OnceLock;

use regex::{Captures, Regex};
use serde_json::{json, Value};

use super::util::{date_medium, escape_html, format_bytes, opt_str, parse_date, str_of};
use super::telemetry;
use crate::app::App;
use crate::dom::Dom;
use crate::t;

const SKIPPED_VERSION_KEY: &str = "skippedUpdateVersion";
const RELEASE_REPO: &str = "MCbabel/Steam-Manifest-Downloader";

pub fn check_for_updates(app: &mut App) {
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if !tasks.invoke::<bool>("get_auto_update_enabled", Value::Null).await.unwrap_or(false) {
            return;
        }
        let Ok(result) = tasks.invoke::<Value>("check_for_updates", Value::Null).await else { return };
        tasks.post(move |app| {
            let error = opt_str(&result, "error");
            let available = result["available"].as_bool().unwrap_or(false);
            app.emit_event("update_checked", json!({ "available": available, "failed": error.is_some() }));
            if let Some(error) = &error {
                telemetry::report_error(app, "update_check", Some(error), None);
            }
            if !available || app.store.get(SKIPPED_VERSION_KEY).as_deref() == result["version"].as_str() {
                return;
            }
            show_update_modal(app, result);
        });
    });
}

fn pattern(cell: &'static OnceLock<Regex>, source: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(source).expect("pattern"))
}

fn release_inline(text: &str) -> String {
    static CODE: OnceLock<Regex> = OnceLock::new();
    static IMAGE: OnceLock<Regex> = OnceLock::new();
    static LINK: OnceLock<Regex> = OnceLock::new();
    static URL: OnceLock<Regex> = OnceLock::new();
    static REPO_REF: OnceLock<Regex> = OnceLock::new();
    static ISSUE_REF: OnceLock<Regex> = OnceLock::new();
    static USER_REF: OnceLock<Regex> = OnceLock::new();
    static BOLD_ITALIC: OnceLock<Regex> = OnceLock::new();
    static BOLD: OnceLock<Regex> = OnceLock::new();
    static STAR: OnceLock<Regex> = OnceLock::new();
    static UNDERSCORE: OnceLock<Regex> = OnceLock::new();
    static LINK_SLOT: OnceLock<Regex> = OnceLock::new();
    static CODE_SLOT: OnceLock<Regex> = OnceLock::new();

    let mut codes: Vec<String> = Vec::new();
    let mut out = pattern(&CODE, r"`([^`]+)`")
        .replace_all(text, |c: &Captures| {
            codes.push(c[1].to_string());
            format!("\u{0}{}\u{0}", codes.len() - 1)
        })
        .to_string();
    let mut links: Vec<String> = Vec::new();
    let mut keep = |html: String| {
        links.push(html);
        format!("\u{1}{}\u{1}", links.len() - 1)
    };
    out = pattern(&IMAGE, r"!\[[^\]]*\]\([^)]*\)").replace_all(&out, "").to_string();
    out = pattern(&LINK, r"\[([^\]]+)\]\(([^)\s]+)\)")
        .replace_all(&out, |c: &Captures| {
            let url = &c[2];
            let lower = url.to_lowercase();
            if lower.starts_with("http://") || lower.starts_with("https://") {
                keep(format!(r#"<a href="{}">{}</a>"#, escape_html(url), escape_html(&c[1])))
            } else {
                c[1].to_string()
            }
        })
        .to_string();
    out = pattern(&URL, r"https?://[^\s<>()]+[^\s<>().,:;!?]")
        .replace_all(&out, |c: &Captures| {
            let url = &c[0];
            let shown = url.trim_start_matches("https://").trim_start_matches("http://");
            keep(format!(r#"<a href="{}">{}</a>"#, escape_html(url), escape_html(shown)))
        })
        .to_string();
    out = pattern(&REPO_REF, r"\b([A-Za-z0-9-]+/[A-Za-z0-9._-]+)#(\d+)\b")
        .replace_all(&out, |c: &Captures| keep(format!(r#"<a class="update-modal__ref" href="https://github.com/{}/issues/{}">{}</a>"#, &c[1], &c[2], escape_html(&c[0]))))
        .to_string();
    out = pattern(&ISSUE_REF, r"(^|[^\w&/])#(\d+)\b")
        .replace_all(&out, |c: &Captures| format!("{}{}", &c[1], keep(format!(r#"<a class="update-modal__ref" href="https://github.com/{RELEASE_REPO}/issues/{n}">#{n}</a>"#, n = &c[2]))))
        .to_string();
    out = pattern(&USER_REF, r"(^|[^\w/])@([A-Za-z0-9][A-Za-z0-9-]{0,38})\b")
        .replace_all(&out, |c: &Captures| format!("{}{}", &c[1], keep(format!(r#"<a class="update-modal__ref" href="https://github.com/{user}">@{user}</a>"#, user = escape_html(&c[2])))))
        .to_string();
    out = escape_html(&out);
    out = pattern(&BOLD_ITALIC, r"\*\*\*(.+?)\*\*\*").replace_all(&out, "<strong><em>$1</em></strong>").to_string();
    out = pattern(&BOLD, r"\*\*(.+?)\*\*").replace_all(&out, "<strong>$1</strong>").to_string();
    out = pattern(&STAR, r"(^|[^*\w])\*([^*\s][^*]*?)\*([^*\w]|$)").replace_all(&out, "$1<em>$2</em>$3").to_string();
    out = pattern(&UNDERSCORE, r"(^|[^_\w])_([^_\s][^_]*?)_([^_\w]|$)").replace_all(&out, "$1<em>$2</em>$3").to_string();
    out = pattern(&LINK_SLOT, "\u{1}(\\d+)\u{1}").replace_all(&out, |c: &Captures| c[1].parse::<usize>().ok().and_then(|i| links.get(i).cloned()).unwrap_or_default()).to_string();
    out = pattern(&CODE_SLOT, "\u{0}(\\d+)\u{0}")
        .replace_all(&out, |c: &Captures| format!("<code>{}</code>", escape_html(c[1].parse::<usize>().ok().and_then(|i| codes.get(i)).map(String::as_str).unwrap_or(""))))
        .to_string();
    out.trim().to_string()
}

fn release_cells(line: &str) -> Vec<String> {
    line.trim().trim_start_matches('|').trim_end_matches('|').split('|').map(|cell| cell.trim().to_string()).collect()
}

fn heading_of(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    let rest = &line[level..];
    ((1..=6).contains(&level) && rest.starts_with(char::is_whitespace) && !rest.trim().is_empty()).then(|| (level, rest.trim()))
}

fn list_item(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix(['-', '*', '+']) {
        return (rest.starts_with(char::is_whitespace) && !rest.trim().is_empty()).then(|| (false, rest.trim()));
    }
    let digits = line.chars().take_while(|c| c.is_ascii_digit()).count();
    let rest = line[digits..].strip_prefix(['.', ')'])?;
    (digits > 0 && rest.starts_with(char::is_whitespace) && !rest.trim().is_empty()).then(|| (true, rest.trim()))
}

fn is_rule(line: &str) -> bool {
    line.len() >= 3 && ['-', '*', '_'].iter().any(|mark| line.chars().all(|c| c == *mark))
}

fn is_table_divider(line: &str) -> bool {
    let rest = line.trim().trim_start_matches('|').trim_start().trim_start_matches(':');
    rest.starts_with("--")
}

pub fn render_release_notes(markdown: &str) -> String {
    let text = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    let mut html: Vec<String> = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut list: Option<(bool, Vec<String>)> = None;
    let mut skip_level = 0usize;
    fn flush_para(html: &mut Vec<String>, para: &mut Vec<&str>) {
        if !para.is_empty() {
            html.push(format!("<p>{}</p>", release_inline(&para.join(" "))));
            para.clear();
        }
    }
    fn flush_list(html: &mut Vec<String>, list: &mut Option<(bool, Vec<String>)>) {
        if let Some((ordered, items)) = list.take() {
            let tag = if ordered { "ol" } else { "ul" };
            html.push(format!("<{tag}>{}</{tag}>", items.iter().map(|item| format!("<li>{}</li>", release_inline(item))).collect::<String>()));
        }
    }
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        index += 1;
        let heading = heading_of(trimmed);
        if let Some((level, _)) = heading {
            if skip_level > 0 && level <= skip_level {
                skip_level = 0;
            }
        }
        if skip_level > 0 {
            continue;
        }
        if let Some((level, title)) = heading {
            let plain: String = title.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_').collect();
            let lower = plain.trim().to_lowercase();
            if level >= 2 && (lower.ends_with("download") || lower.ends_with("downloads")) && lower.rsplit(|c: char| !c.is_alphanumeric() && c != '_').next().is_some_and(|word| word == "download" || word == "downloads") {
                flush_para(&mut html, &mut para);
                flush_list(&mut html, &mut list);
                skip_level = level;
                continue;
            }
        }
        if trimmed.is_empty() {
            flush_para(&mut html, &mut para);
            flush_list(&mut html, &mut list);
            continue;
        }
        if let Some((level, title)) = heading {
            flush_para(&mut html, &mut para);
            flush_list(&mut html, &mut list);
            let level = level.clamp(2, 4);
            html.push(format!("<h{level}>{}</h{level}>", release_inline(title)));
            continue;
        }
        if is_rule(trimmed) {
            flush_para(&mut html, &mut para);
            flush_list(&mut html, &mut list);
            if html.last().is_some_and(|last| !last.starts_with("<hr")) {
                html.push("<hr>".to_string());
            }
            continue;
        }
        if trimmed.starts_with('|') && lines.get(index).is_some_and(|next| is_table_divider(next)) {
            flush_para(&mut html, &mut para);
            flush_list(&mut html, &mut list);
            let head = release_cells(trimmed);
            index += 1;
            let mut rows = Vec::new();
            while lines.get(index).is_some_and(|next| next.trim().starts_with('|')) {
                rows.push(release_cells(lines[index]));
                index += 1;
            }
            let cells = |row: &[String], tag: &str| row.iter().map(|cell| format!("<{tag}>{}</{tag}>", release_inline(cell))).collect::<String>();
            html.push(format!("<table><thead><tr>{}</tr></thead><tbody>{}</tbody></table>", cells(&head, "th"), rows.iter().map(|row| format!("<tr>{}</tr>", cells(row, "td"))).collect::<String>()));
            continue;
        }
        if let Some((ordered, item)) = list_item(trimmed) {
            flush_para(&mut html, &mut para);
            if list.as_ref().is_none_or(|(kind, _)| *kind != ordered) {
                flush_list(&mut html, &mut list);
                list = Some((ordered, Vec::new()));
            }
            if let Some((_, items)) = list.as_mut() {
                items.push(item.to_string());
            }
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            if let Some(last) = list.as_mut().and_then(|(_, items)| items.last_mut()) {
                last.push(' ');
                last.push_str(trimmed);
                continue;
            }
        }
        flush_list(&mut html, &mut list);
        if release_inline(trimmed).is_empty() {
            continue;
        }
        para.push(trimmed);
    }
    flush_para(&mut html, &mut para);
    flush_list(&mut html, &mut list);
    while html.last().is_some_and(|last| last == "<hr>") {
        html.pop();
    }
    html.join("\n")
}

fn show_style(app: &mut App, id: &str, shown: bool) {
    if shown {
        app.ui.clear_style(id, "display");
    } else {
        app.ui.set_style(id, "display", "none");
    }
}

pub fn show_update_modal(app: &mut App, info: Value) {
    app.ui.set_text("update-version", format!("v{}", str_of(&info, "version")));
    let current = opt_str(&info, "currentVersion").map(|v| format!("v{}", v.trim_start_matches('v'))).unwrap_or_default();
    app.ui.set_text("update-current", current);
    match opt_str(&info, "date") {
        Some(date) => {
            let shown = parse_date(&date).map(|d| date_medium(app.i18n.code(), &d)).unwrap_or(date);
            app.ui.set_text("update-date", shown);
            show_style(app, "update-date-row", true);
        }
        None => show_style(app, "update-date-row", false),
    }
    let notes = opt_str(&info, "body").map(|body| render_release_notes(&body)).filter(|html| !html.is_empty());
    let notes = notes.unwrap_or_else(|| format!(r#"<p class="update-modal__empty">{}</p>"#, escape_html(app.i18n.t("modals.update.noReleaseNotes"))));
    app.ui.set_html("update-notes", notes);
    app.ui.hide("update-progress-wrap");
    show_style(app, "update-actions", true);
    app.ui.set_disabled("btn-update-now", false);
    show_style(app, "btn-update-later", true);
    show_style(app, "btn-update-skip", true);
    update_primary_label(app, &info);
    let method = str_of(&info, "installMethod");
    let external = !method.is_empty() && method != "self";
    app.ui.set_hidden("update-system-hint", !external);
    app.state.update_commands = if external { update_commands_for(app, &method) } else { Vec::new() };
    app.ui.set_hidden("btn-update-now", external);
    app.ui.set_hidden("btn-update-skip", external);
    app.ui.set_hidden("btn-update-notes-link", opt_str(&info, "releaseUrl").is_none());
    app.state.pending_update = Some(info);
    app.ui.show("update-modal");
    app.ui.request_focus(if external { "btn-update-later" } else { "btn-update-now" });
}

fn open_external_url(app: &mut App, url: &str) {
    let lower = url.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        app.open_url(url);
    }
}

fn update_commands_for(app: &App, method: &str) -> Vec<(String, String)> {
    match method {
        "flatpak" => vec![(String::new(), "flatpak update de.mcbabel.SteamManifestDownloader".to_string())],
        "snap" => vec![(String::new(), "sudo snap refresh steam-manifest-downloader".to_string())],
        "scoop" => vec![(String::new(), "scoop update steam-manifest-downloader".to_string())],
        _ => vec![
            (app.i18n.t("modals.update.aurBinLabel").to_string(), "paru -Syu steam-manifest-downloader-bin".to_string()),
            (app.i18n.t("modals.update.aurSourceLabel").to_string(), "paru -Syu steam-manifest-downloader".to_string()),
        ],
    }
}

fn hide_update_modal(app: &mut App) {
    app.ui.hide("update-modal");
}

fn update_primary_label(app: &mut App, info: &Value) {
    let page = opt_str(info, "installerUrl").is_none() || info["updateKind"] == "page";
    let label = t!(app, if page { "modals.update.openPage" } else { "modals.update.now" });
    app.ui.set_text("btn-update-now", label);
}

fn set_update_progress(app: &mut App, done: f64, total: f64) {
    let size = |bytes: f64| format_bytes(bytes).unwrap_or_else(|| "null".to_string());
    if total > 0.0 {
        let percent = ((done / total) * 100.0).round().min(100.0);
        app.ui.remove_class("update-progress-fill", "progress-bar__fill--indeterminate");
        app.ui.set_style("update-progress-fill", "width", format!("{percent}%"));
        let text = t!(app, "modals.update.progress", done = size(done), total = size(total), percent = percent);
        app.ui.set_text("update-progress-text", text);
    } else {
        let text = t!(app, "modals.update.progressUnknown", done = size(done));
        app.ui.set_text("update-progress-text", text);
    }
}

fn perform_update(app: &mut App) {
    let info = app.state.pending_update.clone().unwrap_or(Value::Null);
    let installer = opt_str(&info, "installerUrl");
    let kind = opt_str(&info, "updateKind");
    let Some(installer) = installer.filter(|_| kind.as_deref() != Some("page")) else {
        if let Some(url) = opt_str(&info, "releaseUrl") {
            open_external_url(app, &url);
        }
        app.emit_event("update_dismissed", json!({ "action": "release_page" }));
        return hide_update_modal(app);
    };
    let appimage = kind.as_deref() == Some("appimage");
    app.ui.set_disabled("btn-update-now", true);
    show_style(app, "btn-update-later", false);
    show_style(app, "btn-update-skip", false);
    let downloading = t!(app, "modals.update.downloading");
    app.ui.set_text("btn-update-now", downloading);
    app.ui.show("update-progress-wrap");
    let text = t!(app, if appimage { "modals.update.appImageDownloading" } else { "modals.update.installerDownloading" });
    app.ui.set_text("update-progress-text", text);
    app.ui.set_style("update-progress-fill", "width", "100%");
    app.ui.add_class("update-progress-fill", "progress-bar__fill--indeterminate");
    app.state.update_listening = appimage;
    app.emit_event("update_installed", json!({ "via": kind.clone().unwrap_or_else(|| "installer".to_string()) }));
    let args = json!({ "installerUrl": installer, "updateKind": kind, "assetDigest": opt_str(&info, "assetDigest") });
    app.tasks.call("install_update", args, |app, result: Result<Value, String>| {
        app.state.update_listening = false;
        let Err(error) = result else { return };
        telemetry::report_error(app, "update_install", Some(&error), None);
        let text = t!(app, "modals.update.failed", message = app.i18n.localize_error(&error));
        app.ui.set_text("update-progress-text", text);
        app.ui.remove_class("update-progress-fill", "progress-bar__fill--indeterminate");
        app.ui.set_style("update-progress-fill", "width", "0%");
        let retry = t!(app, "common.retry");
        app.ui.set_text("btn-update-now", retry);
        app.ui.set_disabled("btn-update-now", false);
        show_style(app, "btn-update-later", true);
    });
}

fn skip_update_version(app: &mut App) {
    let version = app.ui.text("update-version").unwrap_or("").trim_start_matches('v').to_string();
    app.store.set(SKIPPED_VERSION_KEY, &version);
    app.emit_event("update_dismissed", json!({ "action": "skip" }));
    hide_update_modal(app);
}

pub fn event(app: &mut App, channel: &str, payload: &Value) -> bool {
    if channel != "update-progress" {
        return false;
    }
    if app.state.update_listening {
        let (done, total) = (payload["done"].as_f64().unwrap_or(0.0), payload["total"].as_f64().unwrap_or(0.0));
        set_update_progress(app, done, total);
        if total > 0.0 && done >= total {
            let restarting = t!(app, "modals.update.restarting");
            app.ui.set_text("update-progress-text", restarting);
        }
    }
    true
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-update-now" => perform_update(app),
        "btn-update-later" => {
            app.emit_event("update_dismissed", json!({ "action": "later" }));
            hide_update_modal(app);
        }
        "btn-update-skip" => skip_update_version(app),
        "btn-update-notes-link" => {
            if let Some(url) = app.state.pending_update.as_ref().and_then(|info| opt_str(info, "releaseUrl")) {
                open_external_url(app, &url);
            }
            app.emit_event("update_dismissed", json!({ "action": "github" }));
        }
        "update-modal>modal__backdrop" => hide_update_modal(app),
        _ => match id.strip_prefix("update-copy:").and_then(|index| index.parse::<usize>().ok()) {
            Some(index) => {
                if let Some((_, command)) = app.state.update_commands.get(index).cloned() {
                    app.ctx.copy_text(command);
                    app.state.update_copied = Some(index);
                    app.state.timer_serial += 1;
                    let token = app.state.timer_serial;
                    app.state.update_copy_timer = token;
                    let tasks = app.tasks.clone();
                    app.tasks.spawn(async move {
                        tasks.sleep(1500).await;
                        tasks.post(move |app| {
                            if app.state.update_copy_timer == token {
                                app.state.update_copied = None;
                            }
                        });
                    });
                }
            }
            None => return false,
        },
    }
    true
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    if id != "update-system-cmd-list" {
        return false;
    }
    let copy = escape_html(app.i18n.t("modals.update.copyCmd"));
    let copied = escape_html(app.i18n.t("common.copied"));
    let markup: String = app
        .state
        .update_commands
        .iter()
        .enumerate()
        .map(|(index, (label, command))| {
            let label = if label.is_empty() { String::new() } else { format!(r#"<div class="update-system-cmd__label">{}</div>"#, escape_html(label)) };
            format!(
                r#"<div class="update-system-cmd__row">{label}<div class="update-system-cmd__inner"><pre><code>{}</code></pre><button class="btn btn--outline btn--small update-system-cmd__copy" data-click="update-copy:{index}">{}</button></div></div>"#,
                escape_html(command),
                if app.state.update_copied == Some(index) { &copied } else { &copy },
            )
        })
        .collect();
    if let Some(action) = dom.markup(markup) {
        app.click(&action);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_release_notes_into_markup() {
        let html = render_release_notes("### What's New\n- **Fast** downloads `x`\n- Fix #12 by @someone\n\nSee https://example.com/page.\n\n## Downloads\n| a | b |\n\n## Notes\ntext");
        assert!(html.contains("<h3>What&#39;s New</h3>"), "{html}");
        assert!(html.contains("<li><strong>Fast</strong> downloads <code>x</code></li>"), "{html}");
        assert!(html.contains(r#"href="https://github.com/MCbabel/Steam-Manifest-Downloader/issues/12">#12</a>"#), "{html}");
        assert!(html.contains(r#"href="https://github.com/someone">@someone</a>"#), "{html}");
        assert!(html.contains(r#"<a href="https://example.com/page">example.com/page</a>."#), "{html}");
        assert!(!html.contains("| a | b |") && html.contains("<h2>Notes</h2>"), "{html}");
        assert_eq!(release_inline("*soft* and _also_"), "<em>soft</em> and <em>also</em>");
    }
}
