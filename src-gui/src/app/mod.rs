mod hooks;
pub mod logic;
pub mod state;
pub mod tasks;
pub mod ui_state;
mod view;
pub mod wheel;
mod window;

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

use egui::{Align, Ui};
use egui_taffy::taffy::prelude::percent;
use egui_taffy::{taffy, tui};
use serde_json::{json, Value};
use smd_core::services::settings::Settings;

use crate::backend::{AppEvent, Backend, EXIT_EVENT};
use crate::dom::{self, Dom, DomCx, Record};
use crate::i18n::{self, I18n};
use crate::platform::Platform;
use crate::store::LocalStore;
use crate::style::{Env, Sheet};
use crate::theme::Mode;
use state::State;
use tasks::{Job, Tasks};
use ui_state::UiState;

const STYLE: &str = include_str!("../../assets/style.css");

pub struct Boot {
    pub backend: Arc<dyn Backend>,
    pub events: Receiver<AppEvent>,
    pub runtime: tokio::runtime::Handle,
    pub settings: Value,
    pub store: LocalStore,
    pub system_locale: String,
    pub platform: Arc<dyn Platform>,
}

pub struct App {
    pub ctx: egui::Context,
    pub tasks: Tasks,
    jobs: Receiver<Job>,
    events: Receiver<AppEvent>,
    pub store: LocalStore,
    pub theme: Mode,
    pub i18n: I18n,
    pub settings: Settings,
    pub language_stored: bool,
    pub ui: UiState,
    pub state: State,
    pub platform: Arc<dyn Platform>,
    pub unhandled: BTreeSet<String>,
    pub images: crate::images::Images,
    pub rects: std::collections::HashMap<&'static str, egui::Rect>,
    pub focused: Option<&'static str>,
    open_modals: Vec<&'static str>,
    svg_serial: u64,
    pub exit_requested: bool,
    pub close_pending: bool,
    pub close_allowed: bool,
    pub sheet: Arc<Sheet>,
    pub record_layout: bool,
    pub layout: Vec<Record>,
}

fn parse_settings(value: Value) -> Settings {
    serde_json::from_value(value).unwrap_or_else(|_| serde_json::from_value(json!({})).expect("default settings"))
}

impl App {
    pub fn new(ctx: &egui::Context, boot: Boot) -> Self {
        crate::fonts::install(ctx);
        if cfg!(target_os = "windows") {
            ctx.options_mut(|options| options.input_options.line_scroll_speed = 100.0);
        }
        if cfg!(target_os = "linux") {
            ctx.options_mut(|options| options.input_options.line_scroll_speed = 0.0);
        }
        egui_extras::install_image_loaders(ctx);
        ctx.global_style_mut(|style| {
            style.interaction.selectable_labels = false;
            style.spacing.item_spacing = egui::Vec2::ZERO;
        });
        let settings = parse_settings(boot.settings);
        let language_stored = !settings.language.is_empty();
        let language = if language_stored { settings.language.clone() } else { boot.system_locale.clone() };
        let language = if i18n::is_available(&language) { language } else { i18n::FALLBACK.to_string() };
        let mode = if boot.store.get("theme").as_deref() == Some("light") { Mode::Light } else { Mode::Dark };
        let (tasks, jobs) = Tasks::new(boot.runtime, boot.backend);
        let wake_ctx = ctx.clone();
        tasks.set_wake(Arc::new(move || wake_ctx.request_repaint()));
        let mut ui = UiState::default();
        view::declare(&mut ui);
        let mut app = Self {
            ctx: ctx.clone(),
            tasks,
            jobs,
            events: boot.events,
            store: boot.store,
            theme: mode,
            i18n: I18n::new(&language),
            settings,
            language_stored,
            ui,
            state: State::default(),
            platform: boot.platform,
            unhandled: BTreeSet::new(),
            images: crate::images::Images::default(),
            rects: std::collections::HashMap::new(),
            focused: None,
            open_modals: Vec::new(),
            svg_serial: 0,
            exit_requested: false,
            close_pending: false,
            close_allowed: false,
            sheet: Arc::new(Sheet::new(STYLE)),
            record_layout: false,
            layout: Vec::new(),
        };
        logic::start(&mut app);
        app
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.i18n.t(key)
    }

    pub fn register_svg(&mut self, svg: &str) -> String {
        self.svg_serial += 1;
        let uri = format!("bytes://svg/{}.svg", self.svg_serial);
        self.ctx.include_bytes(uri.clone(), svg.as_bytes().to_vec());
        self.images.set(&uri, crate::images::Status::Ready([24.0, 24.0]));
        uri
    }

    pub fn set_language(&mut self, code: &str) {
        self.i18n = I18n::new(code);
        for id in view::I18N_IDS {
            self.ui.clear_content(id);
        }
        logic::language_changed(self);
    }

    pub fn emit_event(&self, kind: &str, props: Value) {
        self.tasks.fire("emit_telemetry_event", json!({ "kind": kind, "props": props }));
    }

    pub fn toggle_theme(&mut self) {
        self.theme = self.theme.toggled();
        self.store.set("theme", self.theme.name());
        self.emit_event("theme_toggled", json!({ "to": self.theme.name() }));
    }

    fn load_images(&mut self, ctx: &egui::Context) {
        for uri in self.images.take_wanted() {
            let images = self.images.clone();
            let ctx = ctx.clone();
            self.tasks.spawn(async move {
                let status = match crate::images::fetch(&uri).await {
                    Some(bytes) => match crate::images::dimensions(&bytes) {
                        Some(size) => {
                            ctx.include_bytes(uri.clone(), bytes);
                            crate::images::Status::Ready(size)
                        }
                        None => crate::images::Status::Failed,
                    },
                    None => crate::images::Status::Failed,
                };
                images.set(&uri, status);
                ctx.request_repaint();
            });
        }
    }

    fn drain(&mut self) {
        while let Ok(job) = self.jobs.try_recv() {
            job(self);
        }
        while let Ok(event) = self.events.try_recv() {
            self.on_event(event);
        }
    }

    fn on_event(&mut self, event: AppEvent) {
        if event.channel == EXIT_EVENT {
            self.exit_requested = true;
            return;
        }
        logic::event(self, &event.channel, event.payload);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        crate::fonts::text_rendering(ui.ctx(), !self.theme.is_dark());
        if cfg!(target_os = "linux") {
            wheel::frame(ui.ctx());
        }
        page_keys(ui.ctx());
        self.drain();
        self.window_frame(ui.ctx());
        let window = ui.ctx().content_rect();
        let mut root = ui.new_child(egui::UiBuilder::new().max_rect(window).layout(egui::Layout::top_down(Align::Min)));
        let ui = &mut root;
        ui.set_clip_rect(window);
        let cx = DomCx {
            images: self.images.clone(),
            sheet: self.sheet.clone(),
            env: Env { light: !self.theme.is_dark(), viewport: [window.width(), window.height()] },
            time: ui.input(|i| i.time),
            frame: ui.ctx().cumulative_frame_nr(),
            record: self.record_layout.then(|| RefCell::new(Vec::new())),
            rects: RefCell::new(std::collections::HashMap::new()),
            focused: std::cell::Cell::new(None),
            focus_first: std::cell::Cell::new(false),
        };
        tui(ui, ui.scope_id().with("root"))
            .reserve_available_space()
            .style(taffy::Style { display: taffy::Display::Block, size: taffy::Size { width: percent(1.0), height: percent(1.0) }, ..Default::default() })
            .show(|tui| {
                let mut dom = Dom::root(tui, &cx, "main");
                self.view_body(&mut dom);
            });
        let ctx = ui.ctx().clone();
        let mut open_modals = Vec::new();
        for (id, view, in_shell) in view::MODALS {
            if self.ui.hidden(id) {
                continue;
            }
            open_modals.push(*id);
            cx.focus_first.set(!self.open_modals.contains(id));
            dom::layer(&ctx, &cx, id, egui::Order::Foreground, true, |dom| {
                if *in_shell {
                    dom.virtual_parent("div.app-shell");
                }
                view(self, dom);
            });
            cx.focus_first.set(false);
        }
        self.open_modals = open_modals;
        self.rects = cx.rects.take();
        self.focused = cx.focused.take();
        self.load_images(&ctx);
        logic::frame(self, &ctx);
        if let Some(record) = cx.record {
            self.layout = record.into_inner();
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        App::ui(self, ui);
        if self.exit_requested {
            self.exit_requested = false;
            self.close_now();
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        if cfg!(target_os = "linux") {
            wheel::touchpad_like_webkitgtk(&mut raw_input.events);
        }
    }
}

fn page_keys(ctx: &egui::Context) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let page = ctx.content_rect().height() * 0.875;
    ctx.input_mut(|input| {
        let mut delta = 0.0;
        for (key, amount) in [(egui::Key::PageDown, -page), (egui::Key::PageUp, page), (egui::Key::End, -1.0e6), (egui::Key::Home, 1.0e6)] {
            if input.consume_key(egui::Modifiers::NONE, key) {
                delta += amount;
            }
        }
        input.smooth_scroll_delta.y += delta;
    });
}
