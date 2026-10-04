use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use smd_core::ops::download::DownloadConfig;
use smd_core::services::history::HistoryEntry;

use super::action::{Action, InputId, ListId, ScrollTarget, SourceTab, Step};
use super::state::{EmuState, Parsed};
use super::{apply, App, Modal};
use crate::i18n::{t, tf};
use crate::term;
use crate::theme;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{take_bottom, take_top, Ctx, Fid, Kind};

fn is_resumable(e: &HistoryEntry) -> bool {
    e.status == "cancelled_resumable" && e.resume_payload.is_some()
}

fn can_update(e: &HistoryEntry) -> bool {
    e.status == "complete" && !e.download_dir.is_empty()
}

fn status_badge(e: &HistoryEntry) -> (String, ratatui::style::Color) {
    let th = theme::get();
    match e.status.as_str() {
        "complete" => (t("history.statusComplete"), th.success),
        "partial" => (t("history.statusPartial"), th.warning),
        _ if is_resumable(e) => (t("history.statusResumable"), th.accent),
        "cancelled" | "cancelled_resumable" => (t("history.statusCancelled"), th.text_dim),
        _ => (t("history.statusFailed"), th.error),
    }
}

fn format_date(s: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(s) {
        Ok(d) => d
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
        Err(_) => s.to_string(),
    }
}

impl App {
    pub(super) fn load_history(&mut self) {
        self.hist.loading = true;
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let h = smd_core::services::history::fill_missing_sizes(&dir).await;
            apply(move |app| {
                app.hist.loading = false;
                app.hist.entries = h.entries;
                let in_empty_filter =
                    app.focus == Some(Fid::new("history.filter")) && app.hist.filter.is_empty();
                if app.page == super::action::Page::History
                    && (app.focus.is_none() || in_empty_filter)
                {
                    app.focus = Some(Fid::new("history.list"));
                }
                let n = app.hist.visible().len();
                if app.hist.selected >= n {
                    app.hist.selected = n.saturating_sub(1);
                }
            })
        });
    }

    fn hist_entry(&self, pos: usize) -> Option<HistoryEntry> {
        let vis = self.hist.visible();
        vis.get(pos)
            .and_then(|&i| self.hist.entries.get(i))
            .cloned()
    }

    pub(super) fn history_default_action(&self, pos: usize) -> Action {
        match self.hist_entry(pos) {
            Some(e) if is_resumable(&e) && self.settings.use_native_downloader => {
                Action::HistoryResume(pos)
            }
            _ => Action::HistoryRedownload(pos),
        }
    }

    pub(super) fn dispatch_history(&mut self, a: &Action) -> bool {
        match a {
            Action::HistoryResume(pos) => {
                let resumable = self
                    .hist_entry(*pos)
                    .is_some_and(|e| is_resumable(&e) && self.settings.use_native_downloader);
                if resumable {
                    let mut modal = Modal::confirm(
                        t("modals.resume.title"),
                        t("modals.resume.body"),
                        t("modals.resume.fast"),
                        t("common.cancel"),
                        false,
                        Action::HistoryResumeConfirmed(*pos),
                    );
                    if let Modal::Confirm(c) = &mut modal {
                        c.check = Some((t("modals.resume.verifyCheck"), false));
                    }
                    self.queue_modal(modal);
                }
            }
            Action::HistoryResumeConfirmed(pos) => {
                let verify_all = matches!(&self.modal, Some(Modal::Confirm(c))
                    if c.check.as_ref().is_some_and(|(_, on)| *on));
                self.close_modal();
                self.history_resume(*pos, if verify_all { "verify" } else { "fast" });
            }
            Action::HistoryRedownload(pos) => self.history_redownload(*pos),
            Action::HistoryUpdate(pos) => self.history_update(*pos),
            Action::HistoryRepair(pos) => self.history_repair(*pos),
            Action::HistoryOpenFolder(pos) => {
                let dir = if *pos == usize::MAX {
                    self.wiz.result_dir.clone()
                } else {
                    self.hist_entry(*pos).map(|e| e.download_dir)
                };
                if let Some(dir) = dir {
                    match smd_core::ops::system::open_folder(&dir) {
                        Ok(()) => {
                            self.toast(Tone::Info, tf("tui.history.opened", &[("path", &dir)]))
                        }
                        Err(_) if !std::path::Path::new(&dir).exists() => {
                            self.hist.banner = Some((
                                Tone::Error,
                                t("modals.folderMissing.body"),
                                std::time::Instant::now(),
                            ));
                        }
                        Err(e) => {
                            term::copy_to_clipboard(&dir);
                            self.toast(
                                Tone::Warning,
                                format!("{}\n{}", e, tf("tui.history.copied", &[("path", &dir)])),
                            );
                        }
                    }
                }
            }
            Action::HistoryCopyPath(pos) => {
                if let Some(e) = self.hist_entry(*pos) {
                    term::copy_to_clipboard(&e.download_dir);
                    self.toast(
                        Tone::Success,
                        tf("tui.history.copied", &[("path", &e.download_dir)]),
                    );
                }
            }
            Action::HistoryEditEmu(pos) => self.history_edit_emu(*pos),
            Action::AskHistoryRemove(pos) => {
                let Some(e) = self.hist_entry(*pos) else {
                    return true;
                };
                let resumable = is_resumable(&e);
                self.hist.pending_remove = Some((e.id.clone(), resumable));
                let (title, body, yes) = if resumable {
                    (
                        "modals.historyRemove.titleResumable",
                        "modals.historyRemove.bodyResumable",
                        "modals.historyRemove.yesResumable",
                    )
                } else {
                    (
                        "modals.historyRemove.title",
                        "modals.historyRemove.body",
                        "modals.historyRemove.yes",
                    )
                };
                self.queue_modal(Modal::confirm(
                    t(title),
                    t(body),
                    t(yes),
                    t("modals.historyRemove.no"),
                    true,
                    Action::HistoryRemoveConfirmed,
                ));
            }
            Action::HistoryRemoveConfirmed => {
                self.close_modal();
                let Some((id, delete_files)) = self.hist.pending_remove.take() else {
                    return true;
                };
                let dir = self.data_dir.clone();
                self.spawn(async move {
                    let r = smd_core::ops::history::remove_entry(&dir, &id, delete_files).await;
                    apply(move |app| {
                        if let Err(e) = r {
                            app.toast(Tone::Error, e);
                        }
                        app.load_history();
                    })
                });
            }
            Action::AskHistoryClear => {
                let resumable = self.hist.entries.iter().filter(|e| is_resumable(e)).count();
                let body = if resumable > 0 {
                    t("modals.historyClear.bodyShort")
                } else {
                    t("modals.historyClear.body")
                };
                let mut m = Modal::confirm(
                    t("modals.historyClear.title"),
                    body,
                    t("modals.historyClear.yes"),
                    t("modals.historyClear.no"),
                    true,
                    Action::HistoryClearConfirmed,
                );
                if resumable > 0 {
                    if let Modal::Confirm(c) = &mut m {
                        c.note = Some(tf(
                            "modals.historyClear.resumableWarning",
                            &[("count", &resumable)],
                        ));
                        c.check = Some((t("modals.historyClear.deleteFilesLabel"), false));
                    }
                }
                self.queue_modal(m);
            }
            Action::HistoryClearConfirmed => {
                let delete = match &self.modal {
                    Some(Modal::Confirm(c)) => c.check.as_ref().map(|(_, v)| *v).unwrap_or(false),
                    _ => false,
                };
                self.close_modal();
                let dir = self.data_dir.clone();
                self.spawn(async move {
                    let r = smd_core::ops::history::clear(&dir, delete).await;
                    apply(move |app| {
                        if let Err(e) = r {
                            app.toast(Tone::Error, e);
                        }
                        app.load_history();
                    })
                });
            }
            Action::HistoryRefresh => self.load_history(),
            _ => return false,
        }
        true
    }

    fn history_resume(&mut self, pos: usize, resume_mode: &str) {
        let Some(entry) = self.hist_entry(pos) else {
            return;
        };
        if !is_resumable(&entry) || !self.settings.use_native_downloader {
            return;
        }
        let payload = entry.resume_payload.clone().unwrap_or_default();
        let mut config: DownloadConfig = match serde_json::from_value(payload) {
            Ok(c) => c,
            Err(e) => {
                self.toast(Tone::Error, tf("history.resumeError", &[("message", &e)]));
                return;
            }
        };
        config.resume_mode = Some(resume_mode.to_string());
        self.emu = None;
        self.wiz.parsed = Some(Parsed {
            main_app_id: entry.app_id.clone(),
            depots: Vec::new(),
            all_app_ids: Vec::new(),
        });
        self.wiz.game_name = entry.game_name.clone();
        self.wiz.header_image = entry.header_image.clone();
        self.wiz.selected = config.depots.iter().map(|d| d.depot_id.clone()).collect();
        let ids: Vec<String> = config.depots.iter().map(|d| d.depot_id.clone()).collect();
        self.emit(
            "download_started",
            Some(serde_json::json!({
                "depot_count": ids.len(),
                "engine": "native",
                "source_count": self.settings.depot_sources.len(),
                "had_mh_key": config.manifest_hub_api_key.as_deref().is_some_and(|k| !k.is_empty()),
                "resumed": true,
            })),
        );
        self.begin_download(config, ids, true);
        let dir = self.data_dir.clone();
        let id = entry.id.clone();
        tokio::spawn(async move {
            let _ = smd_core::services::history::remove_entry(&dir, &id).await;
        });
        if self.wiz.result_dir.is_none() {
            self.wiz.result_dir = Some(entry.download_dir);
        }
    }

    fn history_redownload(&mut self, pos: usize) {
        let Some(entry) = self.hist_entry(pos) else {
            return;
        };
        self.reset_wizard();
        self.wiz.tab = SourceTab::Search;
        self.wiz.step = Step::Source;
        self.wiz.auto_redownload = true;
        self.wiz.auto_select = Some(Some(entry.depot_ids.clone()).filter(|v| !v.is_empty()));
        self.wiz.search_input.set(entry.app_id.clone());
        self.page = super::action::Page::Wizard;
        self.perform_search();
    }

    fn history_update(&mut self, pos: usize) {
        let Some(entry) = self.hist_entry(pos) else {
            return;
        };
        if !can_update(&entry) {
            return;
        }
        let dir = entry.download_dir.clone();
        let app_id = entry.app_id.clone();
        self.history_redownload(pos);
        self.wiz.update_dir = Some(dir);
        self.wiz.update_app_id = Some(app_id);
    }

    fn history_repair(&mut self, pos: usize) {
        let Some(entry) = self.hist_entry(pos) else {
            return;
        };
        if !can_update(&entry) {
            return;
        }
        let installed =
            smd_core::services::install_state::installed(std::path::Path::new(&entry.download_dir));
        let manifests: std::collections::HashMap<String, String> = installed
            .into_iter()
            .map(|d| (d.depot_id, d.manifest_id))
            .collect();
        let dir = entry.download_dir.clone();
        let app_id = entry.app_id.clone();
        self.history_redownload(pos);
        if !manifests.is_empty() {
            self.wiz.auto_select = Some(Some(manifests.keys().cloned().collect()));
        }
        self.wiz.update_dir = Some(dir);
        self.wiz.update_app_id = Some(app_id);
        self.wiz.repair_manifests = Some(manifests);
    }

    fn history_edit_emu(&mut self, pos: usize) {
        let Some(entry) = self.hist_entry(pos) else {
            return;
        };
        if entry.download_dir.is_empty() || entry.status == "cancelled" {
            return;
        }
        let dir = entry.download_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| match r {
                Err(_) => {
                    app.hist.banner = Some((
                        Tone::Error,
                        t("modals.folderMissing.body"),
                        std::time::Instant::now(),
                    ))
                }
                Ok(scan) => {
                    let patched: Vec<_> = scan.into_iter().filter(|f| f.is_patched).collect();
                    if patched.is_empty() {
                        app.hist.banner = Some((
                            Tone::Warning,
                            t("emulator.editNoPatches"),
                            std::time::Instant::now(),
                        ));
                        return;
                    }
                    app.wiz.result_dir = Some(entry.download_dir.clone());
                    app.wiz.game_name = entry.game_name.clone();
                    app.wiz.header_image = entry.header_image.clone();
                    match app.wiz.parsed.as_mut() {
                        Some(p) => p.main_app_id = entry.app_id.clone(),
                        None => {
                            app.wiz.parsed = Some(Parsed {
                                main_app_id: entry.app_id.clone(),
                                depots: Vec::new(),
                                all_app_ids: Vec::new(),
                            })
                        }
                    }
                    let mut emu = EmuState::new(entry.download_dir.clone(), entry.app_id.clone());
                    emu.edit_mode = true;
                    emu.selection_explicit = true;
                    app.emulator_available = true;
                    app.enter_emulator(emu, patched, false);
                }
            })
        });
    }

    pub(super) fn render_history(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let top = take_top(&mut rest, 3);
        let clear_w = widgets::button_width(&t("modals.history.clear"));
        let refresh_w = widgets::button_width(&t("tui.history.refresh"));
        let input_w = top.width.saturating_sub(clear_w + refresh_w + 4);
        widgets::input(
            buf,
            ctx,
            Rect::new(top.x, top.y, input_w, 3),
            &mut self.hist.filter,
            InputSpec {
                label: &t("tui.history.filter"),
                placeholder: &t("tui.history.filterPlaceholder"),
                fid: Fid::new("history.filter"),
                id: InputId::HistoryFilter,
                error: false,
            },
        );
        let bx = top.x + input_w + 2;
        widgets::button(
            buf,
            ctx,
            bx,
            top.y + 1,
            refresh_w,
            &ButtonSpec::new(
                t("tui.history.refresh"),
                Fid::new("history.refresh"),
                Action::HistoryRefresh,
                Btn::Secondary,
            ),
        );
        widgets::button(
            buf,
            ctx,
            bx + refresh_w + 2,
            top.y + 1,
            clear_w,
            &ButtonSpec::new(
                t("modals.history.clear"),
                Fid::new("history.clear"),
                Action::AskHistoryClear,
                Btn::Danger,
            )
            .enabled(!self.hist.entries.is_empty()),
        );
        let _ = take_top(&mut rest, 1);

        if let Some((tone, msg, _)) = &self.hist.banner {
            let h = widgets::status_height(msg, rest.width).min(3);
            widgets::status(buf, take_top(&mut rest, h), *tone, msg, self.tick);
            let _ = take_top(&mut rest, 1);
        }

        let action_specs = self.history_action_specs();
        let rows = widgets::button_rows(&action_specs, rest.width);
        let actions = take_bottom(&mut rest, rows);
        let _ = take_bottom(&mut rest, 1);

        if self.hist.loading && self.hist.entries.is_empty() {
            widgets::status(
                buf,
                take_top(&mut rest, 1),
                Tone::Busy,
                &t("common.loading"),
                self.tick,
            );
            return;
        }
        let vis = self.hist.visible();
        if vis.is_empty() {
            let msg = if self.hist.entries.is_empty() {
                t("tui.history.empty")
            } else {
                t("tui.select.noMatch")
            };
            widgets::paragraph(buf, take_top(&mut rest, 2), &msg, theme::muted());
            return;
        }
        if self.hist.selected >= vis.len() {
            self.hist.selected = vis.len() - 1;
        }

        let list_fid = Fid::new("history.list");
        let focused = ctx.is_focused(list_fid);
        let card = widgets::card(buf, rest, None, focused);
        let per = 2usize;
        let visible_items = (card.height as usize / per).max(1);
        let range = widgets::list_window(
            self.hist.selected,
            &mut self.hist.offset,
            vis.len(),
            visible_items,
        );
        for (row, pos) in range.enumerate() {
            let e = &self.hist.entries[vis[pos]];
            let r = Rect::new(
                card.x,
                card.y + (row * per) as u16,
                card.width.saturating_sub(1),
                per as u16,
            );
            let cursor = pos == self.hist.selected;
            if cursor {
                widgets::fill(
                    buf,
                    r,
                    if focused {
                        theme::selection()
                    } else {
                        theme::surface()
                    },
                );
            } else if ctx.is_hovered(r) {
                widgets::fill(buf, r, theme::surface());
            }
            let name = e
                .game_name
                .clone()
                .unwrap_or_else(|| format!("App {}", e.app_id));
            let (badge, color) = status_badge(e);
            let date = format_date(e.completed_at.as_deref().unwrap_or(&e.started_at));
            let line1 = Line::from(vec![
                Span::styled(if cursor { " ▸ " } else { "   " }, theme::accent()),
                Span::styled(name, theme::text().add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                widgets::badge(&badge, color),
            ]);
            widgets::line(buf, Rect::new(r.x, r.y, r.width, 1), line1);
            widgets::text_right(
                buf,
                Rect::new(r.x, r.y, r.width.saturating_sub(1), 1),
                &date,
                theme::muted(),
            );
            let size = e
                .size_bytes
                .map(|b| format!("{}  ·  ", widgets::fmt_bytes(b)))
                .unwrap_or_default();
            let meta = format!(
                "   App {}  ·  {}  ·  {}{}",
                e.app_id,
                tf(
                    "tui.history.depots",
                    &[("done", &e.depots_downloaded), ("total", &e.depot_count)]
                ),
                size,
                e.download_dir
            );
            widgets::text(
                buf,
                Rect::new(r.x, r.y + 1, r.width, 1),
                &meta,
                theme::dim(),
            );
            ctx.row(
                r,
                list_fid,
                Action::ListSelect(ListId::History, pos),
                Action::ListActivate(ListId::History, pos),
            );
        }
        widgets::scrollbar(buf, card, vis.len(), visible_items, self.hist.offset);
        ctx.focusable(list_fid, rest, Kind::List(ListId::History), None);
        ctx.scroll_region(rest, ScrollTarget::List(ListId::History));
        ctx.prefer(list_fid);

        widgets::buttons_wrapped(buf, ctx, actions, &action_specs);
    }

    fn history_action_specs(&self) -> Vec<ButtonSpec> {
        let vis = self.hist.visible();
        let pos = self.hist.selected.min(vis.len().saturating_sub(1));
        let Some(e) = vis.get(pos).and_then(|&i| self.hist.entries.get(i)) else {
            return Vec::new();
        };
        let resumable = is_resumable(e) && self.settings.use_native_downloader;
        let has_dir = e.status != "cancelled" && !e.download_dir.is_empty();
        vec![
            ButtonSpec::new(
                format!("▶ {}", t("tui.history.resume")),
                Fid::new("history.resume"),
                Action::HistoryResume(pos),
                Btn::Primary,
            )
            .enabled(resumable),
            ButtonSpec::new(
                format!("↻ {}", t("tui.history.redownload")),
                Fid::new("history.redownload"),
                Action::HistoryRedownload(pos),
                Btn::Secondary,
            ),
            ButtonSpec::new(
                format!("⇡ {}", t("tui.history.update")),
                Fid::new("history.update"),
                Action::HistoryUpdate(pos),
                Btn::Secondary,
            )
            .enabled(can_update(e)),
            ButtonSpec::new(
                format!("✓ {}", t("tui.history.repair")),
                Fid::new("history.repair"),
                Action::HistoryRepair(pos),
                Btn::Secondary,
            )
            .enabled(can_update(e)),
            ButtonSpec::new(
                t("tui.history.openFolder"),
                Fid::new("history.open"),
                Action::HistoryOpenFolder(pos),
                Btn::Secondary,
            )
            .enabled(has_dir),
            ButtonSpec::new(
                t("tui.history.copyPath"),
                Fid::new("history.copy"),
                Action::HistoryCopyPath(pos),
                Btn::Secondary,
            ),
            ButtonSpec::new(
                t("tui.history.editEmu"),
                Fid::new("history.emu"),
                Action::HistoryEditEmu(pos),
                Btn::Secondary,
            )
            .enabled(has_dir),
            ButtonSpec::new(
                t("tui.history.remove"),
                Fid::new("history.remove"),
                Action::AskHistoryRemove(pos),
                Btn::Danger,
            ),
        ]
    }
}
