use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

use super::action::{
    AcTarget, Action, BrowsePurpose, InputId, ListId, ScrollTarget, SourceTab, Step,
};
use super::state::{DepotRow, EmuState, Parsed};
use super::{apply, hint, App};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::input::TextInput;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{take_top, Ctx, Fid, Kind};

const DEBOUNCE: Duration = Duration::from_millis(400);

fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

pub fn is_valid_app_id(s: &str) -> bool {
    let v = s.trim();
    is_numeric(v)
        && !v.starts_with('0')
        && v.len() <= 10
        && v.parse::<u64>()
            .map(|n| n <= u32::MAX as u64)
            .unwrap_or(false)
}

impl App {
    pub(super) fn dispatch_source(&mut self, a: &Action) -> bool {
        match a {
            Action::SourceTab(tab) => {
                self.wiz.tab = *tab;
                self.wiz.ac = Default::default();
                let has_source = !self.settings.depot_sources.is_empty()
                    || !self.settings.hubcap_api_key.is_empty()
                    || !self.settings.ryuu_api_key.is_empty();
                self.focus = Some(Fid::new(match tab {
                    SourceTab::Upload => "upload.path",
                    SourceTab::Search if has_source => "search.input",
                    SourceTab::Search => "search.sourceInput",
                    SourceTab::PatchOnly => "patch.dir",
                }));
            }
            Action::Browse(p) => self.open_browser(*p),
            Action::LoadUpload => self.load_upload(),
            Action::SearchSubmit => self.perform_search(),
            Action::AutocompletePick(target, i) => self.autocomplete_pick(*target, *i),
            Action::SelectRepo(i) => {
                self.wiz.repo_selected = *i;
                self.wiz.repo_chosen = Some(*i);
            }
            Action::SearchNext => self.proceed_from_search(),
            Action::AddSource => self.add_source(),
            Action::PatchStart => self.start_patch_only(),
            _ => return false,
        }
        true
    }

    fn load_upload(&mut self) {
        let path = crate::ui::file_browser::expand_tilde(&self.wiz.upload_path.trimmed());
        let path_s = path.to_string_lossy().to_string();
        let lower = path_s.to_lowercase();
        if !(lower.ends_with(".lua") || lower.ends_with(".st")) {
            self.wiz.upload_error = Some(t("tui.upload.wrongType"));
            return;
        }
        self.wiz.upload_error = None;
        self.wiz.upload_loading = true;
        self.spawn(async move {
            let r = smd_core::ops::files::parse_source_file(&path_s).await;
            apply(move |app| {
                app.wiz.upload_loading = false;
                match r {
                    Ok(parsed) => {
                        let depots = parsed
                            .depots
                            .into_iter()
                            .map(|d| {
                                DepotRow::new(
                                    d.depot_id.to_string(),
                                    d.manifest_id,
                                    d.depot_key,
                                    d.size_bytes,
                                )
                            })
                            .collect::<Vec<_>>();
                        app.emit(
                            "lua_parsed",
                            Some(serde_json::json!({ "depot_count": depots.len() })),
                        );
                        app.wiz.parsed = Some(Parsed {
                            main_app_id: parsed
                                .main_app_id
                                .map(|i| i.to_string())
                                .unwrap_or_default(),
                            depots,
                            all_app_ids: parsed.all_app_ids.iter().map(|i| i.to_string()).collect(),
                        });
                        app.wiz.from_search = false;
                        app.wiz.source_type = None;
                        app.wiz.search_repo = None;
                        app.wiz.search_key_vdf = None;
                        app.wiz.game_name = None;
                        app.wiz.header_image = None;
                        app.wiz.short_description = None;
                        app.show_selection_step();
                    }
                    Err(e) => app.wiz.upload_error = Some(e),
                }
            })
        });
    }

    pub(super) fn perform_search(&mut self) {
        let mut raw = self.wiz.search_input.trimmed();
        if raw.is_empty() {
            return;
        }
        if !is_numeric(&raw) {
            let pick = self.wiz.ac.selected.unwrap_or(0);
            match self.wiz.ac.items.get(pick) {
                Some(hit) => {
                    raw = hit.app_id.to_string();
                    self.wiz.search_input.set(raw.clone());
                }
                None => {
                    self.wiz.search_error = Some(t("tui.search.invalidAppId"));
                    return;
                }
            }
        }
        let Ok(app_id) = raw.parse::<u64>() else {
            self.wiz.search_error = Some(t("tui.search.invalidAppId"));
            return;
        };
        if app_id == 0 {
            self.wiz.search_error = Some(t("tui.search.invalidAppId"));
            return;
        }
        let app_id = app_id.to_string();
        self.wiz.ac = Default::default();
        self.wiz.search_error = None;
        self.wiz.repos.clear();
        self.wiz.repo_chosen = None;
        self.wiz.repo_selected = 0;
        self.wiz.search_app_id = Some(app_id.clone());
        self.wiz.search_loading = true;
        self.wiz.game_name = None;
        self.wiz.header_image = None;
        self.wiz.short_description = None;
        self.fetch_game_info(app_id.clone());

        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::search::search_repos(&core, &dir, &app_id).await;
            apply(move |app| {
                app.wiz.search_loading = false;
                if app.wiz.search_app_id.as_deref() != Some(app_id.as_str()) {
                    return;
                }
                match r {
                    Ok(res) => {
                        let probe = res.source_probe.clone().unwrap_or_default();
                        let mut parts = probe.split(':');
                        app.emit(
                            "search_performed",
                            Some(serde_json::json!({
                                "found": !res.repos.is_empty(),
                                "repo_count": res.repos.len(),
                                "source_probe": parts.next().filter(|s| !s.is_empty()),
                                "probe_class": parts.next(),
                            })),
                        );
                        app.wiz.repos = res.repos;
                        if app.wiz.repos.is_empty() {
                            app.wiz.search_error = Some(t("search.noResults"));
                            app.wiz.auto_redownload = false;
                            return;
                        }
                        if app.wiz.repos.len() == 1 {
                            app.wiz.repo_chosen = Some(0);
                        }
                        if app.wiz.auto_redownload {
                            app.wiz.auto_redownload = false;
                            app.wiz.repo_chosen = Some(0);
                            app.proceed_from_search();
                        } else {
                            app.focus = Some(Fid::new("source.repos"));
                        }
                    }
                    Err(e) => {
                        app.emit("search_performed", Some(serde_json::json!({
                            "found": false, "repo_count": 0, "source_probe": "error", "probe_class": null,
                        })));
                        app.wiz.search_error = Some(e);
                        app.wiz.auto_redownload = false;
                    }
                }
            })
        });
    }

    pub(super) fn fetch_game_info(&mut self, app_id: String) {
        self.wiz.game_info_loading = true;
        let core = self.core.clone();
        self.spawn(async move {
            let r = smd_core::services::steam_store_api::get_game_info(
                &core.http_client,
                &core.steam_cache,
                &app_id,
            )
            .await;
            apply(move |app| {
                app.wiz.game_info_loading = false;
                if app.wiz.main_app_id().as_deref() != Some(app_id.as_str()) {
                    return;
                }
                if let Ok(Some(info)) = r {
                    if info.name.is_some() {
                        app.wiz.game_name = info.name;
                    }
                    if info.header_image.is_some() {
                        app.wiz.header_image = info.header_image;
                    }
                    app.wiz.short_description = info.short_description;
                }
            })
        });
    }

    fn proceed_from_search(&mut self) {
        let Some(idx) = self.wiz.repo_chosen else {
            return;
        };
        let Some(repo) = self.wiz.repos.get(idx).cloned() else {
            return;
        };
        let Some(app_id) = self.wiz.search_app_id.clone() else {
            return;
        };
        self.wiz.manifests_loading = true;
        self.wiz.search_error = None;
        let core = self.core.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::search::get_repo_manifests(
                &core,
                &dir,
                &app_id,
                &repo.repo,
                repo.sha.as_deref(),
            )
            .await;
            apply(move |app| {
                app.wiz.manifests_loading = false;
                match r {
                    Ok(m) => {
                        let depots: Vec<DepotRow> = m
                            .manifests
                            .into_iter()
                            .map(|x| {
                                DepotRow::new(
                                    x.depot_id,
                                    Some(x.manifest_id),
                                    x.depot_key,
                                    x.size_bytes,
                                )
                            })
                            .collect();
                        if depots.is_empty() {
                            app.wiz.search_error = Some(t("errors.noManifests"));
                            return;
                        }
                        app.wiz.search_repo = Some(repo.repo.clone());
                        app.wiz.search_key_vdf = Some(m.depot_keys);
                        app.wiz.source_type = Some(repo.source_type.clone());
                        app.wiz.parsed = Some(Parsed {
                            main_app_id: app_id,
                            depots,
                            all_app_ids: Vec::new(),
                        });
                        app.wiz.from_search = true;
                        app.show_selection_step();
                    }
                    Err(e) => app.wiz.search_error = Some(e),
                }
            })
        });
    }

    fn add_source(&mut self) {
        let raw = self.wiz.source_add.trimmed();
        match self.validate_source(&raw, &self.settings.depot_sources) {
            Err(e) => self.wiz.source_error = Some(e),
            Ok(()) => {
                self.wiz.source_error = None;
                self.wiz.source_add.clear();
                let mut s = self.settings.clone();
                s.depot_sources.push(raw);
                self.persist_settings(s);
            }
        }
    }

    pub(super) fn validate_source(&self, url: &str, existing: &[String]) -> Result<(), String> {
        if url.is_empty() {
            return Err(t("errors.sourceEmpty"));
        }
        let lower = url.to_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err(t("errors.sourceProtocol"));
        }
        if existing.iter().any(|s| s == url) {
            return Err(t("tui.sources.duplicate"));
        }
        Ok(())
    }

    pub(super) fn persist_settings(&mut self, s: smd_core::services::settings::Settings) {
        self.settings = s.clone();
        self.set.draft.depot_sources = s.depot_sources.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::services::settings::save_settings(&dir, &s).await;
            apply(move |app| {
                if let Err(e) = r {
                    app.toast(Tone::Error, e);
                }
            })
        });
    }

    fn start_patch_only(&mut self) {
        self.wiz.ac = Default::default();
        self.wiz.patch_error = None;
        let dir = crate::ui::file_browser::expand_tilde(&self.wiz.patch_dir.trimmed())
            .to_string_lossy()
            .to_string();
        let app_id = self.wiz.patch_app_id.trimmed();
        if dir.is_empty() {
            self.wiz.patch_error = Some(t("patchOnly.errorNoFolder"));
            return;
        }
        if !is_valid_app_id(&app_id) {
            self.wiz.patch_error = Some(t("patchOnly.errorAppId"));
            return;
        }
        self.wiz.patch_loading = true;
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| {
                app.wiz.patch_loading = false;
                match r {
                    Err(e) => {
                        app.wiz.patch_error = Some(tf("patchOnly.errorScan", &[("message", &e)]))
                    }
                    Ok(scan) if scan.is_empty() => {
                        app.wiz.patch_error = Some(t("patchOnly.errorNoApiFiles"))
                    }
                    Ok(scan) => {
                        let unpatched: Vec<String> = scan
                            .iter()
                            .filter(|f| !f.is_patched)
                            .map(|f| f.path.clone())
                            .collect();
                        let prefill = unpatched.len() == scan.len();
                        let mut emu = EmuState::new(dir.clone(), app_id.clone());
                        emu.standalone = true;
                        emu.selection_explicit = true;
                        emu.selected = unpatched.into_iter().collect();
                        app.wiz.result_dir = Some(dir);
                        app.wiz.parsed = Some(Parsed {
                            main_app_id: app_id,
                            depots: Vec::new(),
                            all_app_ids: Vec::new(),
                        });
                        app.emulator_available = true;
                        app.enter_emulator(emu, scan, prefill);
                    }
                }
            })
        });
    }

    pub(super) fn autocomplete_typed(&mut self, target: AcTarget) {
        let val = match target {
            AcTarget::Search => self.wiz.search_input.trimmed(),
            AcTarget::PatchOnly => self.wiz.patch_app_id.trimmed(),
        };
        if val.is_empty() || is_numeric(&val) || val.chars().count() < 2 {
            self.wiz.ac = Default::default();
            return;
        }
        self.wiz.ac.target = Some(target);
        self.wiz.ac.pending = Some((val, Instant::now()));
    }

    pub(super) fn autocomplete_tick(&mut self) {
        let Some((query, at)) = self.wiz.ac.pending.clone() else {
            return;
        };
        if at.elapsed() < DEBOUNCE {
            return;
        }
        self.wiz.ac.pending = None;
        self.wiz.ac.loading = true;
        self.wiz.ac.last_query = query.clone();
        let core = self.core.clone();
        self.spawn(async move {
            let r = smd_core::ops::search::search_steam_games(&core.http_client, &query).await;
            apply(move |app| {
                if app.wiz.ac.last_query != query || app.wiz.ac.target.is_none() {
                    return;
                }
                app.wiz.ac.loading = false;
                match r {
                    Ok(items) => {
                        app.wiz.ac.items = items;
                        app.wiz.ac.selected = None;
                    }
                    Err(_) => app.wiz.ac = Default::default(),
                }
            })
        });
    }

    pub(super) fn autocomplete_key(&mut self, id: InputId, key: KeyEvent) -> bool {
        let target = match id {
            InputId::Search => AcTarget::Search,
            InputId::PatchAppId => AcTarget::PatchOnly,
            _ => return false,
        };
        if self.wiz.ac.target != Some(target) {
            return false;
        }
        let n = self.wiz.ac.items.len();
        match key.code {
            KeyCode::Down if n > 0 => {
                self.wiz.ac.selected = Some(
                    self.wiz
                        .ac
                        .selected
                        .map(|s| (s + 1).min(n - 1))
                        .unwrap_or(0),
                );
                true
            }
            KeyCode::Up if n > 0 => {
                self.wiz.ac.selected = self.wiz.ac.selected.and_then(|s| s.checked_sub(1));
                true
            }
            KeyCode::Enter => match self.wiz.ac.selected {
                Some(i) => {
                    self.autocomplete_pick(target, i);
                    true
                }
                None => false,
            },
            KeyCode::Esc => {
                self.wiz.ac = Default::default();
                true
            }
            _ => false,
        }
    }

    fn autocomplete_pick(&mut self, target: AcTarget, idx: usize) {
        let Some(hit) = self.wiz.ac.items.get(idx).cloned() else {
            return;
        };
        self.wiz.ac = Default::default();
        match target {
            AcTarget::Search => {
                self.wiz.search_input.set(hit.app_id.to_string());
                self.perform_search();
            }
            AcTarget::PatchOnly => {
                self.wiz.patch_app_id.set(hit.app_id.to_string());
                self.wiz.patch_error = None;
            }
        }
    }

    pub(super) fn show_selection_step(&mut self) {
        let Some(parsed) = &self.wiz.parsed else {
            return;
        };
        let app_id = parsed.main_app_id.clone();
        self.wiz.selected.clear();
        self.wiz.depot_cursor = 0;
        self.wiz.depot_offset = 0;
        self.wiz.depot_filter.clear();
        self.wiz.show_selected_only = false;
        self.wiz.depot_meta.clear();
        self.wiz.depot_pics.clear();
        self.wiz.mh_hint = None;
        self.wiz.select_scroll = 0;
        if self.wiz.download_dir.is_empty() {
            self.wiz
                .download_dir
                .set(self.settings.download_location.clone());
        }
        if let Some(sel) = self.wiz.auto_select.take() {
            match sel {
                Some(ids) if !ids.is_empty() => {
                    for d in &parsed.depots {
                        if ids.contains(&d.depot_id) {
                            self.wiz.selected.insert(d.depot_id.clone());
                        }
                    }
                }
                _ => {
                    self.wiz.selected = parsed.depots.iter().map(|d| d.depot_id.clone()).collect();
                }
            }
        }
        if !self.wiz.from_search || self.wiz.game_name.is_none() {
            self.fetch_game_info(app_id.clone());
        }
        self.wiz.step = Step::Select;
        self.focus = None;

        let core = self.core.clone();
        let dir = self.data_dir.clone();
        let id = app_id.clone();
        self.spawn(async move {
            let r = smd_core::services::depot_info::fetch_depot_info(&core.http_client, &dir, &id)
                .await;
            apply(move |app| {
                if let Ok(list) = r {
                    if app.wiz.main_app_id().as_deref() == Some(id.as_str()) {
                        app.wiz.depot_meta =
                            list.into_iter().map(|d| (d.depot_id.clone(), d)).collect();
                    }
                }
            })
        });
        let core = self.core.clone();
        self.spawn(async move {
            let r = match app_id.parse::<u32>() {
                Ok(n) => {
                    smd_core::services::steam_pics::fetch_depots_with_names(
                        core.steam_session.clone(),
                        n,
                    )
                    .await
                }
                Err(_) => Err("invalid app id".into()),
            };
            apply(move |app| {
                if let Ok(list) = r {
                    if app.wiz.main_app_id().as_deref() == Some(app_id.as_str()) {
                        app.wiz.depot_pics =
                            list.into_iter().map(|d| (d.depot_id.clone(), d)).collect();
                    }
                }
            })
        });
    }

    pub(super) fn render_source(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let tabs_row = take_top(&mut rest, 1);
        let tabs = [
            (SourceTab::Upload, t("tabs.upload")),
            (SourceTab::Search, t("tabs.search")),
            (SourceTab::PatchOnly, t("tabs.patchOnly")),
        ];
        let mut x = tabs_row.x;
        for (tab, label) in tabs {
            let active = self.wiz.tab == tab;
            let fid = Fid::idx("source.tab", tab_index(tab));
            let w = widgets::width(&label) + 4;
            let rect = Rect::new(x, tabs_row.y, w.min(tabs_row.right().saturating_sub(x)), 1);
            let focused = ctx.is_focused(fid);
            let hovered = ctx.is_hovered(rect);
            let t_ = theme::get();
            let style = if active {
                if t_.truecolor {
                    ratatui::style::Style::default()
                        .bg(t_.surface_hi)
                        .fg(t_.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    theme::accent_bold().add_modifier(Modifier::UNDERLINED)
                }
            } else if focused || hovered {
                theme::text().add_modifier(Modifier::BOLD)
            } else {
                theme::dim()
            };
            let mark = if focused { "▸" } else { " " };
            buf.set_stringn(
                x,
                tabs_row.y,
                format!("{} {}  ", mark, label),
                rect.width as usize,
                style,
            );
            ctx.focusable(fid, rect, Kind::Button, Some(Action::SourceTab(tab)));
            x += w + 1;
        }
        let rule_row = take_top(&mut rest, 1);
        buf.set_string(
            rule_row.x,
            rule_row.y,
            "─".repeat(rule_row.width as usize),
            theme::border(false),
        );
        let _ = take_top(&mut rest, 1);

        match self.wiz.tab {
            SourceTab::Upload => self.render_upload_tab(buf, rest, ctx),
            SourceTab::Search => self.render_search_tab(buf, rest, ctx),
            SourceTab::PatchOnly => self.render_patch_tab(buf, rest, ctx),
        }
    }

    fn render_upload_tab(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let lead = format!("{} {}", t("upload.dropText"), t("tui.upload.dropHint"));
        let h = widgets::wrap_height(&lead, rest.width);
        widgets::paragraph(buf, take_top(&mut rest, h), &lead, theme::text());
        let _ = take_top(&mut rest, 1);
        let browse = ButtonSpec::new(
            t("tui.browse.button"),
            Fid::new("upload.browse"),
            Action::Browse(BrowsePurpose::UploadFile),
            Btn::Secondary,
        );
        input_with_button(
            buf,
            ctx,
            take_top(&mut rest, 3),
            &mut self.wiz.upload_path,
            InputSpec {
                label: &t("tui.upload.pathLabel"),
                placeholder: &t("tui.upload.pathPlaceholder"),
                fid: Fid::new("upload.path"),
                id: InputId::UploadPath,
                error: self.wiz.upload_error.is_some(),
            },
            &browse,
        );
        let _ = take_top(&mut rest, 1);
        let load = ButtonSpec::new(
            t("tui.upload.load"),
            Fid::new("upload.load"),
            Action::LoadUpload,
            Btn::Primary,
        )
        .enabled(!self.wiz.upload_path.is_empty() && !self.wiz.upload_loading);
        widgets::buttons(buf, ctx, take_top(&mut rest, 1), &[load], false);
        let _ = take_top(&mut rest, 1);
        if self.wiz.upload_loading {
            widgets::status(
                buf,
                take_top(&mut rest, 1),
                Tone::Busy,
                &t("upload.parsing"),
                self.tick,
            );
        } else if let Some(e) = &self.wiz.upload_error {
            let h = widgets::status_height(e, rest.width);
            widgets::status(buf, take_top(&mut rest, h), Tone::Error, e, self.tick);
        }
        ctx.prefer(Fid::new("upload.path"));
    }

    fn render_search_tab(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let has_source = !self.settings.depot_sources.is_empty()
            || !self.settings.hubcap_api_key.is_empty()
            || !self.settings.ryuu_api_key.is_empty();
        if !has_source {
            widgets::heading(buf, take_top(&mut rest, 1), &t("search.noSourceTitle"));
            let lead = t("search.noSourceLead");
            let h = widgets::wrap_height(&lead, rest.width);
            widgets::paragraph(buf, take_top(&mut rest, h), &lead, theme::dim());
            let _ = take_top(&mut rest, 1);
            let add = ButtonSpec::new(
                t("search.addSource"),
                Fid::new("search.addSource"),
                Action::AddSource,
                Btn::Primary,
            );
            input_with_button(
                buf,
                ctx,
                take_top(&mut rest, 3),
                &mut self.wiz.source_add,
                InputSpec {
                    label: &t("settings.depotSources"),
                    placeholder: &t("search.sourcePlaceholder"),
                    fid: Fid::new("search.sourceInput"),
                    id: InputId::SourceAdd,
                    error: self.wiz.source_error.is_some(),
                },
                &add,
            );
            if let Some(e) = &self.wiz.source_error {
                widgets::status(buf, take_top(&mut rest, 1), Tone::Error, e, 0);
            }
            ctx.prefer(Fid::new("search.sourceInput"));
            return;
        }

        let submit = ButtonSpec::new(
            t("search.submit"),
            Fid::new("search.submit"),
            Action::SearchSubmit,
            Btn::Primary,
        )
        .enabled(!self.wiz.search_loading);
        let input_area = take_top(&mut rest, 3);
        input_with_button(
            buf,
            ctx,
            input_area,
            &mut self.wiz.search_input,
            InputSpec {
                label: &t("tabs.search"),
                placeholder: &t("search.inputPlaceholder"),
                fid: Fid::new("search.input"),
                id: InputId::Search,
                error: false,
            },
            &submit,
        );
        ctx.prefer(Fid::new("search.input"));

        if let Some(name) = &self.wiz.game_name {
            let _ = take_top(&mut rest, 1);
            let id = self.wiz.search_app_id.clone().unwrap_or_default();
            widgets::line(
                buf,
                take_top(&mut rest, 1),
                Line::from(vec![
                    Span::styled(name.clone(), theme::text().add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  ·  App {}", id), theme::muted()),
                ]),
            );
            if let Some(d) = &self.wiz.short_description {
                let lines = widgets::wrap(d, rest.width);
                for l in lines.iter().take(2) {
                    widgets::text(buf, take_top(&mut rest, 1), l, theme::dim());
                }
            }
        }
        let _ = take_top(&mut rest, 1);

        if self.wiz.search_loading {
            widgets::status(
                buf,
                take_top(&mut rest, 1),
                Tone::Busy,
                &t("search.loading"),
                self.tick,
            );
        } else if self.wiz.manifests_loading {
            widgets::status(
                buf,
                take_top(&mut rest, 1),
                Tone::Busy,
                &t("search.manifestLoading"),
                self.tick,
            );
        }
        if let Some(e) = &self.wiz.search_error {
            let h = widgets::status_height(e, rest.width).min(3);
            widgets::status(buf, take_top(&mut rest, h), Tone::Error, e, 0);
        }

        if !self.wiz.repos.is_empty() {
            let footer = crate::ui::take_bottom(&mut rest, 1);
            let _ = crate::ui::take_bottom(&mut rest, 1);
            widgets::heading(
                buf,
                take_top(&mut rest, 1),
                &tf("tui.search.results", &[("count", &self.wiz.repos.len())]),
            );
            self.render_repo_list(buf, rest, ctx);
            let next = ButtonSpec::new(
                t("search.next"),
                Fid::new("search.next"),
                Action::SearchNext,
                Btn::Primary,
            )
            .enabled(self.wiz.repo_chosen.is_some() && !self.wiz.manifests_loading);
            widgets::buttons(buf, ctx, footer, &[next], true);
        } else {
            let n = self.settings.depot_sources.len();
            let note = tf("tui.search.sourcesNote", &[("count", &n)]);
            hint(buf, take_top(&mut rest, 2), &note);
        }

        if self.wiz.ac.target == Some(AcTarget::Search) {
            render_autocomplete(
                buf,
                ctx,
                input_area,
                &self.wiz.ac,
                AcTarget::Search,
                self.tick,
            );
        }
    }

    fn render_repo_list(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let fid = Fid::new("source.repos");
        let len = self.wiz.repos.len();
        let range = widgets::list_window(
            self.wiz.repo_selected,
            &mut self.wiz.repo_offset,
            len,
            area.height as usize,
        );
        let focused = ctx.is_focused(fid);
        for (row, i) in range.enumerate() {
            let repo = &self.wiz.repos[i];
            let r = Rect::new(area.x, area.y + row as u16, area.width.saturating_sub(1), 1);
            let cursor = i == self.wiz.repo_selected;
            if cursor && focused {
                widgets::fill(buf, r, theme::selection());
            } else if ctx.is_hovered(r) {
                widgets::fill(buf, r, theme::surface());
            }
            let chosen = self.wiz.repo_chosen == Some(i);
            let (name, badge) = match repo.source_type.as_str() {
                "hubcap" => (t("search.repoNameHubcap"), t("search.repoBadgeHubcap")),
                "remote" => (t("search.repoNameRemote"), t("search.repoBadgeRemote")),
                other => (
                    repo.repo.clone(),
                    repo.source.clone().unwrap_or_else(|| other.to_string()),
                ),
            };
            let mut spans = vec![
                Span::styled(
                    if chosen { " (●) " } else { " ( ) " },
                    if chosen {
                        theme::accent_bold()
                    } else {
                        theme::dim()
                    },
                ),
                Span::styled(
                    name,
                    if chosen {
                        theme::text().add_modifier(Modifier::BOLD)
                    } else {
                        theme::text()
                    },
                ),
                Span::raw("  "),
                widgets::badge(&badge, theme::get().purple),
            ];
            if let Some(d) = &repo.date {
                spans.push(Span::styled(
                    format!(
                        "  {}: {}",
                        t("search.repoUpdated"),
                        d.replace('T', " ").trim_end_matches('Z')
                    ),
                    theme::muted(),
                ));
            }
            widgets::line(buf, r, Line::from(spans));
            ctx.row(
                r,
                fid,
                Action::SelectRepo(i),
                Action::ListActivate(ListId::Repos, i),
            );
        }
        widgets::scrollbar(buf, area, len, area.height as usize, self.wiz.repo_offset);
        ctx.focusable(fid, area, Kind::List(ListId::Repos), None);
        ctx.scroll_region(area, ScrollTarget::List(ListId::Repos));
    }

    fn render_patch_tab(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let lead = t("patchOnly.lead");
        let h = widgets::wrap_height(&lead, rest.width).min(4);
        widgets::paragraph(buf, take_top(&mut rest, h), &lead, theme::dim());
        let _ = take_top(&mut rest, 1);
        let browse = ButtonSpec::new(
            t("tui.browse.button"),
            Fid::new("patch.browse"),
            Action::Browse(BrowsePurpose::PatchDir),
            Btn::Secondary,
        );
        input_with_button(
            buf,
            ctx,
            take_top(&mut rest, 3),
            &mut self.wiz.patch_dir,
            InputSpec {
                label: &t("patchOnly.folderLabel"),
                placeholder: &t("patchOnly.folderPlaceholder"),
                fid: Fid::new("patch.dir"),
                id: InputId::PatchDir,
                error: false,
            },
            &browse,
        );
        let h1 = widgets::wrap_height(&t("patchOnly.folderHint"), rest.width).min(2);
        hint(buf, take_top(&mut rest, h1), &t("patchOnly.folderHint"));
        let _ = take_top(&mut rest, 1);
        let id_area = take_top(&mut rest, 3);
        widgets::input(
            buf,
            ctx,
            id_area,
            &mut self.wiz.patch_app_id,
            InputSpec {
                label: &t("patchOnly.appIdLabel"),
                placeholder: &t("patchOnly.appIdPlaceholder"),
                fid: Fid::new("patch.appid"),
                id: InputId::PatchAppId,
                error: false,
            },
        );
        let h2 = widgets::wrap_height(&t("patchOnly.appIdHint"), rest.width).min(2);
        hint(buf, take_top(&mut rest, h2), &t("patchOnly.appIdHint"));
        let _ = take_top(&mut rest, 1);
        let start = ButtonSpec::new(
            t("patchOnly.submit"),
            Fid::new("patch.start"),
            Action::PatchStart,
            Btn::Primary,
        )
        .enabled(!self.wiz.patch_loading);
        widgets::buttons(buf, ctx, take_top(&mut rest, 1), &[start], false);
        let _ = take_top(&mut rest, 1);
        if self.wiz.patch_loading {
            widgets::status(
                buf,
                take_top(&mut rest, 1),
                Tone::Busy,
                &t("patchOnly.scanning"),
                self.tick,
            );
        } else if let Some(e) = &self.wiz.patch_error {
            let h = widgets::status_height(e, rest.width).min(3);
            widgets::status(buf, take_top(&mut rest, h), Tone::Error, e, 0);
        }
        ctx.prefer(Fid::new("patch.dir"));
        if self.wiz.ac.target == Some(AcTarget::PatchOnly) {
            render_autocomplete(
                buf,
                ctx,
                id_area,
                &self.wiz.ac,
                AcTarget::PatchOnly,
                self.tick,
            );
        }
    }
}

fn tab_index(tab: SourceTab) -> usize {
    match tab {
        SourceTab::Upload => 0,
        SourceTab::Search => 1,
        SourceTab::PatchOnly => 2,
    }
}

pub fn input_with_button(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    area: Rect,
    input: &mut TextInput,
    spec: InputSpec,
    button: &ButtonSpec,
) {
    let bw = widgets::button_width(&button.label);
    let input_w = area.width.saturating_sub(bw + 2);
    widgets::input(buf, ctx, Rect::new(area.x, area.y, input_w, 3), input, spec);
    widgets::button(buf, ctx, area.x + input_w + 2, area.y + 1, bw, button);
}

fn render_autocomplete(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    input_area: Rect,
    ac: &super::state::Autocomplete,
    target: AcTarget,
    tick: u64,
) {
    let rows = if ac.loading || ac.items.is_empty() {
        1
    } else {
        ac.items.len().min(8) as u16
    };
    let width = input_area
        .width
        .saturating_sub(10)
        .max(30)
        .min(input_area.width);
    let rect = Rect::new(input_area.x + 1, input_area.y + 3, width, rows + 2);
    widgets::fill(buf, rect, theme::surface());
    let inner = widgets::card(buf, rect, None, true);
    if theme::get().truecolor {
        widgets::fill(buf, inner, theme::surface());
    }
    if ac.loading || ac.pending.is_some() {
        widgets::status(buf, inner, Tone::Busy, &t("tui.search.suggesting"), tick);
        return;
    }
    if ac.items.is_empty() {
        widgets::text(
            buf,
            Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(1), 1),
            &t("tui.search.noGames"),
            theme::muted(),
        );
        return;
    }
    for (i, item) in ac.items.iter().take(8).enumerate() {
        let r = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
        if ac.selected == Some(i) || ctx.is_hovered(r) {
            widgets::fill(buf, r, theme::selection());
        }
        let id = item.app_id.to_string();
        let name_w = r.width.saturating_sub(widgets::width(&id) + 3);
        buf.set_string(
            r.x + 1,
            r.y,
            widgets::truncate(&item.name, name_w),
            theme::text(),
        );
        widgets::text_right(
            buf,
            Rect::new(r.x, r.y, r.width.saturating_sub(1), 1),
            &id,
            theme::muted(),
        );
        ctx.click(r, Action::AutocompletePick(target, i));
    }
}
