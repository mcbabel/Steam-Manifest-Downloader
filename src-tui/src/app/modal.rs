use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::action::{Action, BrowsePurpose, InputId, ListId, ScrollTarget};
use super::{apply, App};
use crate::i18n::{self, t, tf};
use crate::term;
use crate::theme;
use crate::ui::file_browser::{BrowseMode, FileBrowser};
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{centered, Ctx, Fid, Kind};

pub struct Confirm {
    pub title: String,
    pub body: String,
    pub yes: String,
    pub no: String,
    pub danger: bool,
    pub on_yes: Action,
    pub on_no: Option<Action>,
    pub check: Option<(String, bool)>,
    pub note: Option<String>,
}

pub struct UpdateInfo {
    pub version: String,
    pub date: Option<String>,
    pub notes: String,
    pub url: Option<String>,
    pub method: String,
}

pub enum Modal {
    Confirm(Confirm),
    Browser(FileBrowser),
    Telemetry,
    Language(&'static str),
    Update(UpdateInfo),
    Help,
}

impl Modal {
    pub fn confirm(
        title: String,
        body: String,
        yes: String,
        no: String,
        danger: bool,
        on_yes: Action,
    ) -> Self {
        Modal::Confirm(Confirm {
            title,
            body,
            yes,
            no,
            danger,
            on_yes,
            on_no: None,
            check: None,
            note: None,
        })
    }

    pub fn telemetry() -> Self {
        Modal::Telemetry
    }

    pub fn language(current: &'static str) -> Self {
        Modal::Language(current)
    }

    pub fn update(info: &serde_json::Value) -> Self {
        let date = info["date"]
            .as_str()
            .map(|d| d.split('T').next().unwrap_or(d).to_string());
        Modal::Update(UpdateInfo {
            version: info["version"].as_str().unwrap_or_default().to_string(),
            date,
            notes: info["body"].as_str().unwrap_or_default().to_string(),
            url: info["releaseUrl"].as_str().map(String::from),
            method: info["installMethod"].as_str().unwrap_or("self").to_string(),
        })
    }

    pub fn shortcut(&self, c: char) -> Option<Action> {
        match self {
            Modal::Confirm(c_) => match c {
                'y' | 'j' => Some(c_.on_yes.clone()),
                'n' => Some(c_.on_no.clone().unwrap_or(Action::CloseModal)),
                _ => None,
            },
            Modal::Telemetry => match c {
                'y' | 'j' => Some(Action::TelemetryAnswer(true)),
                'n' => Some(Action::TelemetryAnswer(false)),
                _ => None,
            },
            Modal::Browser(_) => match c {
                '.' => Some(Action::BrowserToggleHidden),
                '~' => Some(Action::BrowserHome),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn cancel_action(&self) -> Option<Action> {
        match self {
            Modal::Confirm(c) => Some(c.on_no.clone().unwrap_or(Action::CloseModal)),
            Modal::Telemetry => Some(Action::TelemetryAnswer(false)),
            Modal::Language(code) => Some(Action::LanguageChosen(code)),
            _ => Some(Action::CloseModal),
        }
    }
}

const fn f(name: &'static str) -> Fid {
    Fid::new(name)
}

impl App {
    pub fn open_browser(&mut self, purpose: BrowsePurpose) {
        let (mode, title, start): (BrowseMode, String, Option<PathBuf>) = match purpose {
            BrowsePurpose::UploadFile => (
                BrowseMode::File {
                    exts: vec!["lua", "st"],
                },
                t("tui.browse.luaTitle"),
                non_empty(self.wiz.upload_path.value()),
            ),
            BrowsePurpose::PatchDir => (
                BrowseMode::Dir,
                t("patchOnly.folderDialogTitle"),
                non_empty(self.wiz.patch_dir.value()),
            ),
            BrowsePurpose::DownloadDir => (
                BrowseMode::Dir,
                t("tui.browse.downloadTitle"),
                non_empty(self.wiz.download_dir.value()),
            ),
            BrowsePurpose::SettingsDownloadDir => (
                BrowseMode::Dir,
                t("tui.browse.downloadTitle"),
                non_empty(self.set.inputs[super::state::SETTING_DOWNLOAD_DIR].value()),
            ),
            BrowsePurpose::DepotManifest(_) => (
                BrowseMode::File {
                    exts: vec!["manifest"],
                },
                t("depots.uploadManifest"),
                None,
            ),
            BrowsePurpose::ShortcutExe | BrowsePurpose::SteamExe => (
                BrowseMode::File {
                    exts: if self.shortcut_supported {
                        vec!["exe"]
                    } else {
                        vec![]
                    },
                },
                t("tui.browse.exeTitle"),
                self.wiz.result_dir.as_deref().and_then(non_empty),
            ),
        };
        let fb = FileBrowser::open(purpose, mode, title, start.as_deref());
        self.show_modal(Modal::Browser(fb));
    }

    pub fn browser_activate(&mut self, idx: usize) {
        let picked = match self.modal.as_mut() {
            Some(Modal::Browser(fb)) => {
                fb.selected = idx;
                let p = fb.activate(idx);
                p.map(|p| (fb.purpose, p))
            }
            _ => None,
        };
        if let Some((purpose, path)) = picked {
            self.close_modal();
            self.on_browsed(purpose, path);
        }
    }

    fn on_browsed(&mut self, purpose: BrowsePurpose, path: PathBuf) {
        let s = path.to_string_lossy().to_string();
        match purpose {
            BrowsePurpose::UploadFile => {
                self.wiz.upload_path.set(s);
                self.dispatch(Action::LoadUpload);
            }
            BrowsePurpose::PatchDir => {
                self.wiz.patch_dir.set(s);
                self.wiz.patch_error = None;
            }
            BrowsePurpose::DownloadDir => self.wiz.download_dir.set(s),
            BrowsePurpose::SettingsDownloadDir => {
                self.set.inputs[super::state::SETTING_DOWNLOAD_DIR].set(s);
                self.set.dirty = true;
            }
            BrowsePurpose::DepotManifest(i) => self.set_depot_manifest(i, path),
            BrowsePurpose::ShortcutExe => self.wiz.shortcut_exe.set(s),
            BrowsePurpose::SteamExe => self.wiz.steam_exe.set(s),
        }
    }

    pub(super) fn dispatch_modal(&mut self, action: Action) {
        match action {
            Action::ModalToggleCheck => {
                if let Some(Modal::Confirm(c)) = self.modal.as_mut() {
                    if let Some((_, v)) = c.check.as_mut() {
                        *v = !*v;
                    }
                }
            }
            Action::BrowserUp => {
                if let Some(Modal::Browser(fb)) = self.modal.as_mut() {
                    fb.up();
                }
            }
            Action::BrowserHome => {
                if let Some(Modal::Browser(fb)) = self.modal.as_mut() {
                    fb.home();
                }
            }
            Action::BrowserToggleHidden => {
                if let Some(Modal::Browser(fb)) = self.modal.as_mut() {
                    fb.toggle_hidden();
                }
            }
            Action::BrowserChoose => {
                let in_path = self.focus == Some(f("modal.browser.path"));
                let picked = match self.modal.as_mut() {
                    Some(Modal::Browser(fb)) => {
                        let p = if in_path {
                            fb.submit_path()
                        } else {
                            fb.choose()
                        };
                        p.map(|p| (fb.purpose, p))
                    }
                    _ => None,
                };
                if let Some((purpose, path)) = picked {
                    self.close_modal();
                    self.on_browsed(purpose, path);
                }
            }
            Action::TelemetryAnswer(accept) => {
                self.close_modal();
                let dir = self.data_dir.clone();
                self.spawn(async move {
                    let r = smd_core::ops::consent::set_telemetry_consent(&dir, accept).await;
                    apply(move |app| {
                        if r.is_ok() && accept {
                            app.emit("consent_accepted", None);
                            app.emit("app_start", None);
                        }
                        app.reload_settings();
                    })
                });
            }
            Action::LanguageChosen(code) => {
                self.close_modal();
                self.set_language(code);
            }
            Action::UpdateSkip => {
                if let Some(Modal::Update(u)) = &self.modal {
                    self.prefs.skipped_update = Some(u.version.clone());
                    self.save_prefs();
                }
                self.close_modal();
            }
            Action::UpdateOpen => {
                if let Some(Modal::Update(u)) = &self.modal {
                    if let Some(url) = &u.url {
                        open_url(url);
                        term::copy_to_clipboard(url);
                        self.toast(Tone::Info, tf("tui.update.opened", &[("url", url)]));
                    }
                }
                self.close_modal();
            }
            _ => {}
        }
    }

    pub fn set_language(&mut self, code: &'static str) {
        i18n::set_language(code);
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let mut s = smd_core::services::settings::load_settings(&dir).await;
            s.language = code.to_string();
            let _ = smd_core::services::settings::save_settings(&dir, &s).await;
            apply(move |app| {
                app.settings.language = code.to_string();
                app.set.draft.language = code.to_string();
            })
        });
    }

    pub(super) fn render_modal(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let Some(modal) = self.modal.as_mut() else {
            return;
        };
        match modal {
            Modal::Confirm(c) => render_confirm(buf, area, ctx, c),
            Modal::Browser(fb) => render_browser(buf, area, ctx, fb),
            Modal::Telemetry => render_telemetry(buf, area, ctx),
            Modal::Language(code) => render_language(buf, area, ctx, code),
            Modal::Update(u) => render_update(buf, area, ctx, u),
            Modal::Help => render_help(buf, area, ctx),
        }
    }
}

fn non_empty(s: &str) -> Option<PathBuf> {
    let s = s.trim();
    (!s.is_empty()).then(|| PathBuf::from(s))
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    let _ = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

fn frame(buf: &mut Buffer, area: Rect, width: u16, height: u16, title: &str) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    let rect = centered(area, w, h);
    widgets::fill(buf, rect, theme::surface());
    let inner = widgets::card(buf, rect, Some(title), true);
    let t = theme::get();
    if t.truecolor {
        widgets::fill(buf, inner, theme::surface());
    }
    Rect::new(
        inner.x + 1,
        inner.y,
        inner.width.saturating_sub(2),
        inner.height,
    )
}

fn render_confirm(buf: &mut Buffer, area: Rect, ctx: &mut Ctx, c: &Confirm) {
    let width = 64u16;
    let inner_w = width.saturating_sub(4);
    let mut h = widgets::wrap_height(&c.body, inner_w) + 4;
    if let Some(n) = &c.note {
        h += widgets::wrap_height(n, inner_w) + 1;
    }
    if c.check.is_some() {
        h += 2;
    }
    let inner = frame(buf, area, width, h + 2, &c.title);
    let mut y = inner.y + 1;
    y += widgets::paragraph(
        buf,
        Rect::new(inner.x, y, inner.width, inner.height),
        &c.body,
        theme::text(),
    );
    if let Some(n) = &c.note {
        y += 1;
        y += widgets::paragraph(
            buf,
            Rect::new(inner.x, y, inner.width, 6),
            n,
            theme::warning(),
        );
    }
    if let Some((label, on)) = &c.check {
        y += 1;
        widgets::checkbox(
            buf,
            ctx,
            Rect::new(inner.x, y, inner.width, 1),
            *on,
            label,
            f("modal.check"),
            Action::ModalToggleCheck,
            true,
        );
    }
    let row = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    let yes = ButtonSpec::new(
        c.yes.clone(),
        f("modal.yes"),
        c.on_yes.clone(),
        if c.danger { Btn::Danger } else { Btn::Primary },
    );
    let no = ButtonSpec::new(
        c.no.clone(),
        f("modal.no"),
        c.on_no.clone().unwrap_or(Action::CloseModal),
        Btn::Secondary,
    );
    widgets::buttons(buf, ctx, row, &[no, yes], true);
    ctx.prefer(f("modal.no"));
}

fn render_browser(buf: &mut Buffer, area: Rect, ctx: &mut Ctx, fb: &mut FileBrowser) {
    let w = area.width.saturating_sub(8).clamp(50, 110);
    let h = area.height.saturating_sub(4).clamp(14, 40);
    let inner = frame(buf, area, w, h, &fb.title.clone());
    let mut rest = inner;
    rest.y += 1;
    rest.height = rest.height.saturating_sub(1);
    let path_area = crate::ui::take_top(&mut rest, 3);
    widgets::input(
        buf,
        ctx,
        path_area,
        &mut fb.path_input,
        InputSpec {
            label: &t("tui.browse.path"),
            placeholder: &t("tui.browse.drives"),
            fid: f("modal.browser.path"),
            id: InputId::BrowserPath,
            error: false,
        },
    );
    let buttons_row = crate::ui::take_bottom(&mut rest, 1);
    let _ = crate::ui::take_bottom(&mut rest, 1);
    if let Some(err) = &fb.error {
        let e = crate::ui::take_bottom(&mut rest, 1);
        widgets::status(buf, e, Tone::Error, err, 0);
    }

    let list_fid = f("modal.browser.list");
    let list_rect = rest;
    let len = fb.entries.len();
    let range = widgets::list_window(fb.selected, &mut fb.offset, len, list_rect.height as usize);
    for (row, i) in range.clone().enumerate() {
        let e = &fb.entries[i];
        let r = Rect::new(
            list_rect.x,
            list_rect.y + row as u16,
            list_rect.width.saturating_sub(1),
            1,
        );
        let selected = i == fb.selected;
        if selected {
            widgets::fill(buf, r, theme::selection());
        } else if ctx.is_hovered(r) {
            widgets::fill(buf, r, theme::surface());
        }
        let (icon, style) = if e.parent {
            ("↰", theme::accent())
        } else if e.is_dir {
            ("▸", theme::accent())
        } else {
            ("•", theme::dim())
        };
        let name = if e.is_dir && !e.parent {
            format!("{}/", e.name)
        } else {
            e.name.clone()
        };
        let size = if e.is_dir {
            String::new()
        } else {
            widgets::fmt_bytes(e.size)
        };
        let name_w = r.width.saturating_sub(widgets::width(&size) + 5);
        let mut name_style = if e.is_dir {
            theme::accent()
        } else {
            theme::text()
        };
        if selected {
            name_style = name_style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(r.x + 1, r.y, icon, style);
        buf.set_string(r.x + 3, r.y, widgets::truncate(&name, name_w), name_style);
        widgets::text_right(
            buf,
            Rect::new(r.x, r.y, r.width.saturating_sub(1), 1),
            &size,
            theme::muted(),
        );
        ctx.row(
            r,
            list_fid,
            Action::ListSelect(ListId::Browser, i),
            Action::ListActivate(ListId::Browser, i),
        );
    }
    if len == 0 {
        widgets::text(buf, list_rect, &t("tui.browse.empty"), theme::muted());
    }
    widgets::scrollbar(buf, list_rect, len, list_rect.height as usize, fb.offset);
    ctx.focusable(list_fid, list_rect, Kind::List(ListId::Browser), None);
    ctx.scroll_region(list_rect, ScrollTarget::List(ListId::Browser));
    ctx.prefer(list_fid);

    let choose_label = if fb.is_dir_mode() {
        t("tui.browse.chooseFolder")
    } else {
        t("tui.browse.open")
    };
    let hidden_label = if fb.show_hidden {
        t("tui.browse.hideHidden")
    } else {
        t("tui.browse.showHidden")
    };
    let specs = [
        ButtonSpec::new(
            t("tui.browse.up"),
            f("modal.browser.up"),
            Action::BrowserUp,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            t("tui.browse.home"),
            f("modal.browser.home"),
            Action::BrowserHome,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            hidden_label,
            f("modal.browser.hidden"),
            Action::BrowserToggleHidden,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            t("common.cancel"),
            f("modal.browser.cancel"),
            Action::CloseModal,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            choose_label,
            f("modal.browser.choose"),
            Action::BrowserChoose,
            Btn::Primary,
        ),
    ];
    widgets::buttons(buf, ctx, buttons_row, &specs, true);
}

fn render_telemetry(buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
    let width = 72u16;
    let iw = width - 4;
    let lead = t("modals.telemetry.lead");
    let hint = t("modals.telemetry.hint");
    let collect: Vec<String> = (1..=4)
        .map(|i| t(&format!("modals.telemetry.collectItem{}", i)))
        .collect();
    let never: Vec<String> = (1..=4)
        .map(|i| t(&format!("modals.telemetry.neverItem{}", i)))
        .collect();
    let list_h = |items: &[String]| {
        items
            .iter()
            .map(|s| widgets::wrap_height(s, iw - 4))
            .sum::<u16>()
    };
    let h = widgets::wrap_height(&lead, iw)
        + list_h(&collect)
        + list_h(&never)
        + widgets::wrap_height(&hint, iw)
        + 10;
    let inner = frame(buf, area, width, h, &t("modals.telemetry.title"));
    let mut y = inner.y + 1;
    let w = inner.width;
    y += widgets::paragraph(buf, Rect::new(inner.x, y, w, 6), &lead, theme::text()) + 1;
    let list = |buf: &mut Buffer, y: &mut u16, title: &str, items: &[String], ok: bool| {
        buf.set_string(
            inner.x,
            *y,
            title,
            theme::text().add_modifier(Modifier::BOLD),
        );
        *y += 1;
        for item in items {
            let (mark, st) = if ok {
                ("✓", theme::success())
            } else {
                ("✗", theme::error())
            };
            buf.set_string(inner.x + 1, *y, mark, st);
            *y += widgets::paragraph(
                buf,
                Rect::new(inner.x + 3, *y, w.saturating_sub(3), 3),
                item,
                theme::dim(),
            );
        }
    };
    list(
        buf,
        &mut y,
        &t("modals.telemetry.collectTitle"),
        &collect,
        true,
    );
    list(
        buf,
        &mut y,
        &t("modals.telemetry.neverTitle"),
        &never,
        false,
    );
    y += 1;
    widgets::paragraph(buf, Rect::new(inner.x, y, w, 4), &hint, theme::muted());
    let row = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    let specs = [
        ButtonSpec::new(
            t("modals.telemetry.decline"),
            f("modal.no"),
            Action::TelemetryAnswer(false),
            Btn::Secondary,
        ),
        ButtonSpec::new(
            t("modals.telemetry.accept"),
            f("modal.yes"),
            Action::TelemetryAnswer(true),
            Btn::Primary,
        ),
    ];
    widgets::buttons(buf, ctx, row, &specs, true);
    ctx.prefer(f("modal.yes"));
}

fn render_language(buf: &mut Buffer, area: Rect, ctx: &mut Ctx, current: &mut &'static str) {
    let h = i18n::LANGUAGES.len() as u16 + 7;
    let inner = frame(buf, area, 50, h, &t("languagePicker.title"));
    let mut y = inner.y + 1;
    widgets::paragraph(
        buf,
        Rect::new(inner.x, y, inner.width, 2),
        &t("languagePicker.subtitle"),
        theme::dim(),
    );
    y += 2;
    for (i, (code, name)) in i18n::LANGUAGES.iter().enumerate() {
        widgets::radio(
            buf,
            ctx,
            Rect::new(inner.x + 2, y, inner.width - 2, 1),
            *current == *code,
            name,
            Fid::idx("modal.lang", i),
            Action::LanguageChosen(code),
        );
        if *current == *code {
            ctx.prefer(Fid::idx("modal.lang", i));
        }
        y += 1;
    }
}

fn render_update(buf: &mut Buffer, area: Rect, ctx: &mut Ctx, u: &UpdateInfo) {
    let width = 76u16;
    let notes = markdown_to_text(&u.notes);
    let max_notes = area.height.saturating_sub(16).max(3);
    let notes_h = widgets::wrap_height(&notes, width - 4).min(max_notes);
    let managed = u.method != "self";
    let extra = if managed { 4 } else { 0 };
    let inner = frame(
        buf,
        area,
        width,
        notes_h + 9 + extra,
        &t("modals.update.title"),
    );
    let mut y = inner.y + 1;
    let mut line = format!("{} v{}", t("modals.update.newVersion"), u.version);
    if let Some(d) = &u.date {
        line.push_str(&format!("   {} {}", t("modals.update.released"), d));
    }
    buf.set_string(inner.x, y, &line, theme::accent_bold());
    y += 2;
    y += widgets::paragraph(
        buf,
        Rect::new(inner.x, y, inner.width, notes_h),
        &notes,
        theme::text(),
    );
    if managed {
        y += 1;
        widgets::paragraph(
            buf,
            Rect::new(inner.x, y, inner.width, 1),
            &t("modals.update.systemText"),
            theme::dim(),
        );
        let cmd = match u.method.as_str() {
            "flatpak" => "flatpak update de.mcbabel.SteamManifestDownloader",
            "snap" => "sudo snap refresh steam-manifest-downloader",
            _ => "paru -Syu steam-manifest-downloader-bin",
        };
        buf.set_string(inner.x + 2, y + 1, cmd, theme::accent());
    }
    let row = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    let specs = [
        ButtonSpec::new(
            t("modals.update.skip"),
            f("modal.update.skip"),
            Action::UpdateSkip,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            t("modals.update.later"),
            f("modal.no"),
            Action::CloseModal,
            Btn::Secondary,
        ),
        ButtonSpec::new(
            t("tui.update.openRelease"),
            f("modal.yes"),
            Action::UpdateOpen,
            Btn::Primary,
        )
        .enabled(u.url.is_some()),
    ];
    widgets::buttons(buf, ctx, row, &specs, true);
    ctx.prefer(f("modal.yes"));
}

fn render_help(buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
    let rows: Vec<(&str, String)> = vec![
        ("Tab / Shift+Tab", t("tui.help.focus")),
        ("↑ ↓ ← →", t("tui.help.arrows")),
        ("Enter / Space", t("tui.help.activate")),
        ("Esc", t("tui.help.back")),
        ("F2 F3 F4", t("tui.help.pages")),
        ("PgUp / PgDn", t("tui.help.scroll")),
        ("Ctrl+A/E/U/K/W", t("tui.help.editing")),
        ("a / n  /  d", t("tui.help.select")),
        ("p / c", t("tui.help.progress")),
        ("r d o e x", t("tui.help.history")),
        ("Ctrl+Q / F10", t("tui.help.quit")),
    ];
    let mouse = t("tui.help.mouse");
    let drop = t("tui.help.drop");
    let width = area.width.saturating_sub(4).min(100);
    let h = rows.len() as u16
        + widgets::wrap_height(&mouse, width - 4)
        + widgets::wrap_height(&drop, width - 4)
        + 7;
    let inner = frame(buf, area, width, h, &t("tui.help.title"));
    let mut y = inner.y + 1;
    for (keys, what) in rows {
        buf.set_string(inner.x, y, keys, theme::accent_bold());
        widgets::text(
            buf,
            Rect::new(inner.x + 18, y, inner.width.saturating_sub(18), 1),
            &what,
            theme::text(),
        );
        y += 1;
    }
    y += 1;
    y += widgets::paragraph(
        buf,
        Rect::new(inner.x, y, inner.width, 3),
        &mouse,
        theme::dim(),
    );
    widgets::paragraph(
        buf,
        Rect::new(inner.x, y, inner.width, 3),
        &drop,
        theme::dim(),
    );
    let row = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
    widgets::buttons(
        buf,
        ctx,
        row,
        &[ButtonSpec::new(
            t("common.close"),
            f("modal.yes"),
            Action::CloseModal,
            Btn::Primary,
        )],
        true,
    );
}

fn markdown_to_text(md: &str) -> String {
    md.lines()
        .map(|l| {
            let l = l.trim_end();
            let l = l.trim_start_matches('#').trim_start();
            let l = if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                format!("• {}", rest)
            } else {
                l.to_string()
            };
            l.replace("**", "").replace('`', "")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
