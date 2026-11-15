use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use serde_json::json;

use super::ui::topmost_visible_dialog;
use crate::app::App;

const MAX_NAMES: usize = 40;

pub struct Screen {
    name: String,
    visit: u32,
    entered: Instant,
    last: Instant,
    first: Option<Instant>,
    longest: f64,
    clicks: BTreeMap<String, u32>,
    dead: BTreeMap<String, u32>,
    rage: BTreeMap<String, u32>,
}

#[derive(Default)]
pub struct Journey {
    active: bool,
    screen: Option<Screen>,
    visits: HashMap<String, u32>,
    last_click: Option<(String, Instant, u32)>,
    clicked: bool,
    hidden: bool,
}

fn journey_label(text: &str) -> String {
    let mut out = String::new();
    let mut digits = String::new();
    let flush = |digits: &mut String, out: &mut String| {
        if digits.len() >= 3 {
            out.push('n');
        } else {
            out.push_str(digits);
        }
        digits.clear();
    };
    for chr in text.chars() {
        if chr.is_ascii_digit() {
            digits.push(chr);
            continue;
        }
        flush(&mut digits, &mut out);
        if chr.is_ascii_alphanumeric() || matches!(chr, '_' | '.' | '-') {
            out.push(chr);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    flush(&mut digits, &mut out);
    let label: String = out.trim_matches('-').chars().take(64).collect();
    if label.is_empty() {
        "unknown".to_string()
    } else {
        label
    }
}

fn step_order(name: &str) -> u32 {
    match name.split('.').next().unwrap_or("") {
        "source" => 1,
        "select" => 2,
        "download" => 3,
        "shortcut" => 4,
        "emulator" => 5,
        "library" => 6,
        _ => 0,
    }
}

fn screen_name(app: &App) -> String {
    if let Some(dialog) = topmost_visible_dialog(app) {
        if dialog == "settings-modal" {
            let tab = ["general", "downloads", "depots", "sources", "steam", "privacy"].into_iter().find(|tab| app.ui.has_class(&format!("settings-tab-{tab}"), "is-active")).unwrap_or("general");
            return format!("settings.{tab}");
        }
        return dialog.trim_end_matches("-modal").to_string();
    }
    match app.state.current_step {
        1 => format!("source.{}", if app.state.mode.is_empty() { "upload" } else { &app.state.mode }),
        2 => "select".to_string(),
        3 => "download".to_string(),
        4 => "shortcut".to_string(),
        5 => {
            let tab = super::emulator::SECTIONS.into_iter().find(|tab| app.ui.has_class(&format!("emu-tab-{tab}"), "is-active")).unwrap_or("files");
            format!("emulator.{tab}")
        }
        6 => "library".to_string(),
        _ => "start".to_string(),
    }
}

fn new_screen(app: &mut App, name: String) -> Screen {
    let visit = app.state.journey.visits.entry(name.clone()).or_insert(0);
    *visit += 1;
    let now = Instant::now();
    Screen { name, visit: *visit, entered: now, last: now, first: None, longest: 0.0, clicks: BTreeMap::new(), dead: BTreeMap::new(), rage: BTreeMap::new() }
}

fn bump(map: &mut BTreeMap<String, u32>, name: &str) {
    if map.len() >= MAX_NAMES && !map.contains_key(name) {
        return;
    }
    *map.entry(name.to_string()).or_insert(0) += 1;
}

fn action(app: &mut App) {
    let Some(screen) = app.state.journey.screen.as_mut() else { return };
    let now = Instant::now();
    screen.longest = screen.longest.max(now.duration_since(screen.last).as_secs_f64());
    screen.first.get_or_insert(now);
    screen.last = now;
}

fn finish(app: &mut App, next: &str) {
    let Some(screen) = app.state.journey.screen.take() else { return };
    let now = Instant::now();
    let dwell = now.duration_since(screen.entered).as_secs_f64().round() as u64;
    if dwell < 1 && screen.clicks.is_empty() && screen.dead.is_empty() {
        return;
    }
    let back = step_order(next) > 0 && step_order(&screen.name) > 0 && step_order(next) < step_order(&screen.name);
    app.emit_event(
        "ui_screen",
        json!({
            "screen": screen.name,
            "next": next,
            "visit": screen.visit,
            "dwell_s": dwell,
            "first_action_s": screen.first.map(|first| first.duration_since(screen.entered).as_secs_f64().round() as u64),
            "longest_pause_s": screen.longest.max(now.duration_since(screen.last).as_secs_f64()).round() as u64,
            "back": u8::from(back),
            "clicks": screen.clicks,
            "dead": screen.dead,
            "rage": screen.rage,
            "hovers": {},
        }),
    );
}

fn check_screen(app: &mut App) {
    let name = screen_name(app);
    if app.state.journey.screen.as_ref().is_some_and(|screen| screen.name == name) {
        return;
    }
    finish(app, &name);
    let screen = new_screen(app, name);
    app.state.journey.screen = Some(screen);
}

fn record(app: &mut App, name: &str, interactive: bool) {
    if !app.state.journey.active || app.state.journey.screen.is_none() {
        return;
    }
    check_screen(app);
    let name = journey_label(name);
    let now = Instant::now();
    let repeated = match &mut app.state.journey.last_click {
        Some((last, at, count)) if *last == name && now.duration_since(*at).as_millis() < 800 => {
            *count += 1;
            *at = now;
            *count == 3
        }
        other => {
            *other = Some((name.clone(), now, 1));
            false
        }
    };
    if let Some(screen) = app.state.journey.screen.as_mut() {
        bump(if interactive { &mut screen.clicks } else { &mut screen.dead }, &name);
        if repeated {
            bump(&mut screen.rage, &name);
        }
    }
    action(app);
}

pub fn click(app: &mut App, name: &str) {
    app.state.journey.clicked = true;
    record(app, name, true);
}

pub fn journey_note(app: &mut App, name: &str) {
    if !app.state.journey.active {
        return;
    }
    let label = journey_label(name);
    if let Some(screen) = app.state.journey.screen.as_mut() {
        bump(&mut screen.clicks, &label);
    }
}

pub fn start_journey(app: &mut App) {
    if app.state.journey.active {
        return;
    }
    app.state.journey.active = true;
    let name = screen_name(app);
    let screen = new_screen(app, name);
    app.state.journey.screen = Some(screen);
}

pub fn stop_journey(app: &mut App) {
    if !app.state.journey.active {
        return;
    }
    finish(app, "off");
    app.state.journey.active = false;
}

pub fn exit(app: &mut App) {
    if app.state.journey.active {
        finish(app, "exit");
    }
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    let clicked = std::mem::take(&mut app.state.journey.clicked);
    if !app.state.journey.active {
        return;
    }
    let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
    if minimized != app.state.journey.hidden {
        app.state.journey.hidden = minimized;
        if minimized {
            finish(app, "background");
        } else {
            let name = screen_name(app);
            let screen = new_screen(app, name);
            app.state.journey.screen = Some(screen);
        }
    }
    if minimized {
        return;
    }
    check_screen(app);
    if ctx.input(|i| i.events.iter().any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))) {
        action(app);
    }
    if !clicked && ctx.input(|i| i.pointer.primary_clicked()) {
        if let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
            let under = app.rects.iter().filter(|(_, rect)| rect.contains(pointer)).min_by(|a, b| a.1.area().total_cmp(&b.1.area())).map(|(name, _)| name.to_string());
            record(app, under.as_deref().unwrap_or("body"), false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_names_and_orders_steps() {
        assert_eq!(journey_label("history-play:12345"), "history-play-n");
        assert_eq!(journey_label("btn-search"), "btn-search");
        assert_eq!(journey_label("///"), "unknown");
        assert_eq!(journey_label("depot 48"), "depot-48");
        assert_eq!(step_order("emulator.files"), 5);
        assert_eq!(step_order("history"), 0);
    }
}
