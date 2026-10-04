pub mod action;
mod emulator;
mod history;
mod modal;
mod post;
mod prefs;
mod progress;
mod select;
mod settings;
mod source;
pub mod state;

use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{
    Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use futures::StreamExt;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use smd_core::services::events::{EventSink, Sink, DOWNLOAD_PROGRESS, EMU_DOWNLOAD_PROGRESS};
use smd_core::services::settings::{self as settings_service, Settings};
use smd_core::services::steam_library::SteamInstall;
use smd_core::services::telemetry::{Event as TelemetryEvent, Telemetry};
use smd_core::services::AppState;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::i18n::{self, t};
use crate::term::Tui;
use crate::theme;
use crate::ui::input::{InputOutcome, TextInput};
use crate::ui::widgets::{self, Tone};
use crate::ui::{Ctx, Fid, Kind};
use action::{Action, InputId, ListId, Page, ScrollTarget, Step};
pub use modal::Modal;
use prefs::Prefs;
use state::{EmuState, HistoryState, SettingsPage, Wizard};

pub const VERSION: &str = env!("SMD_APP_VERSION");
const MIN_WIDTH: u16 = 70;
const MIN_HEIGHT: u16 = 20;
const TICK: Duration = Duration::from_millis(100);

pub type Apply = Box<dyn FnOnce(&mut App) + Send>;

pub fn apply(f: impl FnOnce(&mut App) + Send + 'static) -> Apply {
    Box::new(f)
}

pub enum Msg {
    Event(String, serde_json::Value),
    Apply(Apply),
}

struct ChannelSink(UnboundedSender<Msg>);

impl EventSink for ChannelSink {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        let _ = self.0.send(Msg::Event(channel.to_string(), payload));
    }
}

pub struct App {
    pub core: Arc<AppState>,
    pub data_dir: PathBuf,
    pub sink: Sink,
    tx: UnboundedSender<Msg>,
    pub settings: Settings,
    pub prefs: Prefs,
    pub telemetry: Option<Telemetry>,

    pub page: Page,
    pub wiz: Wizard,
    pub emu: Option<EmuState>,
    pub hist: HistoryState,
    pub set: SettingsPage,
    pub modal: Option<Modal>,
    modal_queue: std::collections::VecDeque<Modal>,

    pub focus: Option<Fid>,
    pub hover: Option<Position>,
    ctx: Ctx,
    last_click: Option<(Position, Instant)>,
    manual_scroll: bool,
    pub tick: u64,
    pub quit: bool,
    pub toast: Option<(Tone, String, Instant)>,

    pub shortcut_supported: bool,
    pub steam_install: Option<SteamInstall>,
    pub emulator_available: bool,
    pub shutdown_after: bool,
    pub shutdown_deadline: Option<Instant>,
    pub followup: Option<smd_core::services::followup::PendingFollowup>,
}

impl App {
    pub async fn new(data_dir: PathBuf) -> (Self, UnboundedReceiver<Msg>) {
        let (tx, rx) = unbounded_channel();
        let _ = tokio::fs::create_dir_all(&data_dir).await;
        let settings = settings_service::seed_defaults_if_needed(&data_dir).await;
        let prefs = Prefs::load(&data_dir).await;

        let lang = if settings.language.is_empty() {
            i18n::detect_system_language()
        } else {
            i18n::normalize(&settings.language).unwrap_or(i18n::FALLBACK)
        };
        i18n::set_language(lang);
        theme::apply(prefs.theme);

        let channel = option_env!("SMD_BUILD_CHANNEL")
            .unwrap_or("dev-local")
            .to_string();
        let telemetry = Telemetry::new(data_dir.clone(), VERSION.to_string(), channel);
        tokio::spawn(telemetry.clone().run_background_flush());

        let mut core = AppState::new();
        core.telemetry = Some(telemetry.clone());

        let wiz = Wizard::new(&settings, &prefs.mh_api_key);
        let set = SettingsPage::new(&settings, &prefs.mh_api_key);
        let app = App {
            core: Arc::new(core),
            data_dir,
            sink: Arc::new(ChannelSink(tx.clone())),
            tx,
            settings,
            prefs,
            telemetry: Some(telemetry),
            page: Page::Wizard,
            wiz,
            emu: None,
            hist: HistoryState::new(),
            set,
            modal: None,
            modal_queue: Default::default(),
            focus: None,
            hover: None,
            ctx: Ctx::default(),
            last_click: None,
            manual_scroll: false,
            tick: 0,
            quit: false,
            toast: None,
            shortcut_supported: smd_core::ops::shortcuts::is_shortcut_supported(),
            steam_install: None,
            emulator_available: false,
            shutdown_after: false,
            shutdown_deadline: None,
            followup: None,
        };
        (app, rx)
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = Apply> + Send + 'static,
    {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let done = fut.await;
            let _ = tx.send(Msg::Apply(done));
        });
    }

    pub fn emit(&self, kind: &str, props: Option<serde_json::Value>) {
        if let Some(tel) = self.telemetry.clone() {
            let mut event = TelemetryEvent::new(kind);
            if let Some(p) = props {
                event = event.with_props(p);
            }
            tokio::spawn(async move { tel.emit(event).await });
        }
    }

    pub fn toast(&mut self, tone: Tone, msg: impl Into<String>) {
        self.toast = Some((tone, msg.into(), Instant::now()));
    }

    pub fn reload_settings(&mut self) {
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let s = settings_service::load_settings(&dir).await;
            apply(move |app| app.settings = s)
        });
    }

    fn startup(&mut self) {
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let status = smd_core::ops::consent::telemetry_status(&dir).await;
            apply(move |app| match status["consent"].as_str() {
                Some("pending") => app.queue_modal(Modal::telemetry()),
                Some("accepted") => app.emit("app_start", None),
                _ => {}
            })
        });
        if self.settings.language.is_empty() {
            self.queue_modal(Modal::language(i18n::language()));
        }
        self.spawn(async {
            let install =
                tokio::task::spawn_blocking(smd_core::services::steam_library::detect_steam)
                    .await
                    .ok()
                    .and_then(Result::ok);
            apply(move |app| app.steam_install = install)
        });
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let followup = smd_core::services::followup::load(&dir).await;
            apply(move |app| {
                if let Some(f) = followup {
                    app.offer_followup(f);
                }
            })
        });
        if self.settings.auto_update {
            self.check_updates(false);
        }
    }

    pub fn check_updates(&mut self, manual: bool) {
        self.spawn(async move {
            let result = smd_core::ops::updater::check_for_updates(VERSION).await;
            apply(move |app| app.on_update_checked(result, manual))
        });
    }

    fn on_update_checked(&mut self, result: Result<serde_json::Value, String>, manual: bool) {
        match result {
            Ok(info) => {
                let available = info["available"].as_bool().unwrap_or(false);
                self.emit(
                    "update_checked",
                    Some(serde_json::json!({ "available": available })),
                );
                if !available {
                    if manual {
                        self.toast(Tone::Success, t("tui.update.upToDate"));
                    }
                    return;
                }
                let version = info["version"].as_str().unwrap_or_default().to_string();
                if !manual && self.prefs.skipped_update.as_deref() == Some(version.as_str()) {
                    return;
                }
                self.queue_modal(Modal::update(&info));
            }
            Err(e) => {
                if manual {
                    self.toast(Tone::Error, e);
                }
            }
        }
    }

    pub fn queue_modal(&mut self, m: Modal) {
        if self.modal.is_none() {
            self.modal = Some(m);
            self.focus = None;
        } else {
            self.modal_queue.push_back(m);
        }
    }

    pub fn show_modal(&mut self, m: Modal) {
        if let Some(open) = self.modal.take() {
            self.modal_queue.push_front(open);
        }
        self.modal = Some(m);
        self.focus = None;
    }

    pub fn close_modal(&mut self) {
        self.modal = self.modal_queue.pop_front();
        self.focus = None;
    }

    pub fn download_active(&self) -> bool {
        self.wiz.job_id.is_some()
            && self
                .wiz
                .progress
                .as_ref()
                .map(|p| p.finished.is_none())
                .unwrap_or(false)
    }

    pub fn request_quit(&mut self) {
        if self.download_active() {
            self.queue_modal(Modal::confirm(
                t("modals.close.title"),
                t("modals.close.body"),
                t("modals.close.yes"),
                t("modals.close.no"),
                true,
                Action::ForceQuit,
            ));
        } else {
            self.quit = true;
        }
    }

    pub async fn run(
        mut self,
        terminal: &mut Tui,
        mut rx: UnboundedReceiver<Msg>,
    ) -> std::io::Result<()> {
        self.startup();
        let mut events = EventStream::new();
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        self.draw(terminal)?;
        while !self.quit {
            tokio::select! {
                ev = events.next() => match ev {
                    Some(Ok(ev)) => self.on_event(ev),
                    Some(Err(_)) | None => self.quit = true,
                },
                msg = rx.recv() => {
                    if let Some(m) = msg {
                        self.on_msg(m);
                        let mut n = 0;
                        while let Ok(m) = rx.try_recv() {
                            self.on_msg(m);
                            n += 1;
                            if n > 2000 {
                                break;
                            }
                        }
                    }
                }
                _ = ticker.tick() => self.on_tick(),
            }
            self.draw(terminal)?;
        }
        self.shutdown().await;
        Ok(())
    }

    async fn shutdown(&mut self) {
        if let Some(job) = self.wiz.job_id.clone() {
            if self.download_active() {
                let _ = smd_core::ops::download::cancel_download(
                    &self.sink,
                    &self.core,
                    &self.data_dir,
                    job,
                )
                .await;
                if !self.settings.cancel_keep_files {
                    tokio::time::sleep(Duration::from_secs(4)).await;
                }
            }
        }
        if let Some(tel) = self.telemetry.clone() {
            let _ = tokio::time::timeout(Duration::from_secs(3), tel.flush()).await;
        }
    }

    fn on_msg(&mut self, msg: Msg) {
        match msg {
            Msg::Apply(f) => f(self),
            Msg::Event(channel, payload) => {
                if channel == DOWNLOAD_PROGRESS {
                    self.on_download_event(payload);
                } else if channel == EMU_DOWNLOAD_PROGRESS {
                    self.on_emu_download_event(payload);
                }
            }
        }
    }

    fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if let Some((_, _, at)) = &self.toast {
            if at.elapsed() > Duration::from_secs(5) {
                self.toast = None;
            }
        }
        if let Some((_, _, at)) = &self.hist.banner {
            if at.elapsed() > Duration::from_secs(6) {
                self.hist.banner = None;
            }
        }
        self.autocomplete_tick();
        self.progress_tick();
        self.shutdown_tick();
    }

    fn on_event(&mut self, ev: Event) {
        match ev {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(key),
            Event::Mouse(m) => self.on_mouse(m),
            Event::Paste(text) => self.on_paste(text),
            _ => {}
        }
    }

    fn focused_entry(&self) -> Option<crate::ui::Focusable> {
        let f = self.focus?;
        self.ctx.find(f).cloned()
    }

    pub fn focus_next(&mut self, dir: i32) {
        let list = &self.ctx.focusables;
        if list.is_empty() {
            return;
        }
        let n = list.len() as i32;
        let cur = self
            .focus
            .and_then(|f| list.iter().position(|x| x.fid == f));
        let next = match cur {
            Some(i) => (i as i32 + dir).rem_euclid(n),
            None if dir > 0 => 0,
            None => n - 1,
        };
        self.focus = Some(list[next as usize].fid);
    }

    pub fn focus_input(&mut self, id: InputId) {
        if let Some(f) = self
            .ctx
            .focusables
            .iter()
            .find(|f| matches!(f.kind, Kind::Input(i) if i == id))
        {
            self.focus = Some(f.fid);
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if !matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            self.manual_scroll = false;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('q')) {
            self.request_quit();
            return;
        }

        if self.modal.is_none() {
            match key.code {
                KeyCode::F(1) => return self.dispatch(Action::Help),
                KeyCode::F(2) => return self.dispatch(Action::Nav(Page::Wizard)),
                KeyCode::F(3) => return self.dispatch(Action::Nav(Page::History)),
                KeyCode::F(4) => return self.dispatch(Action::Nav(Page::Settings)),
                KeyCode::F(10) => return self.request_quit(),
                _ => {}
            }
        }

        let focused = self.focused_entry();
        if let Some(f) = &focused {
            match &f.kind {
                Kind::Input(id) => {
                    let id = *id;
                    if self.autocomplete_key(id, key) {
                        return;
                    }
                    let outcome = match self.input_mut(id) {
                        Some(input) => input.handle_key(key),
                        None => InputOutcome::Ignored,
                    };
                    match outcome {
                        InputOutcome::Changed => {
                            self.on_input_changed(id);
                            return;
                        }
                        InputOutcome::Moved => return,
                        InputOutcome::Submit => {
                            match self.submit_action(id) {
                                Some(a) => self.dispatch(a),
                                None => self.focus_next(1),
                            }
                            return;
                        }
                        InputOutcome::Ignored => {}
                    }
                }
                Kind::List(id) => {
                    if self.list_key(*id, key) {
                        return;
                    }
                }
                Kind::Button => {}
            }
        }

        match key.code {
            KeyCode::Tab => self.focus_next(1),
            KeyCode::BackTab => self.focus_next(-1),
            KeyCode::Down | KeyCode::Right => self.focus_next(1),
            KeyCode::Up | KeyCode::Left => self.focus_next(-1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(a) = focused.and_then(|f| f.action) {
                    self.dispatch(a);
                }
            }
            KeyCode::Esc => self.back(),
            KeyCode::PageDown => self.dispatch(Action::Scroll(ScrollTarget::Page, 10)),
            KeyCode::PageUp => self.dispatch(Action::Scroll(ScrollTarget::Page, -10)),
            KeyCode::Char(c) if !ctrl => self.shortcut_key(c),
            _ => {}
        }
    }

    fn shortcut_key(&mut self, c: char) {
        if let Some(m) = &self.modal {
            if let Some(a) = m.shortcut(c) {
                self.dispatch(a);
            }
            return;
        }
        if c == '?' {
            return self.dispatch(Action::Help);
        }
        match self.page {
            Page::Wizard => match self.wiz.step {
                Step::Select => match c {
                    'a' => self.dispatch(Action::SelectAll),
                    'n' => self.dispatch(Action::DeselectAll),
                    '/' => self.focus_input(InputId::DepotFilter),
                    'd' => self.dispatch(Action::StartDownload),
                    _ => {}
                },
                Step::Progress => match c {
                    'p' => self.dispatch(Action::PauseResume),
                    'c' => self.dispatch(Action::AskCancel),
                    _ => {}
                },
                _ => {}
            },
            Page::History => {
                let sel = self.hist.selected;
                match c {
                    'r' => self.dispatch(Action::HistoryResume(sel)),
                    'd' => self.dispatch(Action::HistoryRedownload(sel)),
                    'u' => self.dispatch(Action::HistoryUpdate(sel)),
                    'o' => self.dispatch(Action::HistoryOpenFolder(sel)),
                    'e' => self.dispatch(Action::HistoryEditEmu(sel)),
                    'x' => self.dispatch(Action::AskHistoryRemove(sel)),
                    '/' => self.focus_input(InputId::HistoryFilter),
                    _ => {}
                }
            }
            Page::Settings => {
                if c == 's' {
                    self.dispatch(Action::SettingsSave);
                }
            }
        }
    }

    fn back(&mut self) {
        if self.modal.is_some() {
            let a = self
                .modal
                .as_ref()
                .and_then(|m| m.cancel_action())
                .unwrap_or(Action::CloseModal);
            return self.dispatch(a);
        }
        if self.wiz.ac.target.is_some() {
            self.wiz.ac = Default::default();
            return;
        }
        match self.page {
            Page::History | Page::Settings => self.dispatch(Action::Nav(Page::Wizard)),
            Page::Wizard => match self.wiz.step {
                Step::Select => self.dispatch(Action::BackToSource),
                Step::Emulator
                    if self
                        .emu
                        .as_ref()
                        .is_some_and(|e| e.standalone || e.edit_mode) =>
                {
                    self.dispatch(Action::Home)
                }
                _ => {}
            },
        }
    }

    fn submit_action(&self, id: InputId) -> Option<Action> {
        Some(match id {
            InputId::UploadPath => Action::LoadUpload,
            InputId::Search => Action::SearchSubmit,
            InputId::SourceAdd => Action::AddSource,
            InputId::PatchAppId | InputId::PatchDir => Action::PatchStart,
            InputId::SettingsSourceAdd => Action::SettingsAddSource,
            InputId::BrowserPath => Action::BrowserChoose,
            InputId::DepotManifest(i) => Action::FetchLatest(i),
            _ => return None,
        })
    }

    pub fn input_mut(&mut self, id: InputId) -> Option<&mut TextInput> {
        let w = &mut self.wiz;
        Some(match id {
            InputId::UploadPath => &mut w.upload_path,
            InputId::Search => &mut w.search_input,
            InputId::SourceAdd => &mut w.source_add,
            InputId::PatchDir => &mut w.patch_dir,
            InputId::PatchAppId => &mut w.patch_app_id,
            InputId::DepotFilter => &mut w.depot_filter,
            InputId::MhKey => &mut w.mh_key,
            InputId::DownloadDir => &mut w.download_dir,
            InputId::DepotManifest(i) => &mut w.parsed.as_mut()?.depots.get_mut(i)?.custom_manifest,
            InputId::ShortcutExe => &mut w.shortcut_exe,
            InputId::SteamExe => &mut w.steam_exe,
            InputId::SteamName => &mut w.steam_name,
            InputId::SteamLaunch => &mut w.steam_launch,
            InputId::EmuField(i) => self.emu.as_mut()?.fields.get_mut(i)?,
            InputId::Setting(i) => self.set.inputs.get_mut(i)?,
            InputId::SettingsSourceAdd => &mut self.set.source_add,
            InputId::HistoryFilter => &mut self.hist.filter,
            InputId::BrowserPath => match self.modal.as_mut()? {
                Modal::Browser(fb) => &mut fb.path_input,
                _ => return None,
            },
        })
    }

    fn on_input_changed(&mut self, id: InputId) {
        match id {
            InputId::Search => self.autocomplete_typed(action::AcTarget::Search),
            InputId::PatchAppId => self.autocomplete_typed(action::AcTarget::PatchOnly),
            InputId::DepotFilter => {
                self.wiz.depot_cursor = 0;
                self.wiz.depot_offset = 0;
            }
            InputId::HistoryFilter => {
                self.hist.selected = 0;
                self.hist.offset = 0;
            }
            InputId::Setting(_) => self.set.dirty = true,
            _ => {}
        }
    }

    pub fn list_len(&self, id: ListId) -> usize {
        match id {
            ListId::Repos => self.wiz.repos.len(),
            ListId::Depots => self.wiz.visible_depots().len(),
            ListId::Log => self.wiz.progress.as_ref().map(|p| p.log.len()).unwrap_or(0),
            ListId::DepotProgress => self
                .wiz
                .progress
                .as_ref()
                .map(|p| p.depots.len())
                .unwrap_or(0),
            ListId::History => self.hist.visible().len(),
            ListId::SettingsSources => self.set.draft.depot_sources.len(),
            ListId::Browser => match &self.modal {
                Some(Modal::Browser(fb)) => fb.entries.len(),
                _ => 0,
            },
        }
    }

    fn list_selected_mut(&mut self, id: ListId) -> Option<&mut usize> {
        Some(match id {
            ListId::Repos => &mut self.wiz.repo_selected,
            ListId::Depots => &mut self.wiz.depot_cursor,
            ListId::DepotProgress => &mut self.wiz.progress.as_mut()?.selected_depot,
            ListId::History => &mut self.hist.selected,
            ListId::SettingsSources => &mut self.set.source_selected,
            ListId::Browser => match self.modal.as_mut()? {
                Modal::Browser(fb) => &mut fb.selected,
                _ => return None,
            },
            ListId::Log => return None,
        })
    }

    fn list_key(&mut self, id: ListId, key: KeyEvent) -> bool {
        let len = self.list_len(id);
        if id == ListId::Log {
            let delta = match key.code {
                KeyCode::Up => -1,
                KeyCode::Down => 1,
                KeyCode::PageUp => -10,
                KeyCode::PageDown => 10,
                KeyCode::Home => -1_000_000,
                KeyCode::End => 1_000_000,
                _ => return false,
            };
            self.scroll_log(delta);
            return true;
        }
        let Some(sel) = self.list_selected_mut(id).map(|s| *s) else {
            return false;
        };
        let new = match key.code {
            KeyCode::Up if sel > 0 => sel - 1,
            KeyCode::Up => return false,
            KeyCode::Down if sel + 1 < len => sel + 1,
            KeyCode::Down => return false,
            KeyCode::PageUp => sel.saturating_sub(10),
            KeyCode::PageDown => (sel + 10).min(len.saturating_sub(1)),
            KeyCode::Home => 0,
            KeyCode::End => len.saturating_sub(1),
            KeyCode::Enter if len > 0 => {
                self.dispatch(Action::ListActivate(id, sel));
                return true;
            }
            KeyCode::Char(' ') if len > 0 => {
                self.list_toggle(id, sel);
                return true;
            }
            KeyCode::Backspace if id == ListId::Browser => {
                self.dispatch(Action::BrowserUp);
                return true;
            }
            KeyCode::Delete if id == ListId::SettingsSources => {
                self.dispatch(Action::SettingsRemoveSource(sel));
                return true;
            }
            _ => return false,
        };
        self.dispatch(Action::ListSelect(id, new));
        true
    }

    fn list_toggle(&mut self, id: ListId, idx: usize) {
        match id {
            ListId::Depots => self.dispatch(Action::ToggleDepot(idx)),
            _ => self.dispatch(Action::ListActivate(id, idx)),
        }
    }

    fn list_select(&mut self, id: ListId, idx: usize) {
        if let Some(sel) = self.list_selected_mut(id) {
            *sel = idx;
        }
        if id == ListId::Repos {
            self.wiz.repo_chosen = Some(idx);
        }
    }

    fn list_activate(&mut self, id: ListId, idx: usize) {
        self.list_select(id, idx);
        match id {
            ListId::Repos => self.dispatch(Action::SearchNext),
            ListId::Depots => self.dispatch(Action::ToggleDepot(idx)),
            ListId::History => {
                let a = self.history_default_action(idx);
                self.dispatch(a);
            }
            ListId::SettingsSources => self.dispatch(Action::SettingsRemoveSource(idx)),
            ListId::Browser => self.browser_activate(idx),
            ListId::Log | ListId::DepotProgress => {}
        }
    }

    fn scroll(&mut self, target: ScrollTarget, delta: i32) {
        match target {
            ScrollTarget::List(ListId::Log) => self.scroll_log(delta as i64),
            ScrollTarget::List(id) => {
                let len = self.list_len(id);
                if let Some(sel) = self.list_selected_mut(id) {
                    let n = (*sel as i64 + delta as i64).clamp(0, len.saturating_sub(1) as i64);
                    *sel = n as usize;
                }
            }
            ScrollTarget::Page => {
                let off = self.page_scroll_mut();
                *off = (*off as i32 + delta).max(0) as u16;
                self.manual_scroll = true;
            }
        }
    }

    fn page_scroll_mut(&mut self) -> &mut u16 {
        match self.page {
            Page::Settings => &mut self.set.scroll,
            Page::History => &mut self.wiz.post_scroll,
            Page::Wizard => match self.wiz.step {
                Step::Select => &mut self.wiz.select_scroll,
                Step::Emulator => match self.emu.as_mut() {
                    Some(e) => &mut e.scroll,
                    None => &mut self.wiz.post_scroll,
                },
                _ => &mut self.wiz.post_scroll,
            },
        }
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        let pos = Position::new(m.column, m.row);
        match m.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.hover = Some(pos),
            MouseEventKind::Down(MouseButton::Left) => {
                self.hover = Some(pos);
                self.manual_scroll = false;
                let double = self
                    .last_click
                    .is_some_and(|(p, at)| p == pos && at.elapsed() < Duration::from_millis(450));
                self.last_click = Some((pos, Instant::now()));
                let Some(hit) = self.ctx.hit_at(pos).cloned() else {
                    return;
                };
                if let Some(fid) = hit.fid {
                    self.focus = Some(fid);
                }
                let action = match (double, hit.double) {
                    (true, Some(d)) => d,
                    _ => hit.action,
                };
                self.dispatch(action);
            }
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let delta = if m.kind == MouseEventKind::ScrollDown {
                    3
                } else {
                    -3
                };
                if let Some(target) = self.ctx.scroll_at(pos) {
                    let delta = if matches!(target, ScrollTarget::List(_)) {
                        delta / 3
                    } else {
                        delta
                    };
                    self.dispatch(Action::Scroll(target, delta));
                }
            }
            _ => {}
        }
    }

    fn on_paste(&mut self, text: String) {
        let clean = text
            .trim()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();
        let focused = self.focused_entry();
        if let Some(crate::ui::Focusable {
            kind: Kind::Input(id),
            ..
        }) = focused
        {
            if let Some(input) = self.input_mut(id) {
                input.insert_str(&text.replace(['\n', '\r'], " "));
            }
            self.on_input_changed(id);
            return;
        }
        let lower = clean.to_lowercase();
        if self.modal.is_none()
            && self.page == Page::Wizard
            && (lower.ends_with(".lua") || lower.ends_with(".st"))
            && std::path::Path::new(&clean).is_file()
        {
            self.wiz.tab = action::SourceTab::Upload;
            self.wiz.step = Step::Source;
            self.wiz.upload_path.set(clean);
            self.dispatch(Action::LoadUpload);
        }
    }

    pub fn dispatch(&mut self, action: Action) {
        match action {
            Action::Quit => self.request_quit(),
            Action::ForceQuit => {
                self.modal = None;
                self.quit = true;
            }
            Action::Help => self.queue_modal(Modal::Help),
            Action::CloseModal => self.close_modal(),
            Action::Nav(page) => self.navigate(page),
            Action::ListSelect(id, idx) => self.list_select(id, idx),
            Action::ListActivate(id, idx) => self.list_activate(id, idx),
            Action::Scroll(target, delta) => self.scroll(target, delta),
            Action::FocusInput(id, col) => {
                self.focus_input(id);
                if let Some(input) = self.input_mut(id) {
                    input.click(col);
                }
            }
            a => {
                if !self.dispatch_source(&a)
                    && !self.dispatch_select(&a)
                    && !self.dispatch_progress(&a)
                    && !self.dispatch_post(&a)
                    && !self.dispatch_emulator(&a)
                    && !self.dispatch_history(&a)
                    && !self.dispatch_settings(&a)
                {
                    self.dispatch_modal(a);
                }
            }
        }
    }

    fn navigate(&mut self, page: Page) {
        if self.page == page {
            return;
        }
        self.page = page;
        self.focus = None;
        match page {
            Page::History => self.load_history(),
            Page::Settings => {
                let s = self.settings.clone();
                self.set.load(&s, &self.prefs.mh_api_key.clone());
                self.set.status = None;
                self.emit("settings_opened", None);
            }
            Page::Wizard => {}
        }
    }

    fn draw(&mut self, terminal: &mut Tui) -> std::io::Result<()> {
        for _ in 0..2 {
            let mut cursor = None;
            terminal.draw(|f| {
                let area = f.area();
                let mut ctx = Ctx::new(self.focus, self.hover);
                ctx.follow_focus = !self.manual_scroll;
                self.render(f.buffer_mut(), area, &mut ctx);
                cursor = ctx.cursor;
                self.ctx = ctx;
                if let Some(c) = cursor {
                    f.set_cursor_position(c);
                }
            })?;
            let valid = self
                .focus
                .map(|f| self.ctx.find(f).is_some())
                .unwrap_or(false);
            if valid {
                break;
            }
            let next = self
                .ctx
                .preferred_focus
                .filter(|f| self.ctx.find(*f).is_some())
                .or_else(|| self.ctx.focusables.first().map(|f| f.fid));
            if next == self.focus {
                break;
            }
            self.focus = next;
        }
        Ok(())
    }

    fn render(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        widgets::fill(buf, area, theme::base());
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            let msg = i18n::tf(
                "tui.tooSmall",
                &[
                    ("w", &MIN_WIDTH),
                    ("h", &MIN_HEIGHT),
                    ("cw", &area.width),
                    ("ch", &area.height),
                ],
            );
            widgets::paragraph(buf, crate::ui::inset_h(area, 1), &msg, theme::warning());
            return;
        }
        let mut rest = area;
        let header = crate::ui::take_top(&mut rest, 3);
        let footer = crate::ui::take_bottom(&mut rest, 1);
        self.render_header(buf, header, ctx);
        let content = crate::ui::inset_h(
            Rect::new(
                rest.x,
                rest.y + 1,
                rest.width,
                rest.height.saturating_sub(1),
            ),
            2,
        );

        match self.page {
            Page::Wizard => match self.wiz.step {
                Step::Source => self.render_source(buf, content, ctx),
                Step::Select => self.render_select(buf, content, ctx),
                Step::Progress => self.render_progress(buf, content, ctx),
                Step::Shortcut => self.render_shortcut(buf, content, ctx),
                Step::SteamLibrary => self.render_steam_library(buf, content, ctx),
                Step::Emulator => self.render_emulator(buf, content, ctx),
            },
            Page::History => self.render_history(buf, content, ctx),
            Page::Settings => self.render_settings(buf, content, ctx),
        }

        self.render_footer(buf, footer, ctx);

        if self.modal.is_some() {
            ctx.begin_modal();
            self.render_modal(buf, area, ctx);
        }
        self.render_toast(buf, area);
    }

    fn render_header(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let th = theme::get();
        if th.truecolor {
            widgets::fill(
                buf,
                Rect::new(area.x, area.y, area.width, 2),
                Style::default().bg(th.surface),
            );
        }
        let title = "◆ Steam Manifest Downloader";
        let n = title.chars().count().max(2) as f32;
        let mut x = area.x + 2;
        for (i, ch) in title.chars().enumerate() {
            let style = Style::default()
                .fg(theme::gradient(i as f32 / (n - 1.0)))
                .add_modifier(Modifier::BOLD);
            buf.set_string(x, area.y, ch.to_string(), style);
            x += 1;
        }
        buf.set_string(x + 1, area.y, format!("v{}", VERSION), theme::muted());

        let tabs = [
            (Page::Wizard, t_key("tui.nav.download"), "F2"),
            (Page::History, t_key("tui.nav.history"), "F3"),
            (Page::Settings, t_key("tui.nav.settings"), "F4"),
        ];
        let mut items: Vec<(Option<Page>, String, &str)> =
            tabs.into_iter().map(|(p, l, k)| (Some(p), l, k)).collect();
        items.push((None, t("tui.nav.help"), "F1"));
        let widths: Vec<u16> = items
            .iter()
            .map(|(_, l, k)| widgets::width(l) + widgets::width(k) + 3)
            .collect();
        let total: u16 = widths.iter().sum::<u16>() + items.len() as u16;
        let mut nx = (area.x + area.width).saturating_sub(total + 1);
        for (i, (page, label, key)) in items.iter().enumerate() {
            let w = widths[i];
            let rect = Rect::new(nx, area.y, w, 1);
            let active = *page == Some(self.page);
            let hovered = ctx.is_hovered(rect);
            let style = if active {
                if th.truecolor {
                    Style::default()
                        .bg(th.accent)
                        .fg(th.on_accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(th.accent)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                }
            } else if hovered {
                theme::accent_bold()
            } else {
                theme::dim()
            };
            let key_style = if active { style } else { theme::muted() };
            buf.set_string(nx, area.y, " ", style);
            buf.set_string(nx + 1, area.y, key, key_style);
            buf.set_string(
                nx + 1 + widgets::width(key),
                area.y,
                format!(" {} ", label),
                style,
            );
            let action = match page {
                Some(p) => Action::Nav(*p),
                None => Action::Help,
            };
            ctx.click(rect, action);
            nx += w + 1;
        }

        let row = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), 1);
        if self.page == Page::Wizard {
            self.render_steps(buf, row);
        } else {
            let label = match self.page {
                Page::History => t("modals.history.title"),
                _ => t("settings.title"),
            };
            buf.set_string(
                row.x,
                row.y,
                label,
                theme::text().add_modifier(Modifier::BOLD),
            );
        }
        let rule = Rect::new(area.x, area.y + 2, area.width, 1);
        buf.set_string(
            rule.x,
            rule.y,
            "─".repeat(rule.width as usize),
            theme::border(false),
        );
    }

    fn visible_steps(&self) -> Vec<(Step, String)> {
        let mut steps = vec![
            (Step::Source, t("steps.upload")),
            (Step::Select, t("steps.select")),
            (Step::Progress, t("steps.download")),
        ];
        if self.shortcut_supported {
            steps.push((Step::Shortcut, t("steps.shortcut")));
        } else if self.steam_install.is_some() {
            steps.push((Step::SteamLibrary, t("steps.steamLibrary")));
        }
        steps.push((Step::Emulator, t("steps.emulator")));
        steps
    }

    fn render_steps(&self, buf: &mut Buffer, row: Rect) {
        let steps = self.visible_steps();
        let current = steps.iter().position(|(s, _)| *s == self.wiz.step);
        let skipped = self
            .emu
            .as_ref()
            .is_some_and(|e| e.standalone || e.edit_mode);
        let mut spans: Vec<Span> = Vec::new();
        for (i, (_, label)) in steps.iter().enumerate() {
            let done = !skipped && current.is_some_and(|c| i < c);
            let active = current == Some(i);
            let (mark, style) = if active {
                (
                    format!(" {} ", i + 1),
                    theme::accent_bold().add_modifier(Modifier::REVERSED),
                )
            } else if done {
                (" ✓ ".to_string(), theme::success())
            } else {
                (format!(" {} ", i + 1), theme::muted())
            };
            spans.push(Span::styled(mark, style));
            let label_style = if active {
                theme::accent_bold()
            } else if done {
                theme::text()
            } else {
                theme::muted()
            };
            spans.push(Span::styled(format!(" {}", label), label_style));
            if i + 1 < steps.len() {
                spans.push(Span::styled(" ── ", theme::border(false)));
            }
        }
        widgets::line(buf, row, Line::from(spans));
    }

    fn footer_hints(&self) -> Vec<(&'static str, String)> {
        let mut hints = vec![("Tab", t("tui.keys.next")), ("Enter", t("tui.keys.select"))];
        if self.modal.is_some() {
            hints.push(("Esc", t("common.close")));
            return hints;
        }
        hints.push(("Esc", t("common.back")));
        match (self.page, self.wiz.step) {
            (Page::Wizard, Step::Select) => {
                hints.push(("Space", t("tui.keys.toggle")));
                hints.push(("a/n", t("tui.keys.allNone")));
                hints.push(("/", t("tui.keys.filter")));
            }
            (Page::Wizard, Step::Progress) if self.download_active() => {
                hints.push(("p", t("progress.pause")));
                hints.push(("c", t("common.cancel")));
            }
            (Page::History, _) => {
                hints.push(("r/d/o/e/x", t("tui.keys.historyActions")));
            }
            _ => {}
        }
        hints.push(("F1", t("tui.nav.help")));
        hints.push(("^Q", t("tui.keys.quit")));
        hints
    }

    fn render_footer(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let t = theme::get();
        if t.truecolor {
            widgets::fill(buf, area, Style::default().bg(t.surface));
        }
        let mut x = area.x + 1;
        let end = area.x + area.width;
        for (key, label) in self.footer_hints() {
            let kw = widgets::width(key) + 2;
            let lw = widgets::width(&label) + 1;
            if x + kw + lw + 2 > end {
                break;
            }
            let key_style = if t.truecolor {
                Style::default()
                    .bg(t.surface_hi)
                    .fg(t.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                theme::accent_bold()
            };
            buf.set_string(x, area.y, format!(" {} ", key), key_style);
            buf.set_string(x + kw, area.y, format!(" {}", label), theme::dim());
            let rect = Rect::new(x, area.y, kw + lw, 1);
            match key {
                "F1" => ctx.click(rect, Action::Help),
                "^Q" => ctx.click(rect, Action::Quit),
                "Esc" => ctx.click(rect, Action::CloseModal),
                _ => {}
            }
            x += kw + lw + 2;
        }
    }

    fn render_toast(&self, buf: &mut Buffer, area: Rect) {
        let Some((tone, msg, _)) = &self.toast else {
            return;
        };
        let width = (widgets::width(msg) + 6)
            .min(area.width.saturating_sub(4))
            .max(20);
        let lines = widgets::wrap(msg, width.saturating_sub(6));
        let h = (lines.len() as u16 + 2).min(6);
        let rect = Rect::new(
            area.x + area.width - width - 2,
            area.y + area.height - h - 2,
            width,
            h,
        );
        widgets::fill(buf, rect, theme::surface());
        let inner = widgets::card(buf, rect, None, true);
        let t = theme::get();
        if t.truecolor {
            widgets::fill(buf, inner, theme::surface());
        }
        widgets::status(
            buf,
            Rect::new(
                inner.x + 1,
                inner.y,
                inner.width.saturating_sub(2),
                inner.height,
            ),
            *tone,
            msg,
            self.tick,
        );
    }
}

fn t_key(key: &str) -> String {
    t(key)
}

pub fn hint(buf: &mut Buffer, area: Rect, text: &str) -> u16 {
    widgets::paragraph(buf, area, text, theme::muted())
}
