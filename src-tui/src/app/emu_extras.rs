use std::path::PathBuf;

use smd_core::ops::emulator::DOWNLOAD_CONFIRM_PREFIX;
use smd_core::services::emu_extras as extras;

use super::action::Action;
use super::state::EmuState;
use super::{apply, App, Modal};
use crate::i18n::{localize_error, t, tf};
use crate::ui::widgets::Tone;

pub const EXTRA_SAVES: usize = 8;
pub const EXTRA_PROFILE: usize = 9;
pub const EXTRA_NETWORK: usize = 10;
pub const EXTRA_MODS: usize = 11;

fn expand(path: &str) -> PathBuf {
    crate::ui::file_browser::expand_tilde(path.trim())
}

impl EmuState {
    pub fn extra_target(&self) -> Option<String> {
        if self.edit_mode {
            self.edit_targets.first().map(|t| t.path.clone())
        } else {
            self.selected_targets().first().map(|t| t.path.clone())
        }
    }

    pub fn extra_targets(&self) -> Vec<String> {
        if self.edit_mode {
            self.edit_targets.iter().map(|t| t.path.clone()).collect()
        } else {
            self.selected_targets().into_iter().map(|t| t.path).collect()
        }
    }

    pub fn game_patched(&self) -> bool {
        self.edit_mode || self.apply_complete || self.scan.iter().any(|f| f.is_patched)
    }
}

impl App {
    fn extra_status(&mut self, tone: Tone, msg: String) {
        if let Some(e) = self.emu.as_mut() {
            e.extras.status = Some((tone, msg));
        }
    }

    pub(super) fn load_extra_section(&mut self, section: usize) {
        let Some(e) = self.emu.as_mut() else { return };
        e.extras.status = None;
        let Some(target) = e.extra_target() else { return };
        let app_id = e.app_id.clone();
        let lang = e.gather().and_then(|g| g.language).unwrap_or_else(|| "english".into());
        self.spawn(async move {
            let t = PathBuf::from(&target);
            let data = tokio::task::spawn_blocking(move || match section {
                EXTRA_SAVES => (
                    Some(extras::save_folders(&t, &app_id)),
                    Some(extras::achievements(&t, &app_id, &lang)),
                    None,
                    None,
                    None,
                ),
                EXTRA_PROFILE => (None, None, Some(extras::overlay_assets(&t)), None, None),
                EXTRA_NETWORK => (None, None, None, Some(extras::broadcasts(&t)), None),
                _ => (None, None, None, None, Some(extras::mods(&t))),
            })
            .await
            .unwrap_or_default();
            apply(move |app| {
                let Some(e) = app.emu.as_mut() else { return };
                let x = &mut e.extras;
                if let Some(saves) = data.0 {
                    let keep = x.saves.get(x.save_idx).map(|f| f.path.clone());
                    x.save_idx = keep
                        .and_then(|p| saves.iter().position(|f| f.path == p))
                        .or_else(|| saves.iter().position(|f| f.exists))
                        .unwrap_or(0);
                    x.saves = saves;
                }
                if let Some(ach) = data.1 {
                    x.ach = Some(ach);
                }
                if let Some(assets) = data.2 {
                    x.assets = assets;
                }
                if let Some(list) = data.3 {
                    x.broadcasts.set(list.join(", "));
                }
                if let Some(mods) = data.4 {
                    x.mods = mods;
                }
            })
        });
    }

    pub(super) fn dispatch_emu_extras(&mut self, a: &Action) -> bool {
        match a {
            Action::EmuSaveSelect(i) => {
                if let Some(e) = self.emu.as_mut() {
                    e.extras.save_idx = *i;
                }
            }
            Action::EmuSavesOpen => {
                let folder = self.emu.as_ref().and_then(|e| e.extras.saves.get(e.extras.save_idx)).cloned();
                if let Some(f) = folder.filter(|f| f.exists) {
                    if let Err(err) = smd_core::ops::system::open_folder(&f.path) {
                        self.toast(Tone::Error, err);
                    }
                }
            }
            Action::EmuSavesBackup | Action::EmuSavesRestore => self.saves_backup_restore(matches!(a, Action::EmuSavesBackup)),
            Action::EmuAchResetAll | Action::EmuStatsReset => self.ach_reset(matches!(a, Action::EmuAchResetAll)),
            Action::EmuAssetSet(kind) => self.asset_change(kind, true),
            Action::EmuAssetClear(kind) => self.asset_change(kind, false),
            Action::EmuBroadcastsSave => self.broadcasts_save(),
            Action::EmuLobbyConnect(allow) => self.lobby_connect(*allow),
            Action::EmuModAdd => self.mod_add(),
            Action::EmuModRemove(i) => self.mod_remove(*i),
            _ => return false,
        }
        true
    }

    fn saves_backup_restore(&mut self, backup: bool) {
        let Some(e) = self.emu.as_ref() else { return };
        let Some(folder) = e.extras.saves.get(e.extras.save_idx).cloned() else { return };
        let zip = e.extras.zip_path.trimmed();
        if zip.is_empty() {
            self.extra_status(Tone::Error, t("tui.emu.extras.zipNeeded"));
            return;
        }
        let zip = expand(&zip);
        self.extra_status(Tone::Busy, t("emulator.saves.working"));
        self.spawn(async move {
            let path = PathBuf::from(&folder.path);
            let r = tokio::task::spawn_blocking(move || {
                if backup {
                    extras::backup_saves(&path, &zip)
                } else {
                    extras::restore_saves(&path, &zip)
                }
            })
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
            apply(move |app| {
                let action = if backup { "backup" } else { "restore" };
                app.emit("emu_saves", Some(serde_json::json!({ "action": action, "ok": r.is_ok() })));
                match r {
                    Ok(count) => {
                        let key = if backup { "emulator.saves.backedUp" } else { "emulator.saves.restored" };
                        app.extra_status(Tone::Success, tf(key, &[("count", &count)]));
                    }
                    Err(err) => app.extra_status(Tone::Error, localize_error(&err)),
                }
                app.load_extra_section(EXTRA_SAVES);
            })
        });
    }

    fn ach_reset(&mut self, achievements: bool) {
        let Some(e) = self.emu.as_mut() else { return };
        let Some(target) = e.extra_target() else { return };
        let key = if achievements { 1 } else { 2 };
        let armed = e.extras.armed.is_some_and(|(k, at)| k == key && at.elapsed().as_secs() < 4);
        if !armed {
            e.extras.armed = Some((key, std::time::Instant::now()));
            e.extras.status = Some((Tone::Warning, t("emulator.achievements.confirm")));
            return;
        }
        e.extras.armed = None;
        let app_id = e.app_id.clone();
        let t_path = PathBuf::from(target);
        let r = if achievements {
            extras::reset_achievements(&t_path, &app_id, None)
        } else {
            extras::reset_stats(&t_path, &app_id)
        };
        let action = if achievements { "lock_all" } else { "clear_stats" };
        match r {
            Ok(n) => {
                self.emit("emu_achievements", Some(serde_json::json!({ "action": action })));
                let key = if achievements { "emulator.achievements.lockedCount" } else { "emulator.achievements.statsCleared" };
                self.extra_status(Tone::Success, tf(key, &[("count", &n)]));
            }
            Err(err) => self.extra_status(Tone::Error, localize_error(&err)),
        }
        self.load_extra_section(EXTRA_SAVES);
    }

    fn asset_change(&mut self, kind: &'static str, set: bool) {
        let Some(e) = self.emu.as_mut() else { return };
        let targets = e.extra_targets();
        if targets.is_empty() {
            return;
        }
        let src = expand(&e.extras.asset_path.trimmed());
        if set && e.extras.asset_path.trimmed().is_empty() {
            e.extras.status = Some((Tone::Error, t("tui.emu.extras.fileNeeded")));
            return;
        }
        let r = match (kind, set) {
            ("avatar", true) => extras::set_avatar(&targets, &src),
            ("avatar", false) => extras::remove_avatar(&targets),
            ("font", true) => extras::set_font(&targets, &src),
            ("font", false) => extras::remove_font(&targets),
            (k, true) => extras::set_sound(&targets, k, &src),
            (k, false) => extras::remove_sound(&targets, k),
        };
        if kind == "avatar" {
            if let Some(e) = self.emu.as_mut() {
                if r.is_ok() {
                    if set {
                        e.bools.insert("enable_account_avatar", true);
                    } else {
                        e.bools.remove("enable_account_avatar");
                    }
                }
            }
        }
        let action = if set { "set" } else { "remove" };
        let mut props = serde_json::json!({ "asset": kind, "action": action, "ok": r.is_ok() });
        if kind == "font" && set {
            props["source"] = serde_json::json!("file");
        }
        self.emit("emu_profile", Some(props));
        match r {
            Ok(()) => self.extra_status(Tone::Success, if set { t("emulator.profile.saved") } else { String::new() }),
            Err(err) => self.extra_status(Tone::Error, localize_error(&err)),
        }
        if let Some(e) = self.emu.as_mut() {
            if e.extras.status.as_ref().is_some_and(|(_, m)| m.is_empty()) {
                e.extras.status = None;
            }
        }
        self.load_extra_section(EXTRA_PROFILE);
    }

    fn broadcasts_save(&mut self) {
        let Some(e) = self.emu.as_ref() else { return };
        let targets = e.extra_targets();
        let list = vec![e.extras.broadcasts.trimmed()];
        match extras::set_broadcasts(&targets, &list) {
            Ok(bad) => {
                let total = list[0].split([',', ';', ' ', '\n']).filter(|s| !s.trim().is_empty()).count();
                let count = total.saturating_sub(bad.len());
                self.emit("emu_network", Some(serde_json::json!({ "action": "broadcasts", "count": super::emulator::count_bucket(count) })));
                if bad.is_empty() {
                    self.extra_status(Tone::Success, t("emulator.network.saved"));
                } else {
                    self.extra_status(Tone::Error, tf("emulator.network.rejected", &[("list", &bad.join(", "))]));
                }
            }
            Err(err) => self.extra_status(Tone::Error, localize_error(&err)),
        }
        self.load_extra_section(EXTRA_NETWORK);
    }

    fn lobby_connect(&mut self, allow: bool) {
        self.close_modal();
        let Some(e) = self.emu.as_ref() else { return };
        let Some(target) = e.extra_target() else { return };
        self.extra_status(Tone::Busy, t("emulator.network.lobbyPreparing"));
        let core = self.core.clone();
        let sink = self.sink.clone();
        let data = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::lobby_connect(&sink, &core, &data, target, allow).await;
            let launched = match &r {
                Ok(path) => Some(smd_core::ops::emulator::blocking({
                    let p = PathBuf::from(path);
                    move || extras::launch_console(&p)
                })
                .await
                .unwrap_or_else(|e| Err(e))),
                Err(_) => None,
            };
            apply(move |app| match (r, launched) {
                (Err(err), _) if err.starts_with(DOWNLOAD_CONFIRM_PREFIX) => {
                    if let Some(e) = app.emu.as_mut() {
                        e.extras.status = None;
                    }
                    app.queue_modal(Modal::confirm(
                        t("modals.emuDownload.title"),
                        crate::i18n::strip_html(&t("modals.emuDownload.body")),
                        t("modals.emuDownload.yes"),
                        t("modals.emuDownload.no"),
                        false,
                        Action::EmuLobbyConnect(true),
                    ));
                }
                (Err(err), _) => {
                    app.emit("emu_network", Some(serde_json::json!({ "action": "lobby", "ok": false })));
                    let msg = if err == "GBE_TOOLS_MISSING" { t("emulator.network.lobbyMissing") } else { localize_error(&err) };
                    app.extra_status(Tone::Error, msg);
                }
                (Ok(path), Some(Err(err))) if err == "WINDOWS_TOOL_ON_LINUX" => {
                    app.emit("emu_network", Some(serde_json::json!({ "action": "lobby", "ok": true })));
                    app.extra_status(Tone::Success, tf("emulator.network.lobbyProton", &[("path", &path)]));
                }
                (Ok(_), Some(Err(err))) => {
                    app.emit("emu_network", Some(serde_json::json!({ "action": "lobby", "ok": false })));
                    app.extra_status(Tone::Error, localize_error(&err));
                }
                (Ok(_), _) => {
                    app.emit("emu_network", Some(serde_json::json!({ "action": "lobby", "ok": true })));
                    app.extra_status(Tone::Success, t("emulator.network.lobbyStarted"));
                }
            })
        });
    }

    fn mod_add(&mut self) {
        let Some(e) = self.emu.as_ref() else { return };
        let targets = e.extra_targets();
        let id = e.extras.mod_id.trimmed();
        let src = e.extras.mod_path.trimmed();
        if id.is_empty() {
            self.extra_status(Tone::Error, t("emulator.mods.needId"));
            return;
        }
        if src.is_empty() {
            self.extra_status(Tone::Error, t("tui.emu.extras.fileNeeded"));
            return;
        }
        let app_id = e.app_id.clone();
        let src = expand(&src);
        self.extra_status(Tone::Busy, t("emulator.mods.adding"));
        let core = self.core.clone();
        self.spawn(async move {
            let r = extras::add_mod(&core.http_client, &targets, &app_id, &id, &src).await;
            apply(move |app| {
                match r {
                    Ok(m) => {
                        app.emit("emu_mod", Some(serde_json::json!({ "action": "add", "ok": true, "details": m.title != m.id, "other_app": m.other_app.is_some() })));
                        let mut msg = tf("emulator.mods.added", &[("title", &m.title)]);
                        if let Some(other) = &m.other_app {
                            msg.push('\n');
                            msg.push_str(&tf("emulator.mods.otherApp", &[("app", other)]));
                        }
                        app.extra_status(if m.other_app.is_some() { Tone::Warning } else { Tone::Success }, msg);
                        if let Some(e) = app.emu.as_mut() {
                            e.extras.mod_id.set(String::new());
                            e.extras.mod_path.set(String::new());
                        }
                    }
                    Err(err) => {
                        app.emit("emu_mod", Some(serde_json::json!({ "action": "add", "ok": false })));
                        app.extra_status(Tone::Error, localize_error(&err));
                    }
                }
                app.load_extra_section(EXTRA_MODS);
            })
        });
    }

    fn mod_remove(&mut self, i: usize) {
        let Some(e) = self.emu.as_ref() else { return };
        let Some(m) = e.extras.mods.get(i).cloned() else { return };
        let targets = e.extra_targets();
        match extras::remove_mod(&targets, &m.id) {
            Ok(()) => self.emit("emu_mod", Some(serde_json::json!({ "action": "remove", "ok": true }))),
            Err(err) => self.extra_status(Tone::Error, localize_error(&err)),
        }
        self.load_extra_section(EXTRA_MODS);
    }
}
