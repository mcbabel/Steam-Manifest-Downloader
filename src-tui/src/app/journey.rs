use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;
use ratatui::layout::Position;
use smd_core::services::settings::TelemetryConsent;
use smd_core::services::telemetry::sanitize_props;

use super::action::{Action, SourceTab, Step};
use super::modal::Modal;
use super::{App, Page};

const MAX_ENTRIES: usize = 40;

pub struct Screen {
    name: String,
    visit: u32,
    entered: Instant,
    last: Instant,
    first: Option<Instant>,
    longest: Duration,
    clicks: BTreeMap<String, u32>,
    dead: BTreeMap<String, u32>,
    rage: BTreeMap<String, u32>,
    keys: BTreeMap<String, u32>,
}

#[derive(Default)]
pub struct Journey {
    screen: Option<Screen>,
    visits: HashMap<String, u32>,
    last_click: Option<(Position, Instant, u32)>,
    pub acted: bool,
}

fn bump(map: &mut BTreeMap<String, u32>, name: &str) {
    if map.len() < MAX_ENTRIES || map.contains_key(name) {
        *map.entry(name.to_string()).or_default() += 1;
    }
}

fn step_order(name: &str) -> u32 {
    match name.split('.').next().unwrap_or("") {
        "source" => 1,
        "select" => 2,
        "download" => 3,
        "shortcut" => 4,
        "library" => 5,
        "emulator" => 6,
        _ => 0,
    }
}

pub fn action_label(action: &Action) -> String {
    let text = format!("{:?}", action);
    let end = text.find(['(', ' ', '{']).unwrap_or(text.len());
    text[..end].to_string()
}

pub fn key_label(code: KeyCode) -> String {
    match code {
        KeyCode::Char(_) => "char".into(),
        KeyCode::F(n) => format!("f{}", n),
        other => format!("{:?}", other).to_lowercase(),
    }
}

impl App {
    fn journey_on(&self) -> bool {
        self.telemetry.is_some()
            && self.settings.telemetry_consent == TelemetryConsent::Accepted
            && self.settings.telemetry_behavior == TelemetryConsent::Accepted
    }

    fn screen_name(&self) -> String {
        if let Some(m) = &self.modal {
            return match m {
                Modal::Confirm(_) => "confirm",
                Modal::Browser(_) => "browser",
                Modal::Telemetry => "telemetry",
                Modal::Behavior => "behavior",
                Modal::Language(_) => "language",
                Modal::Update(_) => "update",
                Modal::Help => "help",
                Modal::Welcome => "welcome",
                Modal::WhatsNew(_) => "whats-new",
            }
            .into();
        }
        match self.page {
            Page::Wizard => match self.wiz.step {
                Step::Source => format!(
                    "source.{}",
                    match self.wiz.tab {
                        SourceTab::Upload => "upload",
                        SourceTab::Search => "search",
                        SourceTab::PatchOnly => "patch-only",
                    }
                ),
                Step::Select => "select".into(),
                Step::Progress => "download".into(),
                Step::Shortcut => "shortcut".into(),
                Step::SteamLibrary => "library".into(),
                Step::Emulator => "emulator".into(),
            },
            Page::History => "history".into(),
            Page::Settings => format!("settings.{}", self.set.tab),
        }
    }

    fn journey_finish(&mut self, next: &str) -> Option<serde_json::Value> {
        let s = self.journey.screen.take()?;
        let now = Instant::now();
        let dwell = now.duration_since(s.entered).as_secs();
        if dwell < 1 && s.clicks.is_empty() && s.dead.is_empty() {
            return None;
        }
        let back = step_order(next) > 0 && step_order(next) < step_order(&s.name);
        Some(serde_json::json!({
            "screen": s.name,
            "next": next,
            "visit": s.visit,
            "dwell_s": dwell,
            "first_action_s": s.first.map(|f| f.duration_since(s.entered).as_secs()),
            "longest_pause_s": s.longest.max(now.duration_since(s.last)).as_secs(),
            "back": u8::from(back),
            "clicks": s.clicks,
            "dead": s.dead,
            "rage": s.rage,
            "keys": s.keys,
        }))
    }

    pub(super) fn journey_check(&mut self) {
        if !self.journey_on() {
            self.journey.screen = None;
            return;
        }
        let name = self.screen_name();
        if self.journey.screen.as_ref().is_some_and(|s| s.name == name) {
            return;
        }
        if let Some(props) = self.journey_finish(&name) {
            self.emit("ui_screen", Some(sanitize_props("ui_screen", props)));
        }
        let visit = {
            let v = self.journey.visits.entry(name.clone()).or_default();
            *v += 1;
            *v
        };
        let now = Instant::now();
        self.journey.screen = Some(Screen {
            name,
            visit,
            entered: now,
            last: now,
            first: None,
            longest: Duration::ZERO,
            clicks: BTreeMap::new(),
            dead: BTreeMap::new(),
            rage: BTreeMap::new(),
            keys: BTreeMap::new(),
        });
    }

    fn journey_touch(&mut self) {
        if let Some(s) = self.journey.screen.as_mut() {
            let now = Instant::now();
            s.longest = s.longest.max(now.duration_since(s.last));
            s.first.get_or_insert(now);
            s.last = now;
        }
    }

    pub(super) fn journey_action(&mut self, action: &Action) {
        self.journey.acted = true;
        let label = action_label(action);
        if let Some(s) = self.journey.screen.as_mut() {
            bump(&mut s.clicks, &label);
        }
        self.journey_touch();
    }

    pub(super) fn journey_click(&mut self, pos: Position, hit: Option<&Action>) {
        if self.journey.screen.is_none() {
            return;
        }
        let label = hit.map(action_label).unwrap_or_else(|| "empty".into());
        let now = Instant::now();
        let repeat = match self.journey.last_click {
            Some((p, at, n)) if p == pos && now.duration_since(at) < Duration::from_millis(800) => n + 1,
            _ => 1,
        };
        self.journey.last_click = Some((pos, now, repeat));
        if let Some(s) = self.journey.screen.as_mut() {
            if hit.is_none() {
                bump(&mut s.dead, "empty");
            }
            if repeat == 3 {
                bump(&mut s.rage, &label);
            }
        }
        self.journey_touch();
    }

    pub(super) fn journey_key(&mut self, code: KeyCode, typing: bool) {
        if typing {
            self.journey_touch();
            return;
        }
        let unbound = !self.journey.acted
            && matches!(code, KeyCode::Char(_) | KeyCode::F(_) | KeyCode::Enter);
        if let Some(s) = self.journey.screen.as_mut() {
            if unbound {
                bump(&mut s.keys, &key_label(code));
            }
        }
        self.journey_touch();
    }

    pub(super) fn journey_final(&mut self) -> Option<serde_json::Value> {
        let props = self.journey_on().then(|| self.journey_finish("exit")).flatten()?;
        Some(sanitize_props("ui_screen", props))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_carry_no_payload() {
        assert_eq!(action_label(&Action::OpenPlaytest(1000)), "OpenPlaytest");
        assert_eq!(action_label(&Action::Quit), "Quit");
        assert_eq!(key_label(KeyCode::Char('x')), "char");
        assert_eq!(key_label(KeyCode::F(5)), "f5");
    }

    #[test]
    fn going_to_an_earlier_step_counts_as_back() {
        assert!(step_order("select") > step_order("source.search"));
        assert_eq!(step_order("settings.2"), 0);
    }
}
