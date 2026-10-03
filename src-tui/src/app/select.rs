use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use smd_core::ops::download::{DepotConfig, DownloadConfig};
use smd_core::services::steam_pics::DepotRole;

use super::action::{Action, BrowsePurpose, InputId, ListId, ScrollTarget, Step};
use super::state::ProgressState;
use super::{apply, hint, App};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{split_h, take_bottom, take_top, Ctx, Fid, Kind, ScrollView};

impl App {
    pub(super) fn dispatch_select(&mut self, a: &Action) -> bool {
        match a {
            Action::ToggleDepot(pos) => {
                let vis = self.wiz.visible_depots();
                if let (Some(&i), Some(p)) = (vis.get(*pos), self.wiz.parsed.as_ref()) {
                    let id = p.depots[i].depot_id.clone();
                    if !self.wiz.selected.remove(&id) {
                        self.wiz.selected.insert(id);
                    }
                    self.wiz.depot_cursor = *pos;
                }
            }
            Action::SelectAll => {
                if let Some(p) = &self.wiz.parsed {
                    self.wiz.selected = p.depots.iter().map(|d| d.depot_id.clone()).collect();
                }
            }
            Action::DeselectAll => self.wiz.selected.clear(),
            Action::ToggleShowSelected => {
                self.wiz.show_selected_only = !self.wiz.show_selected_only;
                self.wiz.depot_cursor = 0;
                self.wiz.depot_offset = 0;
            }
            Action::FetchLatest(i) => self.fetch_latest(*i),
            Action::ClearDepotManifest(i) => {
                if let Some(d) = self.wiz.parsed.as_mut().and_then(|p| p.depots.get_mut(*i)) {
                    d.uploaded_manifest = None;
                    d.status = None;
                }
            }
            Action::ToggleMask(id) => {
                if let Some(input) = self.input_mut(*id) {
                    input.masked = !input.masked;
                }
            }
            Action::StartDownload => self.start_download(),
            Action::BackToSource => {
                self.wiz.step = Step::Source;
                self.focus = None;
            }
            _ => return false,
        }
        true
    }

    pub(super) fn set_depot_manifest(&mut self, idx: usize, path: PathBuf) {
        if let Some(d) = self.wiz.parsed.as_mut().and_then(|p| p.depots.get_mut(idx)) {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            d.status = Some((Tone::Success, name));
            d.uploaded_manifest = Some(path);
        }
    }

    fn fetch_latest(&mut self, idx: usize) {
        let Some(app_id) = self.wiz.main_app_id() else {
            return;
        };
        let Some(d) = self.wiz.parsed.as_mut().and_then(|p| p.depots.get_mut(idx)) else {
            return;
        };
        if d.fetching {
            return;
        }
        d.fetching = true;
        d.status = Some((Tone::Busy, t("depots.fetchingLatest")));
        let depot_id = d.depot_id.clone();
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r =
                smd_core::ops::search::fetch_latest_manifest_id(&core, &dir, &app_id, &depot_id)
                    .await;
            apply(move |app| {
                let Some(d) = app
                    .wiz
                    .parsed
                    .as_mut()
                    .and_then(|p| p.depots.iter_mut().find(|d| d.depot_id == depot_id))
                else {
                    return;
                };
                d.fetching = false;
                match r {
                    Ok(res) => {
                        let src = if res.source == "steam" {
                            t("depots.fetchSourceSteam")
                        } else {
                            t("depots.fetchSourceFallback")
                        };
                        d.custom_manifest.set(res.manifest_id.clone());
                        d.status = Some((Tone::Success, format!("{} ({})", res.manifest_id, src)));
                    }
                    Err(e) => {
                        d.status = Some((
                            Tone::Error,
                            tf("depots.fetchLatestError", &[("message", &e)]),
                        ))
                    }
                }
            })
        });
    }

    fn start_download(&mut self) {
        let Some(parsed) = self.wiz.parsed.clone() else {
            return;
        };
        let selected: Vec<_> = parsed
            .depots
            .iter()
            .filter(|d| self.wiz.selected.contains(&d.depot_id))
            .collect();
        if selected.is_empty() {
            return;
        }
        let native = self.settings.use_native_downloader;
        let mh_key = self.wiz.mh_key.trimmed();
        self.prefs.mh_api_key = mh_key.clone();
        self.save_prefs();
        self.wiz.mh_hint = None;

        let download_dir = self.wiz.download_dir.trimmed();
        if !download_dir.is_empty() && download_dir != self.settings.download_location {
            let mut s = self.settings.clone();
            s.download_location = download_dir.clone();
            self.persist_settings(s);
        }

        let depots: Vec<DepotConfig> = selected
            .iter()
            .map(|d| {
                let mut custom = Some(d.custom_manifest.trimmed()).filter(|s| !s.is_empty());
                let uploaded = d
                    .uploaded_manifest
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string());
                if custom.is_none() {
                    if let Some(p) = &d.uploaded_manifest {
                        custom = manifest_id_from_filename(
                            &p.file_name().unwrap_or_default().to_string_lossy(),
                        );
                    }
                }
                DepotConfig {
                    depot_id: d.depot_id.clone(),
                    manifest_id: d.manifest_id.clone(),
                    custom_manifest_id: custom,
                    depot_key: d.depot_key.clone(),
                    uploaded_manifest_path: uploaded,
                    display_name: self
                        .wiz
                        .depot_pics
                        .get(&d.depot_id)
                        .and_then(|p| p.name.clone())
                        .filter(|n| !n.trim().is_empty()),
                }
            })
            .collect();

        self.emit(
            "download_started",
            Some(serde_json::json!({
                "depot_count": depots.len(),
                "engine": if native { "native" } else { "ddm" },
                "source_count": self.settings.depot_sources.len(),
                "had_mh_key": !mh_key.is_empty(),
            })),
        );

        let config = DownloadConfig {
            app_id: parsed.main_app_id.clone(),
            game_name: self.wiz.game_name.clone(),
            depots,
            mode: Some(
                if self.wiz.from_search {
                    "search"
                } else {
                    "upload"
                }
                .into(),
            ),
            key_vdf_keys: if self.wiz.from_search {
                self.wiz.search_key_vdf.clone()
            } else {
                None
            },
            download_location: Some(download_dir).filter(|s| !s.is_empty()),
            manifest_hub_api_key: Some(mh_key).filter(|s| !s.is_empty()),
            header_image: self.wiz.header_image.clone(),
            source_type: if self.wiz.from_search {
                self.wiz.source_type.clone()
            } else {
                None
            },
            update_dir: self.wiz.active_update_dir(&parsed.main_app_id),
        };
        let ids: Vec<String> = config.depots.iter().map(|d| d.depot_id.clone()).collect();
        self.begin_download(config, ids, native);
    }

    pub(super) fn begin_download(
        &mut self,
        config: DownloadConfig,
        depot_ids: Vec<String>,
        native: bool,
    ) {
        let mut progress = ProgressState::new(&depot_ids, native);
        progress.status = t("tui.progress.initializing");
        self.wiz.progress = Some(progress);
        self.wiz.job_id = None;
        self.wiz.result_dir = None;
        self.wiz.download_started_at = Some(chrono::Utc::now().to_rfc3339());
        self.wiz.step = Step::Progress;
        self.page = super::action::Page::Wizard;
        self.focus = None;
        self.log(super::state::LogKind::Info, t("tui.progress.connected"));

        let core = self.core.clone();
        let dir = self.data_dir.clone();
        let sink = self.sink.clone();
        self.spawn(async move {
            let r = smd_core::ops::download::start_download(sink, &core, &dir, config).await;
            apply(move |app| match r {
                Ok(v) => {
                    if app.wiz.job_id.is_none() {
                        app.wiz.job_id = v["jobId"].as_str().map(String::from);
                    }
                    app.wiz.result_dir = v["downloadDir"].as_str().map(String::from);
                }
                Err(e) => {
                    app.log(super::state::LogKind::Error, format!("Error: {}", e));
                    app.finish_download(false, e);
                }
            })
        });
    }

    pub(super) fn render_select(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let Some(parsed) = &self.wiz.parsed else {
            return;
        };
        let app_id = parsed.main_app_id.clone();
        let depot_count = parsed.depots.len();
        let mut rest = area;

        let name = self
            .wiz
            .game_name
            .clone()
            .unwrap_or_else(|| format!("App {}", app_id));
        let mut spans = vec![
            Span::styled(name, theme::text().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  ·  {} {}", t("select.appIdLabel"), app_id),
                theme::accent(),
            ),
            Span::styled(
                format!(
                    "  ·  {}",
                    tf("tui.select.depotsFound", &[("count", &depot_count)])
                ),
                theme::muted(),
            ),
        ];
        if self.wiz.game_info_loading {
            spans.push(Span::styled(
                format!("  {}", widgets::spinner(self.tick)),
                theme::accent(),
            ));
        }
        widgets::line(buf, take_top(&mut rest, 1), Line::from(spans));
        if let Some(d) = &self.wiz.short_description {
            widgets::text(buf, take_top(&mut rest, 1), d, theme::dim());
        }
        let _ = take_top(&mut rest, 1);

        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        let total: u64 = parsed
            .depots
            .iter()
            .filter(|d| self.wiz.selected.contains(&d.depot_id))
            .filter_map(|d| d.size_bytes)
            .sum();
        let n = self.wiz.selected.len();
        let mut label = if n > 0 {
            format!("{} ({})", t("tui.select.download"), n)
        } else {
            t("tui.select.download")
        };
        if total > 0 {
            label.push_str(&format!(" — ~{}", widgets::fmt_bytes(total)));
        }
        let back = ButtonSpec::new(
            format!("← {}", t("common.back")),
            Fid::new("select.back"),
            Action::BackToSource,
            Btn::Secondary,
        );
        let go = ButtonSpec::new(
            label,
            Fid::new("select.download"),
            Action::StartDownload,
            Btn::Primary,
        )
        .enabled(n > 0);
        widgets::buttons(buf, ctx, footer, &[back], false);
        let gw = widgets::button_width(&go.label);
        widgets::button(
            buf,
            ctx,
            footer.right().saturating_sub(gw),
            footer.y,
            gw,
            &go,
        );

        if rest.width >= 100 {
            let (left, right) = split_h(rest, rest.width * 48 / 100, 2);
            self.render_depot_list(buf, left, ctx);
            self.render_select_panel(buf, right, ctx);
        } else {
            let list_h = (rest.height / 2).max(8).min(rest.height);
            let left = take_top(&mut rest, list_h);
            let _ = take_top(&mut rest, 1);
            self.render_depot_list(buf, left, ctx);
            self.render_select_panel(buf, rest, ctx);
        }
    }

    fn render_depot_list(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        widgets::input(
            buf,
            ctx,
            take_top(&mut rest, 3),
            &mut self.wiz.depot_filter,
            InputSpec {
                label: &t("tui.select.filter"),
                placeholder: &t("select.depotSearchPlaceholder"),
                fid: Fid::new("select.filter"),
                id: InputId::DepotFilter,
                error: false,
            },
        );
        let row = take_top(&mut rest, 1);
        let mut x = row.x;
        x += widgets::button(
            buf,
            ctx,
            x,
            row.y,
            row.width,
            &ButtonSpec::new(
                t("select.selectAll"),
                Fid::new("select.all"),
                Action::SelectAll,
                Btn::Secondary,
            ),
        ) + 1;
        x += widgets::button(
            buf,
            ctx,
            x,
            row.y,
            row.right().saturating_sub(x),
            &ButtonSpec::new(
                t("select.deselectAll"),
                Fid::new("select.none"),
                Action::DeselectAll,
                Btn::Secondary,
            ),
        ) + 2;
        widgets::checkbox(
            buf,
            ctx,
            Rect::new(x, row.y, row.right().saturating_sub(x), 1),
            self.wiz.show_selected_only,
            &t("select.showSelectedOnly"),
            Fid::new("select.onlySelected"),
            Action::ToggleShowSelected,
            true,
        );
        let _ = take_top(&mut rest, 1);

        let list_fid = Fid::new("select.depots");
        let vis = self.wiz.visible_depots();
        if self.wiz.depot_cursor >= vis.len() {
            self.wiz.depot_cursor = vis.len().saturating_sub(1);
        }
        let focused = ctx.is_focused(list_fid);
        let card = widgets::card(buf, rest, None, focused);
        let range = widgets::list_window(
            self.wiz.depot_cursor,
            &mut self.wiz.depot_offset,
            vis.len(),
            card.height as usize,
        );
        let Some(parsed) = &self.wiz.parsed else {
            return;
        };
        for (row, pos) in range.enumerate() {
            let d = &parsed.depots[vis[pos]];
            let r = Rect::new(card.x, card.y + row as u16, card.width.saturating_sub(1), 1);
            let cursor = pos == self.wiz.depot_cursor;
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
            let checked = self.wiz.selected.contains(&d.depot_id);
            let pics = self.wiz.depot_pics.get(&d.depot_id);
            let meta = self.wiz.depot_meta.get(&d.depot_id);
            let mut spans = vec![
                Span::styled(
                    if checked { " [✓] " } else { " [ ] " },
                    if checked {
                        theme::success()
                    } else {
                        theme::dim()
                    },
                ),
                Span::styled(
                    d.depot_id.clone(),
                    if checked {
                        theme::text().add_modifier(Modifier::BOLD)
                    } else {
                        theme::text()
                    },
                ),
            ];
            if let Some(name) = pics
                .and_then(|p| p.name.clone())
                .filter(|n| !n.trim().is_empty())
            {
                spans.push(Span::styled(format!(" — {}", name.trim()), theme::dim()));
            }
            if let Some(p) = pics {
                if let Some((label, color)) = role_badge(p.role, p.language.as_deref()) {
                    spans.push(Span::raw(" "));
                    spans.push(widgets::badge(&label, color));
                }
            }
            for (tag, color) in os_tags(
                meta.and_then(|m| m.oslist.clone())
                    .or_else(|| pics.and_then(|p| p.oslist.clone())),
                meta.and_then(|m| m.osarch.clone()),
            ) {
                spans.push(Span::raw(" "));
                spans.push(widgets::badge(&tag, color));
            }
            widgets::line(buf, r, Line::from(spans));
            if let Some(sz) = d.size_bytes {
                let s = widgets::fmt_bytes(sz);
                let sw = widgets::width(&s) + 1;
                let sr = Rect::new(r.right().saturating_sub(sw), r.y, sw, 1);
                widgets::fill(
                    buf,
                    sr,
                    if cursor && focused {
                        theme::selection()
                    } else {
                        theme::base()
                    },
                );
                widgets::text(buf, sr, &s, theme::muted());
            }
            ctx.row(
                r,
                list_fid,
                Action::ListSelect(ListId::Depots, pos),
                Action::ToggleDepot(pos),
            );
            ctx.click(Rect::new(r.x, r.y, 5, 1), Action::ToggleDepot(pos));
        }
        if vis.is_empty() {
            widgets::text(
                buf,
                Rect::new(card.x + 1, card.y, card.width.saturating_sub(2), 1),
                &t("tui.select.noMatch"),
                theme::muted(),
            );
        }
        widgets::scrollbar(
            buf,
            card,
            vis.len(),
            card.height as usize,
            self.wiz.depot_offset,
        );
        ctx.focusable(list_fid, rest, Kind::List(ListId::Depots), None);
        ctx.scroll_region(rest, ScrollTarget::List(ListId::Depots));
        ctx.prefer(list_fid);
    }

    fn render_select_panel(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut sv = ScrollView::begin(ctx, area, 80, self.wiz.select_scroll);
        let w = sv.width();
        let vis = self.wiz.visible_depots();
        let cur = vis.get(self.wiz.depot_cursor).copied();
        let tick = self.tick;

        let update_dir = self
            .wiz
            .parsed
            .as_ref()
            .and_then(|p| self.wiz.active_update_dir(&p.main_app_id));
        if let Some(dir) = update_dir {
            let msg = tf("tui.select.updateNotice", &[("path", &dir)]);
            let h = widgets::status_height(&msg, w);
            let r_ = sv.next(h);
            widgets::status(&mut sv.buf, r_, Tone::Info, &msg, tick);
            sv.gap(1);
        }

        if let (Some(i), Some(parsed)) = (cur, self.wiz.parsed.as_mut()) {
            let d = &mut parsed.depots[i];
            let pics = self.wiz.depot_pics.get(&d.depot_id);
            let meta = self.wiz.depot_meta.get(&d.depot_id);
            let mut title = format!("Depot {}", d.depot_id);
            if let Some(n) = pics
                .and_then(|p| p.name.clone())
                .filter(|n| !n.trim().is_empty())
            {
                title.push_str(&format!(" — {}", n.trim()));
            }
            let r_ = sv.next(1);
            widgets::heading(&mut sv.buf, r_, &title);
            let mut info = vec![Span::styled(
                format!("{} {}", t("tui.select.manifest"), d.manifest_id),
                theme::dim(),
            )];
            if let Some(sz) = d.size_bytes.or_else(|| meta.and_then(|m| m.size_bytes)) {
                info.push(Span::styled(
                    format!("   {} {}", t("tui.select.size"), widgets::fmt_bytes(sz)),
                    theme::dim(),
                ));
            }
            if d.depot_key.is_none() {
                info.push(Span::styled(
                    format!("   {}", t("tui.select.noKey")),
                    theme::warning(),
                ));
            }
            let r_ = sv.next(1);
            widgets::line(&mut sv.buf, r_, Line::from(info));
            let mut tags: Vec<Span> = Vec::new();
            if let Some(p) = pics {
                if let Some((label, color)) = role_badge(p.role, p.language.as_deref()) {
                    tags.push(widgets::badge(&label, color));
                    tags.push(Span::raw(" "));
                }
            }
            for (tag, color) in os_tags(
                meta.and_then(|m| m.oslist.clone())
                    .or_else(|| pics.and_then(|p| p.oslist.clone())),
                meta.and_then(|m| m.osarch.clone()),
            ) {
                tags.push(widgets::badge(&tag, color));
                tags.push(Span::raw(" "));
            }
            if let Some(lang) = meta
                .and_then(|m| m.language.clone())
                .filter(|l| !l.is_empty())
            {
                tags.push(widgets::badge(&capitalize(&lang), theme::get().warning));
            }
            if !tags.is_empty() {
                let r_ = sv.next(1);
                widgets::line(&mut sv.buf, r_, Line::from(tags));
            }
            sv.gap(1);

            let fetch = ButtonSpec::new(
                t("tui.select.fetchLatest"),
                Fid::idx("select.fetch", i),
                Action::FetchLatest(i),
                Btn::Secondary,
            )
            .enabled(!d.fetching);
            let row = sv.next(3);
            super::source::input_with_button(
                &mut sv.buf,
                ctx,
                row,
                &mut d.custom_manifest,
                InputSpec {
                    label: &t("tui.select.customManifest"),
                    placeholder: &t("tui.select.customManifestPlaceholder"),
                    fid: Fid::idx("select.custom", i),
                    id: InputId::DepotManifest(i),
                    error: false,
                },
                &fetch,
            );
            let row = sv.next(1);
            let mut x = row.x;
            x += widgets::button(
                &mut sv.buf,
                ctx,
                x,
                row.y,
                w,
                &ButtonSpec::new(
                    t("tui.select.pickManifest"),
                    Fid::idx("select.pick", i),
                    Action::Browse(BrowsePurpose::DepotManifest(i)),
                    Btn::Secondary,
                ),
            ) + 2;
            if d.uploaded_manifest.is_some() {
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    x,
                    row.y,
                    w.saturating_sub(x - row.x),
                    &ButtonSpec::new(
                        t("tui.select.clearManifest"),
                        Fid::idx("select.clear", i),
                        Action::ClearDepotManifest(i),
                        Btn::Danger,
                    ),
                );
            }
            if let Some((tone, msg)) = &d.status {
                let h = widgets::status_height(msg, w);
                let r_ = sv.next(h);
                widgets::status(&mut sv.buf, r_, *tone, msg, tick);
            }
            sv.gap(1);
        }

        let r_ = sv.next(1);
        widgets::heading(&mut sv.buf, r_, &t("tui.select.options"));
        sv.gap(1);
        let browse = ButtonSpec::new(
            t("tui.browse.button"),
            Fid::new("select.browseDir"),
            Action::Browse(BrowsePurpose::DownloadDir),
            Btn::Secondary,
        );
        let row = sv.next(3);
        super::source::input_with_button(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.download_dir,
            InputSpec {
                label: &t("select.downloadLocation"),
                placeholder: &t("select.downloadLocationPlaceholder"),
                fid: Fid::new("select.dir"),
                id: InputId::DownloadDir,
                error: false,
            },
            &browse,
        );
        let h = widgets::wrap_height(&t("select.downloadLocationHint"), w);
        let r_ = sv.next(h);
        hint(&mut sv.buf, r_, &t("select.downloadLocationHint"));
        sv.gap(1);

        let mask_label = if self.wiz.mh_key.masked {
            t("tui.show")
        } else {
            t("tui.hide")
        };
        let mask = ButtonSpec::new(
            mask_label,
            Fid::new("select.mhMask"),
            Action::ToggleMask(InputId::MhKey),
            Btn::Secondary,
        );
        let row = sv.next(3);
        super::source::input_with_button(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.mh_key,
            InputSpec {
                label: &t("select.manifestHubLabel"),
                placeholder: &t("select.manifestHubPlaceholder"),
                fid: Fid::new("select.mh"),
                id: InputId::MhKey,
                error: self.wiz.mh_hint.is_some(),
            },
            &mask,
        );
        if let Some(h_) = &self.wiz.mh_hint {
            let h = widgets::status_height(h_, w);
            let r_ = sv.next(h);
            widgets::status(&mut sv.buf, r_, Tone::Warning, h_, tick);
        }
        let mh_hint = t("select.manifestHubHint");
        let h = widgets::wrap_height(&mh_hint, w);
        let r_ = sv.next(h);
        hint(&mut sv.buf, r_, &mh_hint);
        sv.gap(1);

        let engine = if self.settings.use_native_downloader {
            t("tui.select.engineNative")
        } else {
            t("tui.select.engineDdm")
        };
        let h = widgets::wrap_height(&engine, w);
        let r_ = sv.next(h);
        widgets::paragraph(&mut sv.buf, r_, &engine, theme::muted());
        if !self.settings.use_native_downloader {
            static DOTNET: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
            let installed = *DOTNET.get_or_init(|| smd_core::ops::system::check_dotnet().installed);
            if !installed {
                let msg = t("dotnetWarning.text");
                let h = widgets::status_height(&msg, w);
                let r_ = sv.next(h);
                widgets::status(&mut sv.buf, r_, Tone::Warning, &msg, tick);
            }
        }

        let mut scroll = self.wiz.select_scroll;
        sv.finish(buf, ctx, &mut scroll, ScrollTarget::Page);
        self.wiz.select_scroll = scroll;
    }
}

pub fn manifest_id_from_filename(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".manifest")?;
    let (depot, manifest) = stem.split_once('_')?;
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    (digits(depot) && digits(manifest)).then(|| manifest.to_string())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn role_badge(role: DepotRole, language: Option<&str>) -> Option<(String, ratatui::style::Color)> {
    let th = theme::get();
    Some(match role {
        DepotRole::Dlc => (t("depots.roleDlc"), th.purple),
        DepotRole::Language => (
            match language.filter(|l| !l.is_empty()) {
                Some(l) => tf("depots.roleLanguageWithName", &[("name", &capitalize(l))]),
                None => t("depots.roleLanguage"),
            },
            th.warning,
        ),
        DepotRole::SharedContent => (t("depots.roleContent"), th.success),
        DepotRole::Platform => (t("depots.rolePlatform"), th.accent),
        DepotRole::Other => return None,
    })
}

fn os_tags(oslist: Option<String>, osarch: Option<String>) -> Vec<(String, ratatui::style::Color)> {
    let th = theme::get();
    let os = oslist.unwrap_or_default().to_lowercase();
    let mut out = Vec::new();
    if os.contains("windows") {
        out.push((t("depotTags.windows"), th.accent));
    }
    if os.contains("linux") {
        out.push((t("depotTags.linux"), th.warning));
    }
    if os.contains("macos") || os.contains("mac") {
        out.push((t("depotTags.macos"), th.text_dim));
    }
    match osarch.as_deref() {
        Some("64") => out.push((t("depotTags.arch64"), th.text_dim)),
        Some("32") => out.push((t("depotTags.arch32"), th.text_dim)),
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_filenames() {
        assert_eq!(
            manifest_id_from_filename("228990_1234567.manifest"),
            Some("1234567".into())
        );
        assert_eq!(manifest_id_from_filename("foo.manifest"), None);
        assert_eq!(manifest_id_from_filename("228990_12a.manifest"), None);
    }
}
