use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::Instant;

use smd_core::ops::emulator::DlcMergePlan;
use smd_core::ops::search::StoreSearchHit;
use smd_core::ops::shortcuts::DetectedExecutable;
use smd_core::services::depot_info;
use smd_core::services::emulator::{EmuSettings, ReleaseInfo, ScannedFile};
use smd_core::services::history::HistoryEntry;
use smd_core::services::multi_repo_search::RepoResult;
use smd_core::services::settings::Settings;
use smd_core::services::steam_pics::{DepotMetadata, DepotRole};
use smd_core::services::steamless::DrmScanEntry;

use super::action::{AcTarget, SourceTab, Step};
use crate::ui::input::TextInput;
use crate::ui::widgets::Tone;

#[derive(Debug, Clone)]
pub struct DepotRow {
    pub depot_id: String,
    pub manifest_id: String,
    pub depot_key: Option<String>,
    pub size_bytes: Option<u64>,
    pub custom_manifest: TextInput,
    pub uploaded_manifest: Option<PathBuf>,
    pub status: Option<(Tone, String)>,
    pub fetching: bool,
}

impl DepotRow {
    pub fn new(
        depot_id: String,
        manifest_id: Option<String>,
        key: Option<String>,
        size: Option<u64>,
    ) -> Self {
        DepotRow {
            depot_id,
            manifest_id: manifest_id.unwrap_or_else(|| "N/A".into()),
            depot_key: key,
            size_bytes: size,
            custom_manifest: TextInput::default(),
            uploaded_manifest: None,
            status: None,
            fetching: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub main_app_id: String,
    pub depots: Vec<DepotRow>,
    pub all_app_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Stdout,
    Stderr,
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub kind: LogKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepotState {
    Pending,
    Active,
    Done,
    Error,
}

#[derive(Debug, Clone)]
pub struct DepotProgress {
    pub depot_id: String,
    pub state: DepotState,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Finished {
    pub success: bool,
    pub message: String,
    pub tone: Tone,
}

#[derive(Debug, Clone)]
pub struct ProgressState {
    pub status: String,
    pub overall: f64,
    pub depot_ratio: f64,
    pub depot_text: String,
    pub speed: Option<String>,
    pub eta: Option<String>,
    pub speed_history: VecDeque<u64>,
    pub byte_samples: VecDeque<(Instant, u64, u64)>,
    pub last_speed_display: Option<Instant>,
    pub last_update: Instant,
    pub current_depot: Option<String>,
    pub current_depot_size: u64,
    pub depots: Vec<DepotProgress>,
    pub log: Vec<LogLine>,
    pub log_offset: usize,
    pub log_follow: bool,
    pub finished: Option<Finished>,
    pub paused: bool,
    pub cancelling: bool,
    pub disk: Option<(f64, String)>,
    pub engine_native: bool,
    pub stage: String,
    pub last_skipped_shown: u64,
    pub suggest_mh_key: bool,
    pub started: Instant,
    pub results: Vec<serde_json::Value>,
    pub selected_depot: usize,
    pub depot_offset: usize,
    pub percent_samples: VecDeque<(Instant, f64)>,
}

impl ProgressState {
    pub fn new(depot_ids: &[String], native: bool) -> Self {
        ProgressState {
            status: String::new(),
            overall: 0.0,
            depot_ratio: 0.0,
            depot_text: "0%".into(),
            speed: None,
            eta: None,
            speed_history: VecDeque::new(),
            byte_samples: VecDeque::new(),
            last_speed_display: None,
            last_update: Instant::now(),
            current_depot: None,
            current_depot_size: 0,
            depots: depot_ids
                .iter()
                .map(|id| DepotProgress {
                    depot_id: id.clone(),
                    state: DepotState::Pending,
                    text: String::new(),
                })
                .collect(),
            log: Vec::new(),
            log_offset: 0,
            log_follow: true,
            finished: None,
            paused: false,
            cancelling: false,
            disk: None,
            engine_native: native,
            stage: "starting".into(),
            last_skipped_shown: 0,
            suggest_mh_key: false,
            started: Instant::now(),
            results: Vec::new(),
            selected_depot: 0,
            depot_offset: 0,
            percent_samples: VecDeque::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Autocomplete {
    pub target: Option<AcTarget>,
    pub items: Vec<StoreSearchHit>,
    pub selected: Option<usize>,
    pub loading: bool,
    pub pending: Option<(String, Instant)>,
    pub last_query: String,
}

pub struct Wizard {
    pub step: Step,
    pub tab: SourceTab,
    pub from_search: bool,

    pub upload_path: TextInput,
    pub upload_loading: bool,
    pub upload_error: Option<String>,

    pub search_input: TextInput,
    pub search_loading: bool,
    pub search_error: Option<String>,
    pub repos: Vec<RepoResult>,
    pub repo_selected: usize,
    pub repo_offset: usize,
    pub repo_chosen: Option<usize>,
    pub manifests_loading: bool,
    pub search_app_id: Option<String>,
    pub search_repo: Option<String>,
    pub search_key_vdf: Option<HashMap<String, String>>,
    pub source_type: Option<String>,
    pub source_add: TextInput,
    pub source_error: Option<String>,

    pub patch_dir: TextInput,
    pub patch_app_id: TextInput,
    pub patch_error: Option<String>,
    pub patch_loading: bool,

    pub ac: Autocomplete,

    pub parsed: Option<Parsed>,
    pub game_name: Option<String>,
    pub header_image: Option<String>,
    pub short_description: Option<String>,
    pub game_info_loading: bool,

    pub selected: HashSet<String>,
    pub depot_cursor: usize,
    pub depot_offset: usize,
    pub depot_filter: TextInput,
    pub show_selected_only: bool,
    pub depot_meta: HashMap<String, depot_info::DepotInfo>,
    pub depot_pics: HashMap<String, DepotMetadata>,
    pub mh_key: TextInput,
    pub mh_hint: Option<String>,
    pub download_dir: TextInput,
    pub select_scroll: u16,
    pub auto_select: Option<Option<Vec<String>>>,
    pub auto_redownload: bool,
    pub update_dir: Option<String>,
    pub update_app_id: Option<String>,
    pub repair_manifests: Option<HashMap<String, String>>,

    pub job_id: Option<String>,
    pub result_dir: Option<String>,
    pub progress: Option<ProgressState>,
    pub download_started_at: Option<String>,

    pub exes: Vec<DetectedExecutable>,
    pub exe_selected: usize,
    pub exes_loading: bool,
    pub shortcut_exe: TextInput,
    pub shortcut_desktop: bool,
    pub shortcut_start_menu: bool,
    pub shortcut_steam: bool,
    pub shortcut_status: Option<(Tone, String)>,
    pub shortcuts_done: bool,
    pub shortcut_busy: bool,

    pub steam_exe: TextInput,
    pub steam_name: TextInput,
    pub steam_launch: TextInput,
    pub steam_status: Option<(Tone, String)>,
    pub steam_done: bool,
    pub steam_busy: bool,
    pub steam_running: bool,

    pub post_scroll: u16,
}

impl Wizard {
    pub fn active_repair(&self, app_id: &str) -> Option<&HashMap<String, String>> {
        self.active_update_dir(app_id)
            .and(self.repair_manifests.as_ref())
    }

    pub fn active_update_dir(&self, app_id: &str) -> Option<String> {
        match (&self.update_dir, &self.update_app_id) {
            (Some(dir), Some(id)) if id == app_id => Some(dir.clone()),
            _ => None,
        }
    }

    pub fn new(settings: &Settings, mh_key: &str) -> Self {
        Wizard {
            step: Step::Source,
            tab: SourceTab::Upload,
            from_search: false,
            upload_path: TextInput::default(),
            upload_loading: false,
            upload_error: None,
            search_input: TextInput::default(),
            search_loading: false,
            search_error: None,
            repos: Vec::new(),
            repo_selected: 0,
            repo_offset: 0,
            repo_chosen: None,
            manifests_loading: false,
            search_app_id: None,
            search_repo: None,
            search_key_vdf: None,
            source_type: None,
            source_add: TextInput::default(),
            source_error: None,
            patch_dir: TextInput::default(),
            patch_app_id: TextInput::default(),
            patch_error: None,
            patch_loading: false,
            ac: Autocomplete::default(),
            parsed: None,
            game_name: None,
            header_image: None,
            short_description: None,
            game_info_loading: false,
            selected: HashSet::new(),
            depot_cursor: 0,
            depot_offset: 0,
            depot_filter: TextInput::default(),
            show_selected_only: false,
            depot_meta: HashMap::new(),
            depot_pics: HashMap::new(),
            mh_key: TextInput::new(mh_key).masked(),
            mh_hint: None,
            download_dir: TextInput::new(settings.download_location.clone()),
            select_scroll: 0,
            auto_select: None,
            auto_redownload: false,
            update_dir: None,
            update_app_id: None,
            repair_manifests: None,
            job_id: None,
            result_dir: None,
            progress: None,
            download_started_at: None,
            exes: Vec::new(),
            exe_selected: 0,
            exes_loading: false,
            shortcut_exe: TextInput::default(),
            shortcut_desktop: true,
            shortcut_start_menu: true,
            shortcut_steam: false,
            shortcut_status: None,
            shortcuts_done: false,
            shortcut_busy: false,
            steam_exe: TextInput::default(),
            steam_name: TextInput::default(),
            steam_launch: TextInput::default(),
            steam_status: None,
            steam_done: false,
            steam_busy: false,
            steam_running: false,
            post_scroll: 0,
        }
    }

    pub fn main_app_id(&self) -> Option<String> {
        self.parsed
            .as_ref()
            .map(|p| p.main_app_id.clone())
            .filter(|s| !s.is_empty())
            .or_else(|| self.search_app_id.clone())
    }

    pub fn visible_depots(&self) -> Vec<usize> {
        let Some(parsed) = &self.parsed else {
            return Vec::new();
        };
        let filter = self.depot_filter.trimmed().to_lowercase();
        let mut idx: Vec<usize> = (0..parsed.depots.len())
            .filter(|&i| {
                let d = &parsed.depots[i];
                if self.show_selected_only && !self.selected.contains(&d.depot_id) {
                    return false;
                }
                if filter.is_empty() {
                    return true;
                }
                let pics = self.depot_pics.get(&d.depot_id);
                let hay = [
                    Some(d.depot_id.clone()),
                    pics.and_then(|p| p.name.clone()),
                    pics.map(|p| role_name(p.role).to_string()),
                    pics.and_then(|p| p.oslist.clone()),
                    pics.and_then(|p| p.language.clone()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
                hay.contains(&filter)
            })
            .collect();
        idx.sort_by_key(|&i| {
            let d = &parsed.depots[i];
            (
                sort_key(self.depot_pics.get(&d.depot_id)),
                d.depot_id.parse::<u64>().unwrap_or(u64::MAX),
            )
        });
        idx
    }
}

pub fn role_name(role: DepotRole) -> &'static str {
    match role {
        DepotRole::Platform => "platform",
        DepotRole::SharedContent => "shared_content",
        DepotRole::Dlc => "dlc",
        DepotRole::Language => "language",
        DepotRole::Other => "other",
    }
}

fn host_os() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        _ => "windows",
    }
}

fn sort_key(info: Option<&DepotMetadata>) -> u32 {
    let Some(info) = info else { return 100 };
    match info.role {
        DepotRole::SharedContent => 10,
        DepotRole::Platform => {
            let os = info.oslist.clone().unwrap_or_default().to_lowercase();
            if os.contains(host_os()) {
                20
            } else if os.contains("windows") {
                30
            } else if os.contains("linux") {
                31
            } else if os.contains("mac") {
                32
            } else {
                33
            }
        }
        DepotRole::Language => 50,
        DepotRole::Dlc => 60,
        DepotRole::Other => 80,
    }
}

pub struct EmuState {
    pub game_dir: String,
    pub app_id: String,
    pub standalone: bool,
    pub edit_mode: bool,
    pub scan: Vec<ScannedFile>,
    pub selected: HashSet<String>,
    pub selection_explicit: bool,
    pub patched: HashSet<String>,
    pub edit_targets: Vec<ScannedFile>,
    pub settings_prefill_path: Option<String>,
    pub experimental: bool,
    pub bypass: bool,
    pub bypass_initial: bool,
    pub section: usize,
    pub fields: Vec<TextInput>,
    pub bools: HashMap<&'static str, bool>,
    pub release: Option<Result<ReleaseInfo, String>>,
    pub merge_plan: Option<DlcMergePlan>,
    pub merge_status: Option<(Tone, String)>,
    pub merge_busy: bool,
    pub drm: Vec<DrmScanEntry>,
    pub drm_status: Option<(Tone, String)>,
    pub drm_busy: bool,
    pub status: Option<(Tone, String)>,
    pub av_blocked: bool,
    pub busy: bool,
    pub apply_complete: bool,
    pub allow_download: bool,
    pub scroll: u16,
}

impl EmuState {
    pub fn new(game_dir: String, app_id: String) -> Self {
        EmuState {
            game_dir,
            app_id,
            standalone: false,
            edit_mode: false,
            scan: Vec::new(),
            selected: HashSet::new(),
            selection_explicit: false,
            patched: HashSet::new(),
            edit_targets: Vec::new(),
            settings_prefill_path: None,
            experimental: false,
            bypass: false,
            bypass_initial: false,
            section: 0,
            fields: super::emulator::TEXT_FIELDS
                .iter()
                .map(|_| TextInput::default())
                .collect(),
            bools: HashMap::new(),
            release: None,
            merge_plan: None,
            merge_status: None,
            merge_busy: false,
            drm: Vec::new(),
            drm_status: None,
            drm_busy: false,
            status: None,
            av_blocked: false,
            busy: false,
            apply_complete: false,
            allow_download: false,
            scroll: 0,
        }
    }

    pub fn populate(&mut self, s: &EmuSettings) {
        let value = serde_json::to_value(s).unwrap_or_default();
        for (i, f) in super::emulator::TEXT_FIELDS.iter().enumerate() {
            let v = value.get(f.key).cloned().unwrap_or_default();
            let text = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            self.fields[i].set(text);
        }
        self.bools.clear();
        for f in super::emulator::BOOL_FIELDS {
            if value.get(f.key).and_then(|v| v.as_bool()) == Some(true) {
                self.bools.insert(f.key, true);
            }
        }
    }

    pub fn gather(&self) -> Option<EmuSettings> {
        let mut map = serde_json::Map::new();
        for (i, f) in super::emulator::TEXT_FIELDS.iter().enumerate() {
            let raw = self.fields[i].trimmed();
            if raw.is_empty() {
                continue;
            }
            if f.float {
                if let Ok(n) = raw.parse::<f64>() {
                    map.insert(f.key.into(), serde_json::json!(n));
                }
            } else {
                map.insert(f.key.into(), serde_json::Value::String(raw));
            }
        }
        for f in super::emulator::BOOL_FIELDS {
            if self.bools.get(f.key).copied().unwrap_or(false) {
                map.insert(f.key.into(), serde_json::Value::Bool(true));
            }
        }
        if map.is_empty() {
            return None;
        }
        serde_json::from_value(serde_json::Value::Object(map)).ok()
    }

    pub fn selected_targets(&self) -> Vec<ScannedFile> {
        if !self.selection_explicit && self.selected.is_empty() {
            return self.scan.clone();
        }
        self.scan
            .iter()
            .filter(|f| self.selected.contains(&f.path))
            .cloned()
            .collect()
    }

    pub fn refresh_patched(&mut self, scan: Vec<ScannedFile>) {
        self.scan = scan;
        self.edit_targets = self.scan.iter().filter(|f| f.is_patched).cloned().collect();
        self.patched = self.edit_targets.iter().map(|f| f.path.clone()).collect();
    }
}

pub struct HistoryState {
    pub entries: Vec<HistoryEntry>,
    pub loading: bool,
    pub selected: usize,
    pub offset: usize,
    pub filter: TextInput,
    pub banner: Option<(Tone, String, Instant)>,
    pub pending_remove: Option<(String, bool)>,
    pub updates: HashSet<String>,
    pub updates_checked: Option<(String, Instant)>,
}

impl HistoryState {
    pub fn new() -> Self {
        HistoryState {
            entries: Vec::new(),
            loading: false,
            selected: 0,
            offset: 0,
            filter: TextInput::default(),
            banner: None,
            pending_remove: None,
            updates: HashSet::new(),
            updates_checked: None,
        }
    }

    pub fn visible(&self) -> Vec<usize> {
        let f = self.filter.trimmed().to_lowercase();
        (0..self.entries.len())
            .filter(|&i| {
                if f.is_empty() {
                    return true;
                }
                let e = &self.entries[i];
                format!(
                    "{} {} {}",
                    e.game_name.clone().unwrap_or_default(),
                    e.app_id,
                    e.status
                )
                .to_lowercase()
                .contains(&f)
            })
            .collect()
    }
}

pub struct SettingsPage {
    pub tab: usize,
    pub draft: Settings,
    pub inputs: Vec<TextInput>,
    pub source_add: TextInput,
    pub source_error: Option<String>,
    pub source_selected: usize,
    pub source_offset: usize,
    pub status: Option<(Tone, String)>,
    pub scroll: u16,
    pub dirty: bool,
}

pub const SETTING_DOWNLOAD_DIR: usize = 0;
pub const SETTING_CHUNKS: usize = 1;
pub const SETTING_RETRIES: usize = 2;
pub const SETTING_DD_ARGS: usize = 3;
pub const SETTING_SPEED: usize = 4;
pub const SETTING_PROXY: usize = 5;
pub const SETTING_HUBCAP: usize = 6;
pub const SETTING_RYUU: usize = 7;
pub const SETTING_MH: usize = 8;

impl SettingsPage {
    pub fn new(settings: &Settings, mh_key: &str) -> Self {
        let mut page = SettingsPage {
            tab: 0,
            draft: settings.clone(),
            inputs: Vec::new(),
            source_add: TextInput::default(),
            source_error: None,
            source_selected: 0,
            source_offset: 0,
            status: None,
            scroll: 0,
            dirty: false,
        };
        page.load(settings, mh_key);
        page
    }

    pub fn load(&mut self, s: &Settings, mh_key: &str) {
        self.draft = s.clone();
        self.inputs = vec![
            TextInput::new(s.download_location.clone()),
            TextInput::new(s.native_chunk_concurrency.to_string()),
            TextInput::new(s.max_retries.to_string()),
            TextInput::new(s.dd_extra_args.join(" ")),
            TextInput::new(s.download_speed_limit.clone()),
            TextInput::new(s.proxy.clone()),
            TextInput::new(s.hubcap_api_key.clone()).masked(),
            TextInput::new(s.ryuu_api_key.clone()).masked(),
            TextInput::new(mh_key).masked(),
        ];
        self.dirty = false;
    }
}
