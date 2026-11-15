use egui::Key;
use serde_json::{json, Value};

use super::picker::{self, Outcome, Picker};
use super::select::steam_language_name;
use super::util::{escape_html, str_of};
use super::{account, download_control, journey, select_picker, shortcuts, sources, telemetry, ui};
use crate::app::{view, App};
use crate::dom::Dom;
use crate::t;

const SECTION_KEY: &str = "settingsSection";
const SECTIONS: [&str; 6] = ["general", "downloads", "depots", "sources", "steam", "privacy"];
const TARGET_PLATFORMS: [&str; 3] = ["windows", "linux", "macos"];
const GAME_LANGUAGES: [&str; 30] = [
    "english", "german", "french", "italian", "spanish", "latam", "schinese", "tchinese", "japanese", "koreana", "russian", "polish", "brazilian", "portuguese", "turkish", "ukrainian", "czech", "dutch", "danish",
    "finnish", "norwegian", "swedish", "hungarian", "romanian", "thai", "vietnamese", "greek", "bulgarian", "arabic", "indonesian",
];
const GAME_LANGUAGE_PICKER: Picker = Picker { picker: "game-language", button: "game-language-button", menu: "game-language-menu" };
const NOT_SAVED: [&str; 2] = ["settings-search", "sources-add-input"];

#[derive(Default)]
pub struct SettingsUi {
    pub saved_telemetry: bool,
    pub saved_behavior: bool,
    pub build_info: Option<Value>,
    pub folder_style: String,
    pub target_platform: String,
    pub game_language: String,
    pub baseline: Option<String>,
    pub language: String,
    pub regions: Option<Vec<Value>>,
    pub search_hit: Option<bool>,
    pub language_restart: bool,
}

pub fn open_settings(app: &mut App, target: Option<&str>) {
    account::load_account_status(app, |_| {});
    let target = target.map(str::to_string);
    let known = app.state.settings.regions.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let settings = tasks.invoke::<Value>("get_settings", Value::Null).await;
        let regions = match known {
            Some(regions) => regions,
            None => tasks.invoke::<Vec<Value>>("download_regions", Value::Null).await.unwrap_or_default(),
        };
        tasks.post(move |app| {
            app.state.settings.regions = Some(regions);
            match settings {
                Ok(settings) => populate(app, &settings),
                Err(_) => app.ui.set_checked("auto-update-toggle", true),
            }
            load_build_info(app);
            app.ui.set_value("sources-add-input", "");
            app.ui.hide("sources-add-error");
            reset_settings_language(app);
            sync_engine_dependent_settings(app);
            let section = target.or_else(|| app.store.get(SECTION_KEY)).unwrap_or_else(|| "general".to_string());
            show_settings_section(app, &section);
            reset_settings_dirty(app);
            app.emit_event("settings_opened", Value::Null);
            app.ui.show("settings-modal");
        });
    });
}

fn populate(app: &mut App, settings: &Value) {
    let flag = |key: &str| settings[key].as_bool().unwrap_or(false);
    let not_false = |key: &str| settings[key].as_bool() != Some(false);
    app.ui.set_checked("auto-update-toggle", not_false("auto_update"));
    let args: Vec<String> = settings["dd_extra_args"].as_array().map(|list| list.iter().filter_map(|a| a.as_str().map(str::to_string)).collect()).unwrap_or_default();
    app.ui.set_value("dd-extra-args-input", args.join(" "));
    app.ui.set_value("max-retries-input", settings["max_retries"].as_u64().unwrap_or(3).to_string());
    app.ui.set_value("chunk-concurrency-input", settings["native_chunk_concurrency"].as_u64().unwrap_or(16).to_string());
    let (value, unit) = split_speed_limit(&str_of(settings, "download_speed_limit"));
    app.ui.set_value("speed-limit-input", value.clone());
    set_speed_limit_unit(app, unit);
    set_speed_limit_enabled(app, !value.is_empty(), false);
    app.ui.set_value("proxy-input", str_of(settings, "proxy"));
    fill_download_server(app, settings);
    app.ui.set_value("steam-path-input", str_of(settings, "steam_path"));
    refresh_steam_path_status(app);
    app.ui.hide("proxy-error");
    set_proxy_status(app, "", String::new());
    app.ui.set_value("hubcap-apikey-input", str_of(settings, "hubcap_api_key"));
    app.ui.set_value("ryuu-apikey-input", str_of(settings, "ryuu_api_key"));
    app.ui.set_value("steam-webapi-key-input", str_of(settings, "steam_web_api_key"));
    app.ui.set_checked("native-downloader-toggle", flag("use_native_downloader"));
    apply_engine_lock(app);
    app.ui.set_checked("cancel-keep-files-toggle", flag("cancel_keep_files"));
    app.ui.set_checked("auto-select-depots-toggle", flag("auto_select_depots"));
    app.ui.set_checked("auto-start-download-toggle", flag("auto_start_download"));
    app.ui.set_checked("latest-manifests-toggle", flag("latest_manifests"));
    set_folder_style(app, &str_of(settings, "folder_style"));
    app.ui.set_checked("include-dlc-toggle", flag("include_dlc"));
    set_target_platform(app, &str_of(settings, "target_platform"));
    set_game_language(app, &str_of(settings, "game_language"));
    sync_auto_select_options(app);
    app.ui.set_checked("notification-sound-toggle", not_false("notification_sound"));
    app.ui.set_checked("remote-updates-toggle", not_false("remote_updates"));
    let telemetry_on = settings["telemetry_consent"] == "accepted";
    let behavior_on = settings["telemetry_behavior"] == "accepted";
    app.ui.set_checked("telemetry-toggle", telemetry_on);
    app.ui.set_checked("behavior-toggle", behavior_on);
    app.state.settings.saved_telemetry = telemetry_on;
    app.state.settings.saved_behavior = behavior_on;
    sync_behavior_toggle(app);
    let list = |key: &str| settings[key].as_array().map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_default();
    app.state.pending_sources = Some(list("depot_sources"));
    app.state.pristine_sources = Some(list("pristine_default_sources"));
}

fn load_build_info(app: &mut App) {
    app.tasks.call("get_build_info", Value::Null, |app, info: Result<Value, String>| {
        let Ok(info) = info else { return };
        let (text, class): (String, &'static str) = match info["channel"].as_str() {
            Some("stable") => (t!(app, "settings.channelStable"), "build-info__badge--stable"),
            Some("dev") => (t!(app, "settings.channelDev"), "build-info__badge--dev"),
            Some("dev-local") => (t!(app, "settings.channelDevLocal"), "build-info__badge--local"),
            Some(other) if !other.is_empty() => (other.to_string(), "build-info__badge--local"),
            _ => ("—".to_string(), "build-info__badge--local"),
        };
        app.ui.set_text("build-info-channel", text);
        app.ui.set_classes("build-info-channel", &["build-info__badge", class]);
        let or = |key: &str, fallback: String| super::util::opt_str(&info, key).unwrap_or(fallback);
        let unknown = t!(app, "common.unknown");
        app.ui.set_text("build-info-version", or("version", "—".to_string()));
        app.ui.set_text("build-info-sha", or("gitSha", unknown.clone()));
        app.ui.set_text("build-info-date", or("buildDate", unknown));
        app.ui.set_text("build-info-profile", or("profile", "—".to_string()));
        app.ui.set_text("build-info-platform", format!("{} / {} · {}", or("targetOs", "?".to_string()), or("targetArch", "?".to_string()), or("package", "?".to_string())));
        let diagnostic = or("diagnosticId", t!(app, "settings.diagnosticIdOff"));
        app.ui.set_text("build-info-diagnostic", diagnostic);
        app.state.settings.build_info = Some(info);
        apply_engine_lock(app);
    });
}

pub fn close_settings(app: &mut App) {
    app.ui.hide("settings-modal");
    app.state.pending_sources = None;
    app.state.settings.baseline = None;
    reset_settings_language(app);
}

fn show_save_error(app: &mut App, error: &str) -> bool {
    let lower = error.to_lowercase();
    if ["server address", "without a path", "download region"].iter().any(|needle| lower.contains(needle)) {
        let text = app.i18n.localize_error(error);
        app.ui.set_text("download-server-error", text);
        app.ui.show("download-server-error");
        show_settings_section(app, "sources");
        app.ui.request_focus("download-server-input");
        return true;
    }
    if lower.contains("proxy") {
        let text = app.i18n.localize_error(error);
        app.ui.set_text("proxy-error", text);
        app.ui.show("proxy-error");
        show_settings_section(app, "sources");
        app.ui.request_focus("proxy-input");
        return true;
    }
    false
}

fn apply_form(app: &App, current: &mut Value) {
    let text = |id: &str| app.ui.value(id).trim().to_string();
    current["auto_update"] = json!(app.ui.checked("auto-update-toggle"));
    let args = text("dd-extra-args-input");
    current["dd_extra_args"] = if args.is_empty() { json!(["-max-downloads", "8", "-verify-all"]) } else { json!(args.split_whitespace().collect::<Vec<_>>()) };
    let number = |id: &str, fallback: u64| app.ui.value(id).trim().chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse::<u64>().ok().filter(|n| *n > 0).unwrap_or(fallback);
    current["max_retries"] = json!(number("max-retries-input", 3));
    current["native_chunk_concurrency"] = json!(number("chunk-concurrency-input", 16));
    current["download_speed_limit"] = json!(if app.ui.checked("speed-limit-toggle") { join_speed_limit(app.ui.value("speed-limit-input"), get_speed_limit_unit(app)) } else { String::new() });
    current["proxy"] = json!(text("proxy-input"));
    telemetry::merge(current, read_download_server(app));
    current["hubcap_api_key"] = json!(text("hubcap-apikey-input"));
    current["ryuu_api_key"] = json!(text("ryuu-apikey-input"));
    current["steam_web_api_key"] = json!(text("steam-webapi-key-input"));
    current["use_native_downloader"] = json!(app.ui.checked("native-downloader-toggle"));
    current["cancel_keep_files"] = json!(app.ui.checked("cancel-keep-files-toggle"));
    current["steam_path"] = json!(text("steam-path-input"));
    current["auto_select_depots"] = json!(app.ui.checked("auto-select-depots-toggle"));
    current["auto_start_download"] = json!(app.ui.checked("auto-start-download-toggle"));
    current["latest_manifests"] = json!(app.ui.checked("latest-manifests-toggle"));
    current["folder_style"] = json!(app.state.settings.folder_style);
    current["include_dlc"] = json!(app.ui.checked("include-dlc-toggle"));
    current["target_platform"] = json!(app.state.settings.target_platform);
    current["game_language"] = json!(app.state.settings.game_language);
    current["notification_sound"] = json!(app.ui.checked("notification-sound-toggle"));
    current["remote_updates"] = json!(app.ui.checked("remote-updates-toggle"));
    if let Some(sources) = &app.state.pending_sources {
        current["depot_sources"] = json!(sources);
    }
    if !app.state.settings.language.is_empty() && app.state.settings.language != app.i18n.code() {
        current["language"] = json!(app.state.settings.language);
    }
}

pub fn save_settings(app: &mut App) {
    let changed = app.state.settings.baseline.as_ref().is_some_and(|baseline| *baseline != settings_snapshot(app));
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        let result: Result<(), String> = async {
            let before = tasks.invoke::<Value>("get_settings", Value::Null).await?;
            let seed = before.clone();
            let (current, telemetry_on, behavior_checked, saved_telemetry, saved_behavior) = tasks
                .ask(move |app| {
                    let mut current = seed;
                    apply_form(app, &mut current);
                    (current, app.ui.checked("telemetry-toggle"), app.ui.checked("behavior-toggle"), app.state.settings.saved_telemetry, app.state.settings.saved_behavior)
                })
                .await
                .ok_or_else(|| "closed".to_string())?;
            tasks.invoke_raw("save_settings", json!({ "settings": current })).await?;
            let mut keys: Vec<String> = current.as_object().map(|map| map.iter().filter(|(key, value)| before[key.as_str()] != **value).map(|(key, _)| key.clone()).collect()).unwrap_or_default();
            if telemetry_on != saved_telemetry {
                keys.push("telemetry_consent".to_string());
                tasks.invoke_raw("set_telemetry_consent", json!({ "accept": telemetry_on })).await?;
            }
            let behavior_on = behavior_checked && telemetry_on;
            if behavior_on != saved_behavior {
                keys.push("telemetry_behavior".to_string());
                tasks.invoke_raw("set_behavior_consent", json!({ "accept": behavior_on })).await?;
            }
            let sound = current["notification_sound"].as_bool().unwrap_or(true);
            let restart = tasks
                .ask(move |app| {
                    app.state.settings.saved_telemetry = telemetry_on;
                    if behavior_on != saved_behavior {
                        app.state.settings.saved_behavior = behavior_on;
                        if behavior_on {
                            journey::start_journey(app);
                        } else {
                            journey::stop_journey(app);
                        }
                    }
                    if !keys.is_empty() && telemetry_on {
                        app.emit_event("settings_saved", json!({ "keys": keys }));
                    }
                    app.state.pending_sources = None;
                    sources::refresh_sources_ui(app);
                    app.state.notification_sound_enabled = sound;
                    download_control::check_dot_net(app);
                    shortcuts::check_steam_library_support(app, |_| {});
                    app.state.settings.language_restart
                })
                .await
                .unwrap_or(false);
            if restart {
                tasks.post(|app| app.restart());
                return Ok(());
            }
            tasks.post(move |app| {
                if changed {
                    let saved = t!(app, "settings.saved");
                    ui::show_toast(app, saved, "success", 4000);
                }
            });
            Ok(())
        }
        .await;
        tasks.post(move |app| {
            if let Err(error) = &result {
                if show_save_error(app, error) {
                    return;
                }
            }
            close_settings(app);
        });
    });
}

fn split_speed_limit(text: &str) -> (String, &'static str) {
    let empty = (String::new(), "MB/s");
    let trimmed = text.trim();
    let split = trimmed.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(split);
    let Ok(number) = number.replacen(',', ".", 1).parse::<f64>() else { return empty };
    let key: String = unit.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect();
    let key = key.strip_suffix("/s").unwrap_or(&key);
    let (unit, factor) = match key {
        "" | "m" | "mb" | "mib" => ("MB/s", 1.0),
        "k" | "kb" | "kib" => ("MB/s", 1.0 / 1024.0),
        "g" | "gb" | "gib" => ("MB/s", 1024.0),
        "mbit" | "mbits" | "mbps" => ("Mbit/s", 1.0),
        "kbit" | "kbits" | "kbps" => ("Mbit/s", 0.001),
        "gbit" | "gbits" | "gbps" => ("Mbit/s", 1000.0),
        _ => return empty,
    };
    if !number.is_finite() || number <= 0.0 {
        return empty;
    }
    let scaled = (number * factor * 1000.0).round() / 1000.0;
    (scaled.to_string(), unit)
}

fn join_speed_limit(value: &str, unit: &str) -> String {
    match value.trim().replacen(',', ".", 1).parse::<f64>() {
        Ok(number) if number.is_finite() && number > 0.0 => format!("{} {}", number, if unit == "Mbit/s" { "Mbit/s" } else { "MB/s" }),
        _ => String::new(),
    }
}

fn get_speed_limit_unit(app: &App) -> &'static str {
    if app.ui.has_class("speed-unit-mbit", "is-active") {
        "Mbit/s"
    } else {
        "MB/s"
    }
}

fn set_speed_limit_unit(app: &mut App, unit: &str) {
    app.ui.set_class("speed-unit-mb", "is-active", unit != "Mbit/s");
    app.ui.set_class("speed-unit-mbit", "is-active", unit == "Mbit/s");
}

fn set_speed_limit_enabled(app: &mut App, enabled: bool, focus: bool) {
    app.ui.set_checked("speed-limit-toggle", enabled);
    app.ui.set_hidden("speed-limit-controls", !enabled);
    if enabled && focus {
        app.ui.request_focus("speed-limit-input");
    }
}

fn sync_auto_select_options(app: &mut App) {
    let off = !app.ui.checked("auto-select-depots-toggle");
    app.ui.set_class("auto-select-options", "is-disabled", off);
}

fn set_folder_style(app: &mut App, value: &str) {
    app.state.settings.folder_style = value.to_string();
    for option in ["", "underscore", "dash"] {
        app.ui.set_class(&format!("folder-style-options-{option}"), "is-active", option == value);
    }
}

fn set_target_platform(app: &mut App, value: &str) {
    let wanted: Vec<&str> = value.split(',').map(str::trim).collect();
    let list: Vec<&str> = TARGET_PLATFORMS.into_iter().filter(|p| wanted.contains(p)).collect();
    app.state.settings.target_platform = list.join(",");
    app.ui.set_class("target-platform-options-", "is-active", list.is_empty());
    for platform in TARGET_PLATFORMS {
        app.ui.set_class(&format!("target-platform-options-{platform}"), "is-active", list.contains(&platform));
    }
}

fn toggle_target_platform(app: &mut App, value: &str) {
    if value.is_empty() {
        return set_target_platform(app, "");
    }
    let mut list: Vec<String> = app.state.settings.target_platform.split(',').filter(|p| !p.is_empty()).map(str::to_string).collect();
    match list.iter().position(|p| p == value) {
        Some(index) => {
            list.remove(index);
        }
        None => list.push(value.to_string()),
    }
    set_target_platform(app, &list.join(","));
}

fn game_language_values() -> impl Iterator<Item = &'static str> {
    std::iter::once("").chain(GAME_LANGUAGES)
}

fn game_language_label(app: &App, value: &str) -> String {
    if value.is_empty() {
        app.i18n.t("settings.gameLanguageAuto").to_string()
    } else {
        steam_language_name(app, value)
    }
}

fn set_game_language(app: &mut App, value: &str) {
    app.state.settings.game_language = if GAME_LANGUAGES.contains(&value) { value.to_string() } else { String::new() };
    let label = game_language_label(app, &app.state.settings.game_language.clone());
    app.ui.set_text("game-language-label", label);
}

fn game_language_index(app: &App) -> Option<usize> {
    game_language_values().position(|value| value == app.state.settings.game_language)
}

fn choose_game_language(app: &mut App, value: &str) {
    set_game_language(app, value);
    picker::set_open(app, GAME_LANGUAGE_PICKER, false, None);
    app.ui.request_focus("game-language-button");
}

fn set_proxy_status(app: &mut App, kind: &str, text: String) {
    set_status_line(app, "proxy-status", kind, text);
}

fn set_status_line(app: &mut App, id: &str, kind: &str, text: String) {
    let classes: &[&'static str] = match kind {
        "busy" => &["proxy-status", "proxy-status--busy"],
        "ok" => &["proxy-status", "proxy-status--ok"],
        "error" => &["proxy-status", "proxy-status--error"],
        _ => &["proxy-status", "hidden"],
    };
    app.ui.set_classes(id, classes);
    app.ui.set_text(id, text);
}

fn proxy_kind(proxy: &str) -> &'static str {
    let lower = proxy.trim().to_lowercase();
    if lower.is_empty() {
        "none"
    } else if lower.starts_with("socks") {
        "socks"
    } else if lower.starts_with("https") {
        "https"
    } else {
        "http"
    }
}

fn test_proxy(app: &mut App) {
    let proxy = app.ui.value("proxy-input").trim().to_string();
    app.ui.hide("proxy-error");
    app.ui.set_disabled("btn-proxy-test", true);
    let testing = t!(app, if proxy.is_empty() { "settings.proxyTestingDirect" } else { "settings.proxyTesting" });
    set_proxy_status(app, "busy", testing);
    app.tasks.call("test_proxy", json!({ "proxy": proxy.clone() }), move |app, result: Result<Value, String>| {
        match result {
            Ok(result) => {
                let key = if result["proxy"].is_null() || result["proxy"] == "" { "settings.proxyOkDirect" } else { "settings.proxyOk" };
                let text = format!("✓ {}", t!(app, key, ms = result["millis"].as_u64().unwrap_or(0)));
                set_proxy_status(app, "ok", text);
                app.emit_event("proxy_tested", json!({ "proxy": proxy_kind(&proxy), "ok": true }));
            }
            Err(error) => {
                app.emit_event("proxy_tested", json!({ "proxy": proxy_kind(&proxy), "ok": false, "key": telemetry::error_key_or_unmatched(&error) }));
                let key = if proxy.is_empty() { "settings.proxyFailedDirect" } else { "settings.proxyFailed" };
                let text = format!("✗ {}", t!(app, key, error = app.i18n.localize_error(&error)));
                set_proxy_status(app, "error", text);
            }
        }
        app.ui.set_disabled("btn-proxy-test", false);
    });
}

fn refresh_steam_path_status(app: &mut App) {
    let path = app.ui.value("steam-path-input").trim().to_string();
    let detecting = t!(app, "steamLibrary.detecting");
    set_status_line(app, "steam-path-status", "busy", detecting);
    let (command, args) = if path.is_empty() { ("steam_library_detect", Value::Null) } else { ("steam_library_check_dir", json!({ "path": path.clone() })) };
    app.tasks.call(command, args, move |app, install: Result<Value, String>| match install {
        Ok(install) => {
            let name = super::util::opt_str(&install, "persona_name").unwrap_or_else(|| str_of(&install, "user_id3"));
            let text = format!("✓ {}", t!(app, if path.is_empty() { "settings.steamPathAuto" } else { "settings.steamPathFound" }, dir = str_of(&install, "steam_dir"), name = name));
            set_status_line(app, "steam-path-status", "ok", text);
        }
        Err(error) => {
            let text = format!("✗ {}", app.i18n.localize_error(&error));
            set_status_line(app, "steam-path-status", "error", text);
        }
    });
}

fn pick_steam_folder(app: &mut App, then: impl FnOnce(&mut App, String) + Send + 'static) {
    let platform = app.platform.clone();
    let tasks = app.tasks.clone();
    app.tasks.spawn(async move {
        if let Some(picked) = platform.pick_folder(None).await {
            let picked = picked.to_string_lossy().to_string();
            tasks.post(move |app| then(app, picked));
        }
    });
}

fn choose_steam_folder_for_shortcut(app: &mut App) {
    pick_steam_folder(app, |app, picked| {
        let tasks = app.tasks.clone();
        app.tasks.spawn(async move {
            let result: Result<(), String> = async {
                tasks.invoke_raw("steam_library_check_dir", json!({ "path": picked })).await?;
                let mut current = tasks.invoke::<Value>("get_settings", Value::Null).await?;
                current["steam_path"] = json!(picked);
                tasks.invoke_raw("save_settings", json!({ "settings": current })).await?;
                Ok(())
            }
            .await;
            tasks.post(move |app| match result {
                Ok(()) => shortcuts::check_steam_library_support(app, |app| {
                    if app.state.steam_library_supported {
                        app.ui.set_checked("shortcut-steam-library", true);
                    }
                }),
                Err(error) => {
                    let text = app.i18n.localize_error(&error);
                    app.ui.set_text("shortcut-steam-hint", text);
                }
            });
        });
    });
}

fn fill_download_server(app: &mut App, settings: &Value) {
    app.ui.hide("download-server-error");
    let regions = app.state.settings.regions.clone().unwrap_or_default();
    let mut options = vec![("auto".to_string(), t!(app, "settings.downloadServerAuto"))];
    for region in &regions {
        let cell = region["cell"].as_u64().unwrap_or(0);
        let key = format!("settings.regions.{cell}");
        let name = if app.i18n.has(&key) { app.i18n.t(&key).to_string() } else { str_of(region, "name") };
        options.push((format!("cell:{cell}"), t!(app, "settings.downloadServerRegion", name = name)));
    }
    options.push(("custom".to_string(), t!(app, "settings.downloadServerCustom")));
    select_picker::set_options(app, "download-server-select", options);
    let server = str_of(settings, "download_server").trim().to_string();
    let region = settings["download_region"].as_u64().unwrap_or(0);
    let value = if !server.is_empty() {
        "custom".to_string()
    } else if region > 0 && regions.iter().any(|r| r["cell"].as_u64() == Some(region)) {
        format!("cell:{region}")
    } else {
        "auto".to_string()
    };
    app.ui.set_value("download-server-select", value);
    app.ui.set_value("download-server-input", server);
    sync_download_server(app);
}

fn sync_download_server(app: &mut App) {
    let custom = app.ui.value("download-server-select") == "custom";
    app.ui.set_hidden("download-server-custom", !custom);
}

fn read_download_server(app: &App) -> Value {
    let value = app.ui.value("download-server-select");
    if value == "custom" {
        return json!({ "download_server": app.ui.value("download-server-input").trim(), "download_region": 0 });
    }
    let region = value.strip_prefix("cell:").and_then(|cell| cell.parse::<u64>().ok()).unwrap_or(0);
    json!({ "download_server": "", "download_region": region })
}

pub fn sync_behavior_toggle(app: &mut App) {
    let on = app.ui.checked("telemetry-toggle");
    app.ui.set_disabled("behavior-toggle", !on);
    app.ui.set_class("behavior-row", "is-disabled", !on);
}

pub fn show_settings_section(app: &mut App, section: &str) {
    if !app.ui.value("settings-search").is_empty() {
        app.ui.set_value("settings-search", "");
        filter_settings(app, "");
    }
    let target = SECTIONS.into_iter().find(|name| *name == section).unwrap_or("general");
    for name in SECTIONS {
        app.ui.set_class(&format!("settings-tab-{name}"), "is-active", name == target);
        app.ui.set_class(&format!("settings-panel-{name}"), "is-active", name == target);
    }
    app.store.set(SECTION_KEY, target);
}

fn filter_settings(app: &mut App, query: &str) {
    let query = query.trim().to_lowercase();
    let searching = !query.is_empty();
    app.ui.set_class("settings-modal", "settings--searching", searching);
    let mut any = false;
    for panel in SECTIONS {
        let mut panel_hit = false;
        let mut groups: Vec<(&str, &str, bool)> = Vec::new();
        for (row, group, title, section, keys, title_keys) in view::SETTINGS_ROWS.iter().filter(|entry| entry.3 == panel) {
            let _ = section;
            let title_hit = searching && title_keys.iter().any(|key| app.i18n.t(key).to_lowercase().contains(&query));
            let text: String = keys.iter().map(|key| crate::html::plain(app.i18n.t(key))).collect::<Vec<_>>().join(" ").to_lowercase();
            let hit = !searching || title_hit || text.contains(&query);
            app.ui.set_class(row, "is-filtered", !hit);
            match groups.iter_mut().find(|(name, _, _)| name == group) {
                Some(entry) => entry.2 |= hit,
                None => groups.push((group, title, hit)),
            }
        }
        for (group, title, hit) in groups {
            app.ui.set_class(group, "is-filtered", !hit);
            if !title.is_empty() {
                app.ui.set_class(title, "is-filtered", !hit);
            }
            panel_hit |= hit;
        }
        app.ui.set_class(&format!("settings-panel-{panel}"), "is-match", searching && panel_hit);
        any |= panel_hit;
    }
    app.ui.set_hidden("settings-no-results", !searching || any);
    if query.chars().count() >= 3 && !any && app.state.settings.search_hit != Some(false) {
        journey::journey_note(app, "settings.searchEmpty");
    }
    app.state.settings.search_hit = Some(!searching || any);
}

fn settings_snapshot(app: &App) -> String {
    let limit_on = app.ui.checked("speed-limit-toggle");
    let mut parts: Vec<String> = Vec::new();
    for (id, kind) in view::SETTINGS_INPUTS {
        if NOT_SAVED.contains(id) || (*id == "speed-limit-input" && !limit_on) {
            continue;
        }
        parts.push(if *kind == "checkbox" { format!("{id}={}", app.ui.checked(id)) } else { format!("{id}={}", app.ui.value(id).trim()) });
    }
    parts.push(format!("folder={}", app.state.settings.folder_style));
    parts.push(format!("platform={}", app.state.settings.target_platform));
    if limit_on {
        parts.push(format!("unit={}", get_speed_limit_unit(app)));
    }
    parts.push(format!("language={}", app.state.settings.language));
    parts.push(format!("gameLanguage={}", app.state.settings.game_language));
    parts.push(format!("sources={}", app.state.pending_sources.clone().unwrap_or_default().join(" ")));
    parts.join("|")
}

fn reset_settings_dirty(app: &mut App) {
    app.state.settings.baseline = Some(settings_snapshot(app));
    app.ui.hide("settings-dirty");
}

pub fn update_settings_dirty(app: &mut App) {
    let Some(baseline) = app.state.settings.baseline.clone() else { return };
    let same = settings_snapshot(app) == baseline;
    app.ui.set_hidden("settings-dirty", same);
}

fn apply_engine_lock(app: &mut App) {
    if app.state.settings.build_info.as_ref().is_none_or(|info| info["ddmAvailable"].as_bool() != Some(false)) {
        return;
    }
    app.ui.set_checked("native-downloader-toggle", true);
    app.ui.set_disabled("native-downloader-toggle", true);
    let hint = t!(app, "settings.nativeDownloaderOnly");
    app.ui.set_text("native-downloader-hint", hint);
    sync_engine_dependent_settings(app);
}

fn sync_engine_dependent_settings(app: &mut App) {
    let native = app.ui.checked("native-downloader-toggle");
    app.ui.set_class("dd-extra-args-row", "is-disabled", native);
}

fn update_settings_save_label(app: &mut App) {
    let restart = !app.state.settings.language.is_empty() && app.state.settings.language != app.i18n.code();
    let label = t!(app, if restart { "settings.languageRestartButton" } else { "settings.save" });
    app.ui.set_text("btn-settings-save", label);
    app.state.settings.language_restart = restart;
}

fn reset_settings_language(app: &mut App) {
    app.state.settings.language = app.i18n.code().to_string();
    update_settings_save_label(app);
}

pub fn init(app: &mut App) {
    set_game_language(app, "");
    reset_settings_language(app);
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-settings" => open_settings(app, None),
        "btn-settings-save" => save_settings(app),
        "btn-settings-cancel" | "settings-modal>modal__backdrop" => close_settings(app),
        "btn-proxy-test" => test_proxy(app),
        "btn-steam-path-browse" => pick_steam_folder(app, |app, picked| {
            app.ui.set_value("steam-path-input", picked);
            refresh_steam_path_status(app);
        }),
        "btn-shortcut-steam-choose" => choose_steam_folder_for_shortcut(app),
        "game-language-button" => {
            let open = !picker::is_open(app, GAME_LANGUAGE_PICKER);
            let active = game_language_index(app);
            picker::set_open(app, GAME_LANGUAGE_PICKER, open, active);
        }
        _ => {
            if let Some(section) = id.strip_prefix("settings-tab:") {
                show_settings_section(app, section);
            } else if let Some(unit) = id.strip_prefix("speed-unit:") {
                set_speed_limit_unit(app, unit);
            } else if let Some(value) = id.strip_prefix("folder-style-options:") {
                set_folder_style(app, value);
            } else if let Some(value) = id.strip_prefix("target-platform-options:") {
                toggle_target_platform(app, value);
            } else if let Some(value) = id.strip_prefix("game-language:") {
                choose_game_language(app, value);
            } else {
                return false;
            }
        }
    }
    true
}

pub fn changed(app: &mut App, id: &str) -> bool {
    match id {
        "speed-limit-toggle" => {
            let on = app.ui.checked(id);
            set_speed_limit_enabled(app, on, true);
        }
        "auto-select-depots-toggle" => sync_auto_select_options(app),
        "telemetry-toggle" => sync_behavior_toggle(app),
        "native-downloader-toggle" => sync_engine_dependent_settings(app),
        "download-server-select" => {
            sync_download_server(app);
            app.ui.hide("download-server-error");
            if app.ui.value(id) == "custom" {
                app.ui.request_focus("download-server-input");
            }
        }
        _ => return view::SETTINGS_INPUTS.iter().any(|(input, _)| *input == id),
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    match id {
        "settings-search" => {
            let query = app.ui.value(id).to_string();
            filter_settings(app, &query);
        }
        "proxy-input" => set_proxy_status(app, "", String::new()),
        "download-server-input" => app.ui.hide("download-server-error"),
        _ => return view::SETTINGS_INPUTS.iter().any(|(input, _)| *input == id),
    }
    true
}

pub fn key(app: &mut App, id: &str, key: Key) -> bool {
    match (id, key) {
        ("proxy-input", Key::Enter) => test_proxy(app),
        ("steam-path-input", Key::Enter) => refresh_steam_path_status(app),
        ("settings-search", Key::Escape) if !app.ui.value(id).is_empty() => {
            app.ui.set_value(id, "");
            filter_settings(app, "");
            return true;
        }
        _ => {}
    }
    false
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "settings-language-cards" => {
            let active = app.state.settings.language.clone();
            if let Some(code) = ui::language_cards(app, dom, &active) {
                app.state.settings.language = code.to_string();
                update_settings_save_label(app);
            }
        }
        "game-language-menu" => {
            let focused = picker::focused(app, GAME_LANGUAGE_PICKER);
            let markup: String = game_language_values()
                .enumerate()
                .map(|(index, value)| {
                    let active = if value == app.state.settings.game_language { " is-active" } else { "" };
                    let focus = if focused == Some(index) { " is-focused" } else { "" };
                    format!(r#"<li class="history-sort__option{active}{focus}" data-click="game-language:{value}">{}</li>"#, escape_html(&game_language_label(app, value)))
                })
                .collect();
            if let Some(action) = dom.markup(markup) {
                app.click(&action);
            }
        }
        _ => return false,
    }
    true
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    if app.ui.hidden("settings-modal") {
        return;
    }
    update_settings_dirty(app);
    let active = game_language_index(app);
    if let Outcome::Choose(index) = picker::frame(app, ctx, GAME_LANGUAGE_PICKER, GAME_LANGUAGES.len() + 1, active) {
        if let Some(value) = game_language_values().nth(index) {
            choose_game_language(app, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_speed_limits() {
        assert_eq!(split_speed_limit("10 MB/s"), ("10".to_string(), "MB/s"));
        assert_eq!(split_speed_limit("512k"), ("0.5".to_string(), "MB/s"));
        assert_eq!(split_speed_limit("100 mbit/s"), ("100".to_string(), "Mbit/s"));
        assert_eq!(split_speed_limit("2,5 gbit"), ("2500".to_string(), "Mbit/s"));
        assert_eq!(split_speed_limit("fast"), (String::new(), "MB/s"));
        assert_eq!(join_speed_limit("2,5", "Mbit/s"), "2.5 Mbit/s");
        assert_eq!(join_speed_limit("0", "MB/s"), "");
        assert_eq!(proxy_kind("socks5://x"), "socks");
        assert_eq!(proxy_kind(""), "none");
    }
}
