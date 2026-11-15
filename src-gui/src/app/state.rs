use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use serde_json::Value;

use super::App;

pub type Action = Rc<dyn Fn(&mut App)>;
pub type Answer = Box<dyn FnOnce(&mut App, bool)>;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Depot {
    pub depot_id: String,
    pub manifest_id: String,
    pub depot_key: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct ParsedData {
    pub main_app_id: String,
    pub depots: Vec<Depot>,
    pub all_app_ids: Vec<String>,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Repo {
    pub name: String,
    pub date: Option<String>,
    pub sha: Option<String>,
    pub kind: String,
    pub source: String,
}

pub struct NextSteps {
    pub title: String,
    pub text: String,
    pub error: bool,
    pub actions: Vec<(String, Action)>,
}

#[derive(Default)]
pub struct Autocomplete {
    pub loading: bool,
    pub results: Option<Vec<Value>>,
}

#[derive(Default)]
pub struct State {
    pub current_step: u8,
    pub update_dir: Option<String>,
    pub update_app_id: Option<String>,
    pub mode: String,
    pub account_mode: bool,
    pub parsed_data: Option<ParsedData>,
    pub selected_depots: std::collections::BTreeSet<String>,
    pub job_id: Option<String>,
    pub game_name: Option<String>,
    pub game_denuvo: bool,
    pub header_image: Option<String>,
    pub download_dir: Option<String>,
    pub shortcut_supported: bool,
    pub shortcuts_created: bool,
    pub download_failed: bool,
    pub emulator_available: bool,
    pub emu_edit_mode: bool,
    pub emu_standalone: bool,
    pub steam_library_supported: bool,
    pub search_repos: Vec<Repo>,
    pub selected_repo: Option<usize>,
    pub search_app_id: Option<u64>,
    pub search_repo: Option<String>,
    pub search_sha: Option<String>,
    pub search_key_vdf_keys: Option<Value>,
    pub playtests: Vec<Value>,
    pub pending_sources: Option<Vec<String>>,
    pub pristine_sources: Option<Vec<String>>,
    pub saved_sources: Vec<String>,
    pub boot_next: Option<Box<dyn FnOnce(&mut App)>>,
    pub auto_redownload_pending: bool,
    pub default_download_dir: String,
    pub dl_nonce: Option<String>,
    pub dl_source_count: Option<usize>,
    pub dl_had_mh_key: bool,
    pub dl_context: Option<Value>,
    pub dl_started_at: Option<std::time::Instant>,
    pub dl_stage: Option<String>,
    pub dl_depot_count: Option<usize>,
    pub current_engine: Option<String>,
    pub telemetry_focused: Option<bool>,
    pub reported_errors: HashSet<String>,
    pub parsed_serial: u64,
    pub timer_serial: u64,
    pub reported_denuvo: HashSet<String>,
    pub depot_order: Vec<String>,
    pub depot_auto_pending: bool,
    pub depot_selection_touched: bool,
    pub depot_pics_list: Option<Vec<Value>>,
    pub depot_skip_reasons: HashMap<String, String>,
    pub depot_include_dlc: Option<bool>,
    pub depot_manifests: HashMap<String, (String, String)>,
    pub depot_names: HashMap<String, String>,
    pub depot_name_shown: HashMap<String, String>,
    pub depot_badge_shown: HashSet<String>,
    pub depot_pics_info: HashMap<String, Value>,
    pub depot_tags: HashMap<String, Vec<(String, String)>>,
    pub depot_custom: HashMap<String, String>,
    pub depot_latest: HashSet<String>,
    pub depot_status: HashMap<String, String>,
    pub depot_status_timer: HashMap<String, u64>,
    pub depot_fetching: HashSet<String>,
    pub depot_empty_note: HashMap<String, String>,
    pub depot_nokey_removed: HashSet<String>,
    pub mark_keyless: bool,
    pub steam_keys_asked: Option<String>,
    pub steam_mode: bool,
    pub auto_start_timer: Option<u64>,
    pub latest_on_select: bool,
    pub queue_running: bool,
    pub repair_running: bool,
    pub repair_manifests: Option<Value>,
    pub auto_select_all_on_step2: bool,
    pub auto_select_depot_ids: Option<Vec<String>>,
    pub notification_sound_enabled: bool,
    pub progress_listening: bool,
    pub paused: bool,
    pub last_skipped_shown: u64,
    pub depot_progress: Vec<super::logic::progress::DepotProgress>,
    pub terminal: Vec<(&'static str, String)>,
    pub terminal_scroll: bool,
    pub speed: super::logic::progress::SpeedTracker,
    pub taskbar: super::logic::progress::Taskbar,
    pub suggest_mh_key: bool,
    pub pending_history_entry: Option<Value>,
    pub download_started_at: Option<String>,
    pub last_failure_text: Option<String>,
    pub mh_hint: Option<String>,
    pub shutdown_timer: Option<u64>,
    pub shutdown_remaining: u32,
    pub resume_answer: Option<Box<dyn FnOnce(&mut App, Option<&'static str>)>>,
    pub followup: Option<Value>,
    pub dotnet_warning_dismissed: bool,
    pub steam_library_user: Option<Value>,
    pub steam_library_error: Option<String>,
    pub steam_detected_exes: Vec<Value>,
    pub shortcut_detected_exes: Vec<Value>,
    pub steam_running_prompt: bool,
    pub queue: Vec<Value>,
    pub queue_current: Option<Value>,
    pub queue_timer: Option<u64>,
    pub account: super::logic::account::AccountUi,
    pub emu: super::logic::emulator::EmuState,
    pub settings: super::logic::settings::SettingsUi,
    pub history: super::logic::history::HistoryUi,
    pub remote: super::logic::remote::RemoteUi,
    pub journey: super::logic::journey::Journey,
    pub whats_new_url: Option<String>,
    pub bug_report_source: &'static str,
    pub pending_update: Option<Value>,
    pub update_commands: Vec<(String, String)>,
    pub update_copied: Option<usize>,
    pub update_copy_timer: u64,
    pub update_listening: bool,
    pub history_banner_timer: u64,
    pub picker_focus: HashMap<&'static str, usize>,
    pub select_options: HashMap<String, Vec<(String, String)>>,
    pub open_select: Option<String>,
    pub open_select_options: Vec<(String, String)>,
    pub select_focus: usize,
    pub select_scroll: bool,
    pub parked: HashMap<u64, Box<dyn std::any::Any>>,
    pub next_steps: HashMap<&'static str, NextSteps>,
    pub autocomplete: HashMap<&'static str, Autocomplete>,
    pub autocomplete_timer: u64,
    pub toast_timer: u64,
    pub answers: HashMap<&'static str, Answer>,
    pub focus_return: HashMap<&'static str, Option<egui::Id>>,
    pub open_dialogs: Vec<&'static str>,
}

pub struct Parked<T> {
    id: u64,
    kind: std::marker::PhantomData<fn() -> T>,
}

impl<T> Clone for Parked<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Parked<T> {}

impl State {
    pub fn park<T: 'static>(&mut self, value: T) -> Parked<T> {
        self.timer_serial += 1;
        let id = self.timer_serial;
        self.parked.insert(id, Box::new(value));
        Parked { id, kind: std::marker::PhantomData }
    }

    pub fn unpark<T: 'static>(&mut self, token: Parked<T>) -> Option<T> {
        self.parked.remove(&token.id).and_then(|value| value.downcast::<T>().ok()).map(|value| *value)
    }

    pub fn set_parsed(&mut self, data: Option<ParsedData>) {
        self.parsed_serial += 1;
        self.parsed_data = data;
    }
}
