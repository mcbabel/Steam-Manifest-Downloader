use serde_json::{json, Map, Value};

use super::{select_picker, telemetry, ui};
use crate::app::App;
use crate::dom::{el, Dom};
use crate::style::selector::{CHECKED, HAS_CHECKED};
use crate::t;

pub const GROUPS: [&str; 7] = ["account", "saves", "system", "network", "stats", "overlay", "compat"];
const LAST_SETTINGS_KEY: &str = "lastEmuSettings_v1";
const HARDWARE_OPTIONS: [(&str, &str); 5] = [("", "hwDefault"), ("0", "hwNone"), ("1", "hwDeck"), ("2", "hwMachine"), ("3", "hwFrame")];

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Text,
    Int,
    Float,
    Bool,
    Invert,
    Select,
}

pub struct Def {
    pub key: &'static str,
    pub kind: Kind,
    pub group: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub placeholder_key: &'static str,
}

const fn def(key: &'static str, kind: Kind, group: &'static str, label: &'static str) -> Def {
    Def { key, kind, group, label, placeholder: "", placeholder_key: "" }
}

const fn hint(key: &'static str, kind: Kind, group: &'static str, label: &'static str, placeholder: &'static str) -> Def {
    Def { key, kind, group, label, placeholder, placeholder_key: "" }
}

const fn keyed(key: &'static str, kind: Kind, group: &'static str, label: &'static str, placeholder_key: &'static str) -> Def {
    Def { key, kind, group, label, placeholder: "", placeholder_key }
}

pub const SETTINGS: [Def; 39] = [
    keyed("account_name", Kind::Text, "account", "accountName", "accountNamePlaceholder"),
    hint("account_steamid", Kind::Text, "account", "accountSteamId", "76561198000000000"),
    hint("language", Kind::Text, "account", "language", "english"),
    hint("ip_country", Kind::Text, "account", "ipCountry", "US"),
    keyed("local_save_path", Kind::Text, "saves", "localSavePath", "localSavePathPlaceholder"),
    hint("saves_folder_name", Kind::Text, "saves", "savesFolderName", "GSE Saves"),
    def("steam_hardware_type", Kind::Select, "system", "steamHardwareType"),
    def("is_under_proton", Kind::Bool, "system", "isUnderProton"),
    def("offline", Kind::Bool, "system", "offline"),
    def("enable_account_avatar", Kind::Bool, "system", "enableAccountAvatar"),
    def("new_app_ticket", Kind::Invert, "system", "oldAppTicket"),
    def("disable_networking", Kind::Bool, "network", "disableNetworking"),
    def("disable_lan_only", Kind::Bool, "network", "disableLanOnly"),
    hint("listen_port", Kind::Int, "network", "listenPort", "47584"),
    def("disable_lobby_creation", Kind::Bool, "network", "disableLobbyCreation"),
    def("share_leaderboards_over_network", Kind::Bool, "network", "shareLeaderboards"),
    def("record_playtime", Kind::Bool, "stats", "recordPlaytime"),
    def("pause_total_when_unfocused", Kind::Bool, "stats", "pauseTotal"),
    def("pause_session_when_unfocused", Kind::Bool, "stats", "pauseSession"),
    def("achievements_bypass", Kind::Bool, "stats", "achievementsBypass"),
    def("allow_unknown_stats", Kind::Bool, "stats", "allowUnknownStats"),
    def("enable_experimental_overlay", Kind::Bool, "overlay", "enableExperimentalOverlay"),
    hint("hook_delay_sec", Kind::Int, "overlay", "hookDelay", "0"),
    hint("key_combo", Kind::Text, "overlay", "keyCombo", "shift + tab"),
    hint("screenshot_combo", Kind::Text, "overlay", "screenshotCombo", "f12"),
    def("enable_screenshot", Kind::Invert, "overlay", "disableScreenshot"),
    def("overlay_always_show_fps", Kind::Bool, "overlay", "overlayAlwaysShowFps"),
    def("overlay_always_show_frametime", Kind::Bool, "overlay", "overlayAlwaysShowFrametime"),
    def("overlay_always_show_playtime", Kind::Bool, "overlay", "overlayAlwaysShowPlaytime"),
    def("overlay_always_show_user_info", Kind::Bool, "overlay", "overlayAlwaysShowUserInfo"),
    def("disable_achievement_notification", Kind::Bool, "overlay", "disableAchievementNotification"),
    def("disable_achievement_progress", Kind::Invert, "overlay", "showAchievementProgress"),
    def("disable_friend_notification", Kind::Bool, "overlay", "disableFriendNotification"),
    def("disable_warning_any", Kind::Bool, "overlay", "disableWarnings"),
    hint("font_size", Kind::Float, "overlay", "fontSize", "16"),
    hint("notification_duration_achievement", Kind::Float, "overlay", "notificationDuration", "7"),
    def("force_steamhttp_success", Kind::Bool, "compat", "forceSteamhttpSuccess"),
    def("enable_steam_preowned_ids", Kind::Bool, "compat", "enableSteamPreownedIds"),
    def("free_weekend", Kind::Bool, "compat", "freeWeekend"),
];

const STATIC_KEYS: [(&str, Kind); 2] = [("controller_type", Kind::Text), ("disable_steamoverlaygameid_env_var", Kind::Bool)];

pub fn field_id(key: &str) -> String {
    if key == "controller_type" {
        "emu-controller-type".to_string()
    } else {
        format!("emu-set-{key}")
    }
}

fn all_keys() -> impl Iterator<Item = (&'static str, Kind)> {
    STATIC_KEYS.into_iter().chain(SETTINGS.iter().map(|def| (def.key, if def.kind == Kind::Select { Kind::Int } else { def.kind })))
}

fn field_value(app: &App, key: &str, kind: Kind) -> Option<Value> {
    let id = field_id(key);
    match kind {
        Kind::Bool => app.ui.checked(&id).then_some(json!(true)),
        Kind::Invert => app.ui.checked(&id).then_some(json!(false)),
        _ => {
            let raw = app.ui.value(&id).trim();
            if raw.is_empty() {
                return None;
            }
            match kind {
                Kind::Int | Kind::Select => {
                    let digits: String = raw.chars().take_while(|c| c.is_ascii_digit()).collect();
                    digits.parse::<u64>().ok().map(|n| json!(n))
                }
                Kind::Float => raw.parse::<f64>().ok().filter(|n| n.is_finite()).map(|n| json!(n)),
                _ => Some(json!(raw)),
            }
        }
    }
}

pub fn gather(app: &App) -> Option<Value> {
    let mut settings = Map::new();
    for (key, kind) in all_keys() {
        if let Some(value) = field_value(app, key, kind) {
            settings.insert(key.to_string(), value);
        }
    }
    (!settings.is_empty()).then_some(Value::Object(settings))
}

pub fn custom_count(app: &App) -> usize {
    all_keys().filter(|(key, kind)| field_value(app, key, *kind).is_some()).count()
}

pub fn populate(app: &mut App, settings: &Value) {
    let mut values = settings.clone();
    if values["steam_deck"] == true && values["steam_hardware_type"].is_null() {
        values["steam_hardware_type"] = json!(1);
    }
    for (key, kind) in all_keys() {
        let id = field_id(key);
        match kind {
            Kind::Bool => app.ui.set_checked(&id, values[key] == true),
            Kind::Invert => app.ui.set_checked(&id, values[key] == false),
            _ => {
                let text = match &values[key] {
                    Value::Null => String::new(),
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                };
                app.ui.set_value(&id, text);
            }
        }
    }
}

pub fn load_last(app: &App) -> Option<Value> {
    app.store.get(LAST_SETTINGS_KEY).and_then(|raw| serde_json::from_str(&raw).ok())
}

pub fn save_last(app: &mut App, settings: Option<&Value>) {
    let text = settings.cloned().unwrap_or_else(|| json!({})).to_string();
    app.store.set(LAST_SETTINGS_KEY, &text);
}

fn is_changed(app: &App, def: &Def) -> bool {
    field_value(app, def.key, if def.kind == Kind::Select { Kind::Int } else { def.kind }).is_some()
}

fn field_text(app: &App, def: &Def) -> String {
    let mut text = format!("{} {}", app.i18n.t(&format!("emulator.fields.{}", def.label)), app.i18n.t(&format!("emulator.fields.{}Hint", def.label)));
    if def.kind == Kind::Select {
        for (_, label) in HARDWARE_OPTIONS {
            text.push(' ');
            text.push_str(app.i18n.t(&format!("emulator.fields.{label}")));
        }
    }
    text.to_lowercase()
}

fn view_state(app: &App) -> (String, bool, &'static str) {
    let query = app.ui.value("emu-settings-search").trim().to_lowercase();
    let only_changed = app.ui.checked("emu-settings-changed");
    (query, only_changed, app.state.emu.settings_group.unwrap_or(GROUPS[0]))
}

fn visible(app: &App, def: &Def, query: &str, only_changed: bool, group: &str) -> bool {
    let matches = if query.is_empty() { only_changed || def.group == group } else { field_text(app, def).contains(query) || def.key.contains(query) };
    matches && (!only_changed || is_changed(app, def))
}

pub fn shown_count(app: &App) -> usize {
    let (query, only_changed, group) = view_state(app);
    SETTINGS.iter().filter(|def| visible(app, def, &query, only_changed, group)).count()
}

fn fill_groups(app: &mut App, dom: &mut Dom<'_>) {
    let (query, only_changed, group) = view_state(app);
    let mut picked = None;
    for name in GROUPS {
        let active = query.is_empty() && !only_changed && name == group;
        let count = SETTINGS.iter().filter(|def| def.group == name && is_changed(app, def)).count();
        let clicked = dom
            .button(el("button.emu-settings__group").class_if("is-active", active), |dom| {
                dom.el("span", |dom| dom.text(app.i18n.t(&format!("emulator.sections.{name}"))));
                dom.el(el("span.emu-settings__count").class_if("hidden", count == 0), |dom| {
                    if count > 0 {
                        dom.text(count.to_string());
                    }
                });
            })
            .clicked();
        if clicked {
            picked = Some(name);
        }
    }
    if let Some(name) = picked {
        app.state.emu.settings_group = Some(name);
        app.ui.set_value("emu-settings-search", "");
        app.ui.set_checked("emu-settings-changed", false);
    }
}

fn fill_fields(app: &mut App, dom: &mut Dom<'_>) {
    let (query, only_changed, group) = view_state(app);
    let mut shown = 0;
    for def in &SETTINGS {
        let is_visible = visible(app, def, &query, only_changed, group);
        if is_visible {
            shown += 1;
        }
        let toggle = matches!(def.kind, Kind::Bool | Kind::Invert);
        let id: &'static str = crate::dom::spec::intern(&field_id(def.key));
        let label = t!(app, &format!("emulator.fields.{}", def.label));
        let hint = t!(app, &format!("emulator.fields.{}Hint", def.label));
        let row = el("div.emu-field").class_if("emu-field--toggle", toggle).class_if("hidden", !is_visible).key(def.key);
        dom.el(row, |dom| {
            if toggle {
                dom.el("div.emu-field__label-block", |dom| {
                    dom.el("span.emu-field__label", |dom| dom.text(&label));
                    dom.el("p.emu-field__hint", |dom| dom.text(&hint));
                });
                let checked = app.ui.checked(id);
                let clicked = dom
                    .button(el("label.toggle-switch").state(HAS_CHECKED, checked).label(label.clone()), |dom| {
                        dom.empty(el("input[type=checkbox]").attr("id", id).state(CHECKED, checked));
                        dom.empty("span.toggle-switch__slider");
                    })
                    .clicked();
                if clicked {
                    app.toggle(id);
                }
            } else {
                dom.el("div.emu-field__label-block", |dom| {
                    if dom.button(el("label.emu-field__label").attr("for", id), |dom| dom.text(&label)).clicked() {
                        app.ui.request_focus(id);
                    }
                    if def.kind == Kind::Select {
                        if !app.state.select_options.contains_key(id) {
                            let options = HARDWARE_OPTIONS.iter().map(|(value, name)| (value.to_string(), t!(app, &format!("emulator.fields.{name}")))).collect();
                            select_picker::set_options(app, id, options);
                        }
                        select_picker::render(app, dom, id, &[]);
                    } else {
                        let placeholder = if def.placeholder_key.is_empty() { def.placeholder.to_string() } else { t!(app, &format!("emulator.fields.{}", def.placeholder_key)) };
                        let element = el(if def.kind == Kind::Text { "input.emu-field__input[type=text]" } else { "input.emu-field__input.emu-field__input--narrow[type=number]" }).attr("id", id);
                        let response = dom.input_with(element, app.ui.value_mut(id), &placeholder, false);
                        app.input_events(id, &response);
                    }
                    dom.el("p.emu-field__hint", |dom| dom.text(&hint));
                });
            }
        });
    }
    app.ui.set_class("emu-settings-fields", "emu-settings__fields--mixed", !query.is_empty() || only_changed);
    app.ui.set_hidden("emu-settings-empty", shown > 0);
}

pub fn language_changed(app: &mut App) {
    let id = field_id("steam_hardware_type");
    app.state.select_options.remove(&id);
}

fn reset_fields(app: &mut App) {
    for def in &SETTINGS {
        let id = field_id(def.key);
        app.ui.set_checked(&id, false);
        app.ui.set_value(&id, "");
    }
    let done = t!(app, "emulator.settingsResetDone");
    ui::show_toast(app, done, "success", 4000);
}

pub fn click(app: &mut App, id: &str) -> bool {
    match id {
        "btn-emu-settings-reset" => reset_fields(app),
        _ => return false,
    }
    true
}

pub fn input(app: &mut App, id: &str) -> bool {
    if id == "emu-settings-search" {
        app.state.emu.search_timer += 1;
        let token = app.state.emu.search_timer;
        let tasks = app.tasks.clone();
        app.tasks.spawn(async move {
            tasks.sleep(1500).await;
            tasks.post(move |app| {
                if app.state.emu.search_timer != token || app.ui.value("emu-settings-search").trim().is_empty() {
                    return;
                }
                let shown = shown_count(app);
                app.emit_event("emu_settings_view", json!({ "action": "search", "results": telemetry::count_bucket(shown) }));
            });
        });
        return true;
    }
    id.starts_with("emu-set-")
}

pub fn changed(app: &mut App, id: &str) -> bool {
    if id == "emu-settings-changed" {
        if app.ui.checked(id) {
            app.emit_event("emu_settings_view", json!({ "action": "changed_only" }));
        }
        return true;
    }
    id.starts_with("emu-set-") || id == "emu-controller-type"
}

pub fn fill(app: &mut App, id: &'static str, dom: &mut Dom<'_>) -> bool {
    match id {
        "emu-settings-groups" => fill_groups(app, dom),
        "emu-settings-fields" => fill_fields(app, dom),
        _ => return false,
    }
    true
}
