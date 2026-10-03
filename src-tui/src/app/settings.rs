use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use smd_core::services::settings::{self as settings_service, TelemetryConsent};

use super::action::{Action, BrowsePurpose, InputId, ListId, ScrollTarget, SettingToggle};
use super::state::*;
use super::{apply, hint, App, Modal, VERSION};
use crate::i18n::{self, t, tf};
use crate::term;
use crate::theme::{self, ThemeMode};
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{take_bottom, take_top, Ctx, Fid, Kind, ScrollView};

const TABS: [&str; 5] = [
    "tui.settings.tabGeneral",
    "tui.settings.tabDownloads",
    "settings.depotSources",
    "tui.settings.tabKeys",
    "tui.settings.tabAbout",
];

pub fn build_info() -> Vec<(String, String)> {
    let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local");
    let channel_label = match channel {
        "stable" => "Stable",
        "dev" => "Dev",
        "dev-local" => "Dev (local)",
        other => other,
    };
    vec![
        (t("settings.debugInfoChannel"), channel_label.to_string()),
        (t("settings.debugInfoVersion"), VERSION.to_string()),
        (
            t("settings.debugInfoCommit"),
            option_env!("SMD_GIT_SHA").unwrap_or("unknown").to_string(),
        ),
        (
            t("settings.debugInfoBuildDate"),
            option_env!("SMD_BUILD_DATE")
                .unwrap_or("unknown")
                .to_string(),
        ),
        (
            t("settings.debugInfoProfile"),
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
        ),
        (
            t("settings.debugInfoPlatform"),
            format!(
                "{} / {} (TUI)",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
        ),
    ]
}

impl App {
    pub(super) fn dispatch_settings(&mut self, a: &Action) -> bool {
        match a {
            Action::SettingsTab(i) => {
                self.set.tab = *i;
                self.set.scroll = 0;
                self.focus = Some(Fid::idx("settings.tab", *i));
            }
            Action::SettingsToggle(which) => {
                let d = &mut self.set.draft;
                match which {
                    SettingToggle::AutoUpdate => d.auto_update = !d.auto_update,
                    SettingToggle::NativeDownloader => {
                        d.use_native_downloader = !d.use_native_downloader
                    }
                    SettingToggle::CancelKeepFiles => d.cancel_keep_files = !d.cancel_keep_files,
                    SettingToggle::Telemetry => {
                        let accept = self.settings.telemetry_consent != TelemetryConsent::Accepted;
                        let dir = self.data_dir.clone();
                        self.spawn(async move {
                            let r =
                                smd_core::ops::consent::set_telemetry_consent(&dir, accept).await;
                            let s = settings_service::load_settings(&dir).await;
                            apply(move |app| {
                                if let Err(e) = r {
                                    app.toast(Tone::Error, e);
                                }
                                if accept {
                                    app.emit("consent_accepted", None);
                                }
                                app.settings.telemetry_consent = s.telemetry_consent.clone();
                                app.settings.installation_id = s.installation_id.clone();
                                app.set.draft.telemetry_consent = s.telemetry_consent;
                                app.set.draft.installation_id = s.installation_id;
                            })
                        });
                        return true;
                    }
                }
                self.set.dirty = true;
            }
            Action::SettingsLanguage(code) => self.set_language(code),
            Action::SettingsTheme(mode) => {
                self.prefs.theme = *mode;
                theme::apply(*mode);
                self.save_prefs();
            }
            Action::SettingsAddSource => {
                let raw = self.set.source_add.trimmed();
                match self.validate_source(&raw, &self.settings.depot_sources) {
                    Err(e) => self.set.source_error = Some(e),
                    Ok(()) => {
                        self.set.source_error = None;
                        self.set.source_add.clear();
                        let mut s = self.settings.clone();
                        s.depot_sources.push(raw);
                        self.persist_settings(s);
                    }
                }
            }
            Action::SettingsRemoveSource(i) => {
                let Some(url) = self.settings.depot_sources.get(*i).cloned() else {
                    return true;
                };
                if self.settings.pristine_default_sources.contains(&url) {
                    self.queue_modal(Modal::confirm(
                        t("modals.removeDefaultSource.title"),
                        t("modals.removeDefaultSource.body"),
                        t("modals.removeDefaultSource.yes"),
                        t("modals.removeDefaultSource.no"),
                        true,
                        Action::SettingsRemoveSourceConfirmed(*i),
                    ));
                } else {
                    self.remove_source_now(*i);
                }
            }
            Action::SettingsRemoveSourceConfirmed(i) => {
                self.close_modal();
                self.remove_source_now(*i);
            }
            Action::SettingsSave => self.save_settings_page(),
            Action::SettingsRevert => {
                let s = self.settings.clone();
                let mh = self.prefs.mh_api_key.clone();
                self.set.load(&s, &mh);
                self.set.status = None;
            }
            Action::CopyBuildInfo => {
                let text = build_info()
                    .into_iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect::<Vec<_>>()
                    .join("\n");
                term::copy_to_clipboard(&text);
                self.toast(Tone::Success, t("tui.settings.copied"));
            }
            Action::CheckUpdates => self.check_updates(true),
            _ => return false,
        }
        true
    }

    fn remove_source_now(&mut self, idx: usize) {
        if idx >= self.settings.depot_sources.len() {
            return;
        }
        let mut s = self.settings.clone();
        s.depot_sources.remove(idx);
        self.persist_settings(s);
        let n = self.settings.depot_sources.len();
        if self.set.source_selected >= n {
            self.set.source_selected = n.saturating_sub(1);
        }
    }

    fn save_settings_page(&mut self) {
        let inputs: Vec<String> = self.set.inputs.iter().map(|i| i.trimmed()).collect();
        if smd_core::services::speed_limit::parse_speed_limit(&inputs[SETTING_SPEED]).is_err() {
            self.set.status = Some((Tone::Error, t("settings.speedLimitInvalid")));
            return;
        }
        let draft = self.set.draft.clone();
        let sources = self.settings.depot_sources.clone();
        let old_default = self.settings.download_location.clone();
        self.prefs.mh_api_key = inputs[SETTING_MH].clone();
        self.wiz.mh_key.set(inputs[SETTING_MH].clone());
        self.save_prefs();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let mut s = settings_service::load_settings(&dir).await;
            if !inputs[SETTING_DOWNLOAD_DIR].is_empty() {
                s.download_location = inputs[SETTING_DOWNLOAD_DIR].clone();
            }
            s.native_chunk_concurrency = inputs[SETTING_CHUNKS]
                .parse()
                .ok()
                .filter(|n| *n > 0)
                .unwrap_or(8);
            s.max_retries = inputs[SETTING_RETRIES].parse().unwrap_or(3);
            s.dd_extra_args = if inputs[SETTING_DD_ARGS].is_empty() {
                vec!["-max-downloads".into(), "8".into(), "-verify-all".into()]
            } else {
                inputs[SETTING_DD_ARGS]
                    .split_whitespace()
                    .map(String::from)
                    .collect()
            };
            s.download_speed_limit = inputs[SETTING_SPEED].clone();
            s.proxy = inputs[SETTING_PROXY].clone();
            s.hubcap_api_key = inputs[SETTING_HUBCAP].clone();
            s.ryuu_api_key = inputs[SETTING_RYUU].clone();
            s.auto_update = draft.auto_update;
            s.use_native_downloader = draft.use_native_downloader;
            s.cancel_keep_files = draft.cancel_keep_files;
            s.depot_sources = sources;
            let r = settings_service::save_settings(&dir, &s).await;
            apply(move |app| match r {
                Ok(()) => {
                    if app.wiz.download_dir.trimmed() == old_default {
                        app.wiz.download_dir.set(s.download_location.clone());
                    }
                    app.settings = s.clone();
                    let mh = app.prefs.mh_api_key.clone();
                    app.set.load(&s, &mh);
                    app.set.status = Some((Tone::Success, t("tui.settings.saved")));
                }
                Err(e) => app.set.status = Some((Tone::Error, e)),
            })
        });
    }

    pub(super) fn render_settings(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let tabs = take_top(&mut rest, 1);
        let mut x = tabs.x;
        for (i, key) in TABS.iter().enumerate() {
            let label = t(key);
            let kind = if self.set.tab == i {
                Btn::Primary
            } else {
                Btn::Secondary
            };
            let room = tabs.right().saturating_sub(x);
            if room < 6 {
                break;
            }
            x += widgets::button(
                buf,
                ctx,
                x,
                tabs.y,
                room,
                &ButtonSpec::new(
                    label,
                    Fid::idx("settings.tab", i),
                    Action::SettingsTab(i),
                    kind,
                ),
            ) + 1;
        }
        let _ = take_top(&mut rest, 1);

        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        if let Some((tone, msg)) = &self.set.status {
            let r = take_bottom(&mut rest, 1);
            widgets::status(buf, r, *tone, msg, self.tick);
        }
        let dirty = self.set.dirty;
        let mut save = t("settings.save");
        if dirty {
            save.push_str(" •");
        }
        let specs = [
            ButtonSpec::new(
                t("tui.settings.revert"),
                Fid::new("settings.revert"),
                Action::SettingsRevert,
                Btn::Secondary,
            )
            .enabled(dirty),
            ButtonSpec::new(
                save,
                Fid::new("settings.save"),
                Action::SettingsSave,
                Btn::Primary,
            ),
        ];
        widgets::buttons(buf, ctx, footer, &specs, true);

        if self.set.tab == 2 {
            self.render_sources_tab(buf, rest, ctx);
            return;
        }
        let mut sv = ScrollView::begin(ctx, rest, 120, self.set.scroll);
        let w = sv.width();
        match self.set.tab {
            0 => self.render_general(&mut sv, ctx, w),
            1 => self.render_downloads(&mut sv, ctx, w),
            3 => self.render_keys(&mut sv, ctx, w),
            _ => self.render_about(&mut sv, ctx, w),
        }
        let mut scroll = self.set.scroll;
        sv.finish(buf, ctx, &mut scroll, ScrollTarget::Page);
        self.set.scroll = scroll;
        ctx.prefer(Fid::idx("settings.tab", self.set.tab));
    }

    #[allow(clippy::too_many_arguments)]
    fn toggle_row(
        &self,
        sv: &mut ScrollView,
        ctx: &mut Ctx,
        w: u16,
        on: bool,
        label: &str,
        hint_text: &str,
        fid: Fid,
        action: Action,
    ) {
        let r = sv.next(1);
        widgets::checkbox(&mut sv.buf, ctx, r, on, label, fid, action, true);
        let h = widgets::wrap_height(hint_text, w.saturating_sub(4));
        let r = sv.next(h);
        hint(
            &mut sv.buf,
            Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
            hint_text,
        );
        sv.gap(1);
    }

    fn render_general(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("settings.language"));
        let current = i18n::language();
        for (i, (code, name)) in i18n::LANGUAGES.iter().enumerate() {
            let r = sv.next(1);
            widgets::radio(
                &mut sv.buf,
                ctx,
                r,
                current == *code,
                name,
                Fid::idx("settings.lang", i),
                Action::SettingsLanguage(code),
            );
        }
        let r = sv.next(1);
        hint(&mut sv.buf, r, &t("tui.settings.languageHint"));
        sv.gap(1);

        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("tui.settings.theme"));
        for (i, mode) in ThemeMode::ALL.iter().enumerate() {
            let r = sv.next(1);
            widgets::radio(
                &mut sv.buf,
                ctx,
                r,
                self.prefs.theme == *mode,
                &t(mode.label_key()),
                Fid::idx("settings.theme", i),
                Action::SettingsTheme(*mode),
            );
        }
        let note = if theme::no_color() {
            t("tui.settings.noColor")
        } else if !theme::supports_truecolor() {
            t("tui.settings.noTruecolor")
        } else {
            String::new()
        };
        if !note.is_empty() {
            let h = widgets::wrap_height(&note, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &note);
        }
        sv.gap(1);

        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("tui.settings.behaviour"));
        let d = self.set.draft.clone();
        self.toggle_row(
            sv,
            ctx,
            w,
            d.auto_update,
            &t("settings.autoUpdateToggle"),
            &t("tui.settings.autoUpdateHint"),
            Fid::new("settings.autoUpdate"),
            Action::SettingsToggle(SettingToggle::AutoUpdate),
        );
        let r = sv.next(1);
        widgets::button(
            &mut sv.buf,
            ctx,
            r.x + 4,
            r.y,
            r.width,
            &ButtonSpec::new(
                t("tui.settings.checkNow"),
                Fid::new("settings.checkUpdates"),
                Action::CheckUpdates,
                Btn::Secondary,
            ),
        );
        sv.gap(1);
        let accepted = self.settings.telemetry_consent == TelemetryConsent::Accepted;
        self.toggle_row(
            sv,
            ctx,
            w,
            accepted,
            &t("settings.telemetryToggle"),
            &t("settings.telemetryHint"),
            Fid::new("settings.telemetry"),
            Action::SettingsToggle(SettingToggle::Telemetry),
        );
    }

    fn render_downloads(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let browse = ButtonSpec::new(
            t("tui.browse.button"),
            Fid::new("settings.browseDir"),
            Action::Browse(BrowsePurpose::SettingsDownloadDir),
            Btn::Secondary,
        );
        let r = sv.next(3);
        super::source::input_with_button(
            &mut sv.buf,
            ctx,
            r,
            &mut self.set.inputs[SETTING_DOWNLOAD_DIR],
            InputSpec {
                label: &t("select.downloadLocation"),
                placeholder: "",
                fid: Fid::idx("settings.input", SETTING_DOWNLOAD_DIR),
                id: InputId::Setting(SETTING_DOWNLOAD_DIR),
                error: false,
            },
            &browse,
        );
        sv.gap(1);
        let d = self.set.draft.clone();
        self.toggle_row(
            sv,
            ctx,
            w,
            d.use_native_downloader,
            &t("settings.nativeDownloaderToggle"),
            &t("settings.nativeDownloaderHint"),
            Fid::new("settings.native"),
            Action::SettingsToggle(SettingToggle::NativeDownloader),
        );
        self.toggle_row(
            sv,
            ctx,
            w,
            d.cancel_keep_files,
            &t("settings.cancelKeepFilesToggle"),
            &t("settings.cancelKeepFilesHint"),
            Fid::new("settings.keepFiles"),
            Action::SettingsToggle(SettingToggle::CancelKeepFiles),
        );

        let fields: [(usize, &str, &str, &str); 5] = [
            (
                SETTING_CHUNKS,
                "settings.chunkConcurrency",
                "settings.chunkConcurrencyHint",
                "8",
            ),
            (
                SETTING_RETRIES,
                "settings.maxRetries",
                "settings.maxRetriesHint",
                "3",
            ),
            (
                SETTING_DD_ARGS,
                "settings.ddExtraArgs",
                "settings.ddExtraArgsHint",
                "-max-downloads 8 -verify-all",
            ),
            (
                SETTING_SPEED,
                "settings.speedLimit",
                "settings.speedLimitHint",
                "@settings.speedLimitExample",
            ),
            (
                SETTING_PROXY,
                "settings.proxy",
                "settings.proxyHint",
                "@settings.proxyPlaceholder",
            ),
        ];
        for (idx, label, hint_key, ph) in fields {
            let placeholder = match ph.strip_prefix('@') {
                Some(k) => t(k),
                None => ph.to_string(),
            };
            let r = sv.next(3);
            widgets::input(
                &mut sv.buf,
                ctx,
                r,
                &mut self.set.inputs[idx],
                InputSpec {
                    label: &t(label),
                    placeholder: &placeholder,
                    fid: Fid::idx("settings.input", idx),
                    id: InputId::Setting(idx),
                    error: false,
                },
            );
            let ht = t(hint_key);
            let h = widgets::wrap_height(&ht, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &ht);
            sv.gap(1);
        }
    }

    fn render_keys(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let fields: [(usize, &str, &str, &str); 3] = [
            (
                SETTING_MH,
                "select.manifestHubLabel",
                "select.manifestHubPlaceholder",
                "select.manifestHubHint",
            ),
            (
                SETTING_HUBCAP,
                "settings.hubcapApiKey",
                "settings.hubcapApiKeyPlaceholder",
                "settings.hubcapApiKeyHint",
            ),
            (
                SETTING_RYUU,
                "settings.ryuuApiKey",
                "settings.ryuuApiKeyPlaceholder",
                "settings.ryuuApiKeyHint",
            ),
        ];
        for (idx, label, ph, hint_key) in fields {
            let mask_label = if self.set.inputs[idx].masked {
                t("tui.show")
            } else {
                t("tui.hide")
            };
            let mask = ButtonSpec::new(
                mask_label,
                Fid::idx("settings.mask", idx),
                Action::ToggleMask(InputId::Setting(idx)),
                Btn::Secondary,
            );
            let r = sv.next(3);
            super::source::input_with_button(
                &mut sv.buf,
                ctx,
                r,
                &mut self.set.inputs[idx],
                InputSpec {
                    label: &t(label),
                    placeholder: &t(ph),
                    fid: Fid::idx("settings.input", idx),
                    id: InputId::Setting(idx),
                    error: false,
                },
                &mask,
            );
            let ht = t(hint_key);
            let h = widgets::wrap_height(&ht, w);
            let r = sv.next(h);
            hint(&mut sv.buf, r, &ht);
            sv.gap(1);
        }
        let note = t("tui.settings.mhStoredLocally");
        let h = widgets::wrap_height(&note, w);
        let r = sv.next(h);
        hint(&mut sv.buf, r, &note);
    }

    fn render_about(&mut self, sv: &mut ScrollView, ctx: &mut Ctx, w: u16) {
        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("settings.debugInfo"));
        for (k, v) in build_info() {
            let r = sv.next(1);
            widgets::line(
                &mut sv.buf,
                r,
                Line::from(vec![
                    Span::styled(format!("{:<14}", k), theme::dim()),
                    Span::styled(v, theme::text()),
                ]),
            );
        }
        let r = sv.next(1);
        widgets::button(
            &mut sv.buf,
            ctx,
            r.x,
            r.y,
            r.width,
            &ButtonSpec::new(
                t("settings.copyDebugInfo"),
                Fid::new("settings.copyInfo"),
                Action::CopyBuildInfo,
                Btn::Secondary,
            ),
        );
        let ht = t("settings.debugInfoHint");
        let h = widgets::wrap_height(&ht, w);
        let r = sv.next(h);
        hint(&mut sv.buf, r, &ht);
        sv.gap(1);
        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &t("tui.settings.paths"));
        let paths = [
            (
                t("tui.settings.dataDir"),
                self.data_dir.to_string_lossy().to_string(),
            ),
            (
                t("tui.settings.debugLog"),
                smd_core::services::debug_log::log_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
            (
                t("tui.settings.stderrLog"),
                crate::term::stderr_log_path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        ];
        for (k, v) in paths {
            let r = sv.next(1);
            widgets::text(&mut sv.buf, r, &k, theme::dim());
            let r = sv.next(1);
            widgets::text(
                &mut sv.buf,
                Rect::new(r.x + 2, r.y, r.width.saturating_sub(2), 1),
                &v,
                theme::text(),
            );
        }
        let ht = tf(
            "tui.settings.dataDirHint",
            &[("env", &smd_core::paths::DATA_DIR_ENV)],
        );
        let h = widgets::wrap_height(&ht, w);
        let r = sv.next(h);
        hint(&mut sv.buf, r, &ht);
        sv.gap(1);
        let r = sv.next(1);
        widgets::text(
            &mut sv.buf,
            r,
            "https://github.com/MCbabel/Steam-Manifest-Downloader",
            theme::accent(),
        );
    }

    fn render_sources_tab(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let ht = t("settings.depotSourcesHint");
        let h = widgets::wrap_height(&ht, rest.width).min(4);
        hint(buf, take_top(&mut rest, h), &ht);
        let _ = take_top(&mut rest, 1);
        let add = ButtonSpec::new(
            t("settings.add"),
            Fid::new("settings.addSource"),
            Action::SettingsAddSource,
            Btn::Primary,
        );
        super::source::input_with_button(
            buf,
            ctx,
            take_top(&mut rest, 3),
            &mut self.set.source_add,
            InputSpec {
                label: &t("tui.settings.newSource"),
                placeholder: &t("settings.depotSourcePlaceholder"),
                fid: Fid::new("settings.sourceInput"),
                id: InputId::SettingsSourceAdd,
                error: self.set.source_error.is_some(),
            },
            &add,
        );
        if let Some(e) = &self.set.source_error {
            widgets::status(buf, take_top(&mut rest, 1), Tone::Error, e, 0);
        }
        let _ = take_top(&mut rest, 1);
        let sources = self.settings.depot_sources.clone();
        let list_fid = Fid::new("settings.sources");
        let focused = ctx.is_focused(list_fid);
        let card = widgets::card(
            buf,
            rest,
            Some(&tf(
                "tui.settings.sourceCount",
                &[("count", &sources.len())],
            )),
            focused,
        );
        if sources.is_empty() {
            widgets::text(
                buf,
                Rect::new(card.x + 1, card.y, card.width.saturating_sub(2), 1),
                &t("search.noSourceTitle"),
                theme::muted(),
            );
            return;
        }
        if self.set.source_selected >= sources.len() {
            self.set.source_selected = sources.len() - 1;
        }
        let range = widgets::list_window(
            self.set.source_selected,
            &mut self.set.source_offset,
            sources.len(),
            card.height as usize,
        );
        let remove_label = format!("✕ {}", t("settings.removeSource"));
        let rw = widgets::button_width(&remove_label);
        for (row, i) in range.enumerate() {
            let r = Rect::new(card.x, card.y + row as u16, card.width.saturating_sub(1), 1);
            let cursor = i == self.set.source_selected;
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
            }
            let default = self.settings.pristine_default_sources.contains(&sources[i]);
            let mut spans = vec![Span::styled(
                if cursor { " ▸ " } else { "   " },
                theme::accent(),
            )];
            spans.push(Span::styled(
                widgets::truncate(&sources[i], r.width.saturating_sub(rw + 14)),
                theme::text(),
            ));
            if default {
                spans.push(Span::raw(" "));
                spans.push(widgets::badge(
                    &t("tui.settings.defaultSource"),
                    theme::get().purple,
                ));
            }
            widgets::line(buf, r, Line::from(spans));
            ctx.row(
                r,
                list_fid,
                Action::ListSelect(ListId::SettingsSources, i),
                Action::ListSelect(ListId::SettingsSources, i),
            );
            widgets::button(
                buf,
                ctx,
                r.right().saturating_sub(rw),
                r.y,
                rw,
                &ButtonSpec::new(
                    remove_label.clone(),
                    Fid::idx("settings.removeSource", i),
                    Action::SettingsRemoveSource(i),
                    Btn::Danger,
                ),
            );
        }
        widgets::scrollbar(
            buf,
            card,
            sources.len(),
            card.height as usize,
            self.set.source_offset,
        );
        ctx.focusable(list_fid, rest, Kind::List(ListId::SettingsSources), None);
        ctx.scroll_region(rest, ScrollTarget::List(ListId::SettingsSources));
    }
}
