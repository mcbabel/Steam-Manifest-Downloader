use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};

use super::action::{Action, BrowsePurpose, InputId, ScrollTarget, Step};
use super::{apply, hint, App};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{take_bottom, Ctx, Fid, ScrollView};
use smd_core::services::steam_library::STEAM_RUNNING_ERROR;

pub fn derive_start_dir(exe: &str) -> String {
    let idx = exe.rfind(['/', '\\']);
    match idx {
        Some(i) if i > 0 => {
            let sep = if exe.contains('\\') { '\\' } else { '/' };
            format!("{}{}", &exe[..i], sep)
        }
        _ => String::new(),
    }
}

impl App {
    pub(super) fn go_to_shortcut_step(&mut self) {
        self.wiz.step = Step::Shortcut;
        self.wiz.shortcuts_done = false;
        self.wiz.shortcut_status = None;
        self.wiz.post_scroll = 0;
        self.focus = None;
        self.detect_executables();
    }

    pub(super) fn go_to_steam_step(&mut self) {
        self.wiz.step = Step::SteamLibrary;
        self.wiz.steam_done = false;
        self.wiz.steam_running = false;
        self.wiz.steam_status = None;
        self.wiz
            .steam_name
            .set(self.wiz.game_name.clone().unwrap_or_default());
        self.wiz.steam_launch.clear();
        self.wiz.post_scroll = 0;
        self.focus = None;
        self.detect_executables();
    }

    fn detect_executables(&mut self) {
        let Some(dir) = self.wiz.result_dir.clone() else {
            return;
        };
        self.wiz.exes.clear();
        self.wiz.exe_selected = 0;
        self.wiz.exes_loading = true;
        self.spawn(async move {
            let r = smd_core::ops::shortcuts::detect_executables(&dir).await;
            apply(move |app| {
                app.wiz.exes_loading = false;
                let exes = r.unwrap_or_default();
                if let Some(rec) = exes.iter().find(|e| e.recommended).or(exes.first()) {
                    let path = rec.path.clone();
                    if app.wiz.shortcut_exe.is_empty() {
                        app.wiz.shortcut_exe.set(path.clone());
                    }
                    if app.wiz.steam_exe.is_empty() {
                        app.wiz.steam_exe.set(path);
                    }
                }
                app.wiz.exes = exes;
            })
        });
    }

    fn continue_after_post(&mut self) {
        if self.emulator_available {
            self.go_to_emulator_step();
        } else {
            self.reset_wizard();
        }
    }

    pub(super) fn dispatch_post(&mut self, a: &Action) -> bool {
        match a {
            Action::PickExe(i) => {
                if let Some(e) = self.wiz.exes.get(*i) {
                    let p = e.path.clone();
                    self.wiz.exe_selected = *i;
                    match self.wiz.step {
                        Step::SteamLibrary => self.wiz.steam_exe.set(p),
                        _ => self.wiz.shortcut_exe.set(p),
                    }
                }
            }
            Action::ToggleShortcutDesktop => self.wiz.shortcut_desktop = !self.wiz.shortcut_desktop,
            Action::ToggleShortcutStartMenu => {
                self.wiz.shortcut_start_menu = !self.wiz.shortcut_start_menu
            }
            Action::ToggleShortcutSteam => {
                if self.steam_install.is_some() {
                    self.wiz.shortcut_steam = !self.wiz.shortcut_steam;
                }
            }
            Action::CreateShortcuts => {
                if self.wiz.shortcuts_done {
                    self.continue_after_post();
                } else {
                    self.create_shortcuts();
                }
            }
            Action::ShortcutNext => self.continue_after_post(),
            Action::SteamAdd => {
                if self.wiz.steam_done {
                    self.continue_after_post();
                } else {
                    self.steam_add(false);
                }
            }
            Action::SteamCloseAndAdd => self.steam_add(true),
            Action::SteamNext => self.continue_after_post(),
            _ => return false,
        }
        true
    }

    fn create_shortcuts(&mut self) {
        let exe = self.wiz.shortcut_exe.trimmed();
        if exe.is_empty() {
            return;
        }
        let (desktop, start) = (self.wiz.shortcut_desktop, self.wiz.shortcut_start_menu);
        if !desktop && !start {
            self.wiz.shortcut_status = Some((Tone::Error, t("tui.shortcut.pickLocation")));
            return;
        }
        let name = self.wiz.game_name.clone().unwrap_or_else(|| "Game".into());
        let also_steam = self.wiz.shortcut_steam && self.steam_install.is_some();
        let app_id = self.wiz.main_app_id().unwrap_or_default();
        let steam_name = self
            .wiz
            .game_name
            .clone()
            .unwrap_or_else(|| format!("App {}", app_id));
        self.wiz.shortcut_busy = true;
        let core = self.core.clone();
        self.spawn(async move {
            let r = smd_core::ops::shortcuts::create_shortcuts(exe.clone(), name, None, desktop, start).await;
            let steam = if also_steam && !app_id.is_empty() {
                Some(
                    smd_core::services::steam_library::add_to_steam_library(
                        &core.http_client,
                        &app_id,
                        &steam_name,
                        &exe,
                        &derive_start_dir(&exe),
                        "",
                        false,
                    )
                    .await,
                )
            } else {
                None
            };
            apply(move |app| {
                app.wiz.shortcut_busy = false;
                let mut msgs = Vec::new();
                let mut ok = false;
                match r {
                    Ok(res) => {
                        if res.desktop {
                            msgs.push(t("tui.shortcut.desktopDone"));
                        }
                        if res.start_menu {
                            msgs.push(t("tui.shortcut.startMenuDone"));
                        }
                        if !res.errors.is_empty() {
                            msgs.push(tf("shortcut.errors", &[("list", &res.errors.iter().map(|e| crate::i18n::localize_error(e)).collect::<Vec<_>>().join(", "))]));
                        }
                        ok = (!desktop || res.desktop) && (!start || res.start_menu);
                        if ok {
                            app.emit("shortcut_created", Some(serde_json::json!({ "desktop": res.desktop, "start_menu": res.start_menu })));
                        }
                    }
                    Err(e) => msgs.push(e),
                }
                match steam {
                    Some(Ok(added)) => msgs.push(format!(
                        "{} ({})",
                        tf(steam_success_key(&added), &[("name", &steam_name)]),
                        tf("steamLibrary.gridArtCount", &[("count", &added.grid_files.len())])
                    )),
                    Some(Err(e)) if e == STEAM_RUNNING_ERROR => msgs.push(t("steamLibrary.steamRunning")),
                    Some(Err(e)) => msgs.push(tf("steamLibrary.error", &[("message", &e)])),
                    None => {}
                }
                app.wiz.shortcut_status = Some((if ok { Tone::Success } else { Tone::Error }, msgs.join("\n")));
                app.wiz.shortcuts_done = ok;
            })
        });
    }

    fn steam_add(&mut self, close_steam: bool) {
        let exe = self.wiz.steam_exe.trimmed();
        if exe.is_empty() {
            self.wiz.steam_status = Some((
                Tone::Error,
                tf(
                    "steamLibrary.error",
                    &[("message", &t("tui.errors.noExe"))],
                ),
            ));
            return;
        }
        let Some(app_id) = self.wiz.main_app_id() else {
            self.wiz.steam_status = Some((
                Tone::Error,
                tf("steamLibrary.error", &[("message", &t("tui.errors.noAppId"))]),
            ));
            return;
        };
        let name = Some(self.wiz.steam_name.trimmed())
            .filter(|s| !s.is_empty())
            .or_else(|| self.wiz.game_name.clone())
            .unwrap_or_else(|| format!("App {}", app_id));
        let launch = self.wiz.steam_launch.trimmed();
        self.wiz.steam_busy = true;
        self.wiz.steam_running = false;
        self.wiz.steam_status = Some((Tone::Busy, t("steamLibrary.adding")));
        let core = self.core.clone();
        self.spawn(async move {
            let r = smd_core::services::steam_library::add_to_steam_library(
                &core.http_client,
                &app_id,
                &name,
                &exe,
                &derive_start_dir(&exe),
                &launch,
                close_steam,
            )
            .await;
            apply(move |app| {
                app.wiz.steam_busy = false;
                match r {
                    Ok(added) => {
                        let mut msg = format!(
                            "{}\n{}",
                            tf(steam_success_key(&added), &[("name", &name)]),
                            tf(
                                "steamLibrary.gridArtCount",
                                &[("count", &added.grid_files.len())]
                            )
                        );
                        if exe.to_lowercase().ends_with(".exe") {
                            msg.push('\n');
                            msg.push_str(&t("steamLibrary.protonNote"));
                        }
                        app.wiz.steam_status = Some((Tone::Success, msg));
                        app.wiz.steam_done = true;
                    }
                    Err(e) if e == STEAM_RUNNING_ERROR => {
                        app.wiz.steam_running = true;
                        app.wiz.steam_status =
                            Some((Tone::Warning, t("steamLibrary.steamRunning")));
                    }
                    Err(e) => {
                        app.wiz.steam_status =
                            Some((Tone::Error, tf("steamLibrary.error", &[("message", &e)])))
                    }
                }
            })
        });
    }

    fn render_exe_list(&self, sv: &mut ScrollView, ctx: &mut Ctx) {
        if self.wiz.exes_loading {
            let r = sv.next(1);
            widgets::status(
                &mut sv.buf,
                r,
                Tone::Busy,
                &t("shortcut.exePlaceholder"),
                self.tick,
            );
            return;
        }
        if self.wiz.exes.len() < 2 {
            return;
        }
        sv.gap(1);
        let r = sv.next(1);
        widgets::text(&mut sv.buf, r, &t("shortcut.otherDetected"), theme::dim());
        let current = match self.wiz.step {
            Step::SteamLibrary => self.wiz.steam_exe.trimmed(),
            _ => self.wiz.shortcut_exe.trimmed(),
        };
        for (i, exe) in self.wiz.exes.iter().take(12).enumerate() {
            let r = sv.next(1);
            let chosen = exe.path == current;
            if ctx.is_hovered(r) || chosen {
                widgets::fill(
                    &mut sv.buf,
                    r,
                    if chosen {
                        theme::selection()
                    } else {
                        theme::surface()
                    },
                );
            }
            let mut spans = vec![
                Span::styled(
                    if chosen { " ● " } else { " ○ " },
                    if chosen {
                        theme::accent_bold()
                    } else {
                        theme::dim()
                    },
                ),
                Span::styled(exe.name.clone(), theme::text()),
            ];
            if exe.recommended {
                spans.push(Span::raw(" "));
                spans.push(widgets::badge(
                    &t("tui.shortcut.recommended"),
                    theme::get().success,
                ));
            }
            spans.push(Span::styled(
                format!("  {}", widgets::fmt_bytes(exe.size)),
                theme::muted(),
            ));
            widgets::line(&mut sv.buf, r, Line::from(spans));
            ctx.focusable(
                Fid::idx("post.exe", i),
                r,
                crate::ui::Kind::Button,
                Some(Action::PickExe(i)),
            );
        }
    }

    pub(super) fn render_shortcut(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        let mut sv = ScrollView::begin(ctx, rest, 80, self.wiz.post_scroll);
        let w = sv.width();
        let r_ = sv.next(1);
        widgets::heading(&mut sv.buf, r_, &t("shortcut.title"));
        let d = t("shortcut.description");
        let h = widgets::wrap_height(&d, w);
        let r_ = sv.next(h);
        widgets::paragraph(&mut sv.buf, r_, &d, theme::dim());
        sv.gap(1);
        let browse = ButtonSpec::new(
            t("shortcut.browse"),
            Fid::new("shortcut.browse"),
            Action::Browse(BrowsePurpose::ShortcutExe),
            Btn::Secondary,
        );
        let row = sv.next(3);
        super::source::input_with_button(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.shortcut_exe,
            InputSpec {
                label: &t("shortcut.exeLabel"),
                placeholder: &t("shortcut.exePlaceholder"),
                fid: Fid::new("shortcut.exe"),
                id: InputId::ShortcutExe,
                error: false,
            },
            &browse,
        );
        let h = widgets::wrap_height(&t("shortcut.exeHint"), w);
        let r_ = sv.next(h);
        hint(&mut sv.buf, r_, &t("shortcut.exeHint"));
        self.render_exe_list(&mut sv, ctx);
        sv.gap(1);
        let r_ = sv.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r_,
            self.wiz.shortcut_desktop,
            &t("shortcut.desktopShortcut"),
            Fid::new("shortcut.desktop"),
            Action::ToggleShortcutDesktop,
            true,
        );
        let r_ = sv.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r_,
            self.wiz.shortcut_start_menu,
            &t("shortcut.startMenuShortcut"),
            Fid::new("shortcut.startMenu"),
            Action::ToggleShortcutStartMenu,
            true,
        );
        let steam_ok = self.steam_install.is_some();
        let r_ = sv.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r_,
            self.wiz.shortcut_steam && steam_ok,
            &t("steamLibrary.windowsToggle"),
            Fid::new("shortcut.steam"),
            Action::ToggleShortcutSteam,
            steam_ok,
        );
        let note = if steam_ok {
            t("steamLibrary.windowsToggleHint")
        } else {
            t("steamLibrary.notDetected")
        };
        let h = widgets::wrap_height(&note, w.saturating_sub(4));
        let r = sv.next(h);
        hint(
            &mut sv.buf,
            Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
            &note,
        );
        if let Some((tone, msg)) = &self.wiz.shortcut_status {
            sv.gap(1);
            let h = widgets::status_height(msg, w);
            let r_ = sv.next(h);
            widgets::status(&mut sv.buf, r_, *tone, msg, self.tick);
        }
        let mut scroll = self.wiz.post_scroll;
        sv.finish(buf, ctx, &mut scroll, ScrollTarget::Page);
        self.wiz.post_scroll = scroll;

        let main_label = if self.wiz.shortcuts_done {
            t("common.next")
        } else {
            t("shortcut.createShortcuts")
        };
        let specs = [
            ButtonSpec::new(
                t("shortcut.skip"),
                Fid::new("shortcut.skip"),
                Action::ShortcutNext,
                Btn::Secondary,
            ),
            ButtonSpec::new(
                main_label,
                Fid::new("shortcut.create"),
                Action::CreateShortcuts,
                Btn::Primary,
            )
            .enabled(!self.wiz.shortcut_busy && !self.wiz.shortcut_exe.is_empty()),
        ];
        widgets::buttons(buf, ctx, footer, &specs, true);
        ctx.prefer(Fid::new("shortcut.create"));
    }

    pub(super) fn render_steam_library(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let mut rest = area;
        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        let mut sv = ScrollView::begin(ctx, rest, 90, self.wiz.post_scroll);
        let w = sv.width();
        let r_ = sv.next(1);
        widgets::heading(&mut sv.buf, r_, &t("steamLibrary.title"));
        let d = t("steamLibrary.description");
        let h = widgets::wrap_height(&d, w);
        let r_ = sv.next(h);
        widgets::paragraph(&mut sv.buf, r_, &d, theme::dim());
        sv.gap(1);
        match &self.steam_install {
            Some(inst) => {
                let name = inst
                    .persona_name
                    .clone()
                    .unwrap_or_else(|| inst.user_id3.clone());
                let r_ = sv.next(1);
                widgets::status(
                    &mut sv.buf,
                    r_,
                    Tone::Success,
                    &tf("steamLibrary.detected", &[("name", &name)]),
                    0,
                );
            }
            None => {
                let r_ = sv.next(1);
                widgets::status(
                    &mut sv.buf,
                    r_,
                    Tone::Warning,
                    &t("steamLibrary.notDetected"),
                    0,
                );
            }
        }
        sv.gap(1);
        let row = sv.next(3);
        widgets::input(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.steam_name,
            InputSpec {
                label: &t("steamLibrary.gameNameLabel"),
                placeholder: &t("steamLibrary.gameNamePlaceholder"),
                fid: Fid::new("steam.name"),
                id: InputId::SteamName,
                error: false,
            },
        );
        let browse = ButtonSpec::new(
            t("shortcut.browse"),
            Fid::new("steam.browse"),
            Action::Browse(BrowsePurpose::SteamExe),
            Btn::Secondary,
        );
        let row = sv.next(3);
        super::source::input_with_button(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.steam_exe,
            InputSpec {
                label: &t("steamLibrary.exeLabel"),
                placeholder: &t("shortcut.exePlaceholder"),
                fid: Fid::new("steam.exe"),
                id: InputId::SteamExe,
                error: false,
            },
            &browse,
        );
        let h = widgets::wrap_height(&t("steamLibrary.exeHint"), w);
        let r_ = sv.next(h);
        hint(&mut sv.buf, r_, &t("steamLibrary.exeHint"));
        self.render_exe_list(&mut sv, ctx);
        sv.gap(1);
        let row = sv.next(3);
        widgets::input(
            &mut sv.buf,
            ctx,
            row,
            &mut self.wiz.steam_launch,
            InputSpec {
                label: &t("steamLibrary.launchOptionsLabel"),
                placeholder: &t("steamLibrary.launchOptionsPlaceholder"),
                fid: Fid::new("steam.launch"),
                id: InputId::SteamLaunch,
                error: false,
            },
        );
        let h = widgets::wrap_height(&t("steamLibrary.launchOptionsHint"), w);
        let r_ = sv.next(h);
        hint(&mut sv.buf, r_, &t("steamLibrary.launchOptionsHint"));
        if let Some((tone, msg)) = &self.wiz.steam_status {
            sv.gap(1);
            let h = widgets::status_height(msg, w);
            let r_ = sv.next(h);
            widgets::status(&mut sv.buf, r_, *tone, msg, self.tick);
        }
        let mut scroll = self.wiz.post_scroll;
        sv.finish(buf, ctx, &mut scroll, ScrollTarget::Page);
        self.wiz.post_scroll = scroll;

        let main = if self.wiz.steam_done {
            t("steamLibrary.next")
        } else {
            t("steamLibrary.add")
        };
        let can_add = !self.wiz.steam_busy && self.steam_install.is_some();
        let mut specs = vec![ButtonSpec::new(
            t("steamLibrary.skip"),
            Fid::new("steam.skip"),
            Action::SteamNext,
            Btn::Secondary,
        )];
        if self.wiz.steam_running && !self.wiz.steam_done {
            specs.push(
                ButtonSpec::new(
                    t("steamLibrary.closeAndAdd"),
                    Fid::new("steam.closeAndAdd"),
                    Action::SteamCloseAndAdd,
                    Btn::Secondary,
                )
                .enabled(can_add),
            );
        }
        specs.push(
            ButtonSpec::new(main, Fid::new("steam.add"), Action::SteamAdd, Btn::Primary)
                .enabled(can_add),
        );
        widgets::buttons(buf, ctx, footer, &specs, true);
        ctx.prefer(Fid::new(if self.wiz.steam_running {
            "steam.closeAndAdd"
        } else {
            "steam.add"
        }));
    }
}

fn steam_success_key(added: &smd_core::services::steam_library::ShortcutAdded) -> &'static str {
    if added.steam_restarted {
        "steamLibrary.successRestarted"
    } else {
        "steamLibrary.success"
    }
}

#[cfg(test)]
mod tests {
    use super::derive_start_dir;

    #[test]
    fn start_dir_keeps_separator_style() {
        assert_eq!(derive_start_dir("/games/x/bin/game"), "/games/x/bin/");
        assert_eq!(derive_start_dir("C:\\Games\\x\\game.exe"), "C:\\Games\\x\\");
        assert_eq!(derive_start_dir("game.exe"), "");
    }
}
