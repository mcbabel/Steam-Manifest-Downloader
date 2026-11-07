use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use serde_json::Value;
use smd_core::ops::emulator::DOWNLOAD_CONFIRM_PREFIX;
use smd_core::services::emulator::{Platform, ReplaceResult, ScannedFile, Variant};

use super::action::{Action, InputId, ScrollTarget, Step};
use super::state::EmuState;
use super::{apply, hint, App, Modal};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::widgets::{self, Btn, ButtonSpec, InputSpec, Tone};
use crate::ui::{take_bottom, Ctx, Fid, ScrollView};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NumKind {
    Text,
    Int,
    Float,
}

pub struct TextField {
    pub key: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub section: usize,
    pub kind: NumKind,
}

pub struct BoolField {
    pub key: &'static str,
    pub label: &'static str,
    pub section: usize,
    pub invert: bool,
}

pub const SECTIONS: [&str; 12] = [
    "emulator.sections.account",
    "emulator.sections.saves",
    "emulator.sections.system",
    "emulator.sections.network",
    "emulator.sections.stats",
    "emulator.sections.overlay",
    "emulator.sections.compat",
    "emulator.sections.controller",
    "emulator.tabs.saves",
    "emulator.tabs.profile",
    "emulator.tabs.network",
    "emulator.tabs.mods",
];

pub const SYSTEM_SECTION: usize = 2;
pub const CONTROLLER_SECTION: usize = 7;

pub const HARDWARE_TYPES: [(&str, &str); 5] = [
    ("", "hwDefault"),
    ("0", "hwNone"),
    ("1", "hwDeck"),
    ("2", "hwMachine"),
    ("3", "hwFrame"),
];

const CONTROLLER_TYPES: [(&str, &str); 7] = [
    ("", "@emulator.fields.controllerDefault"),
    ("XBOX360", "Xbox 360"),
    ("XBOXONE", "Xbox One"),
    ("PS3", "PlayStation 3"),
    ("PS4", "PlayStation 4"),
    ("PS5", "PlayStation 5"),
    ("SWITCH", "Nintendo Switch"),
];

pub const TEXT_FIELDS: &[TextField] = &[
    TextField {
        key: "account_name",
        label: "accountName",
        placeholder: "@emulator.fields.accountNamePlaceholder",
        section: 0,
        kind: NumKind::Text,
    },
    TextField {
        key: "account_steamid",
        label: "accountSteamId",
        placeholder: "76561198000000000",
        section: 0,
        kind: NumKind::Text,
    },
    TextField {
        key: "language",
        label: "language",
        placeholder: "english",
        section: 0,
        kind: NumKind::Text,
    },
    TextField {
        key: "ip_country",
        label: "ipCountry",
        placeholder: "US",
        section: 0,
        kind: NumKind::Text,
    },
    TextField {
        key: "local_save_path",
        label: "localSavePath",
        placeholder: "@emulator.fields.localSavePathPlaceholder",
        section: 1,
        kind: NumKind::Text,
    },
    TextField {
        key: "saves_folder_name",
        label: "savesFolderName",
        placeholder: "GSE Saves",
        section: 1,
        kind: NumKind::Text,
    },
    TextField {
        key: "listen_port",
        label: "listenPort",
        placeholder: "47584",
        section: 3,
        kind: NumKind::Int,
    },
    TextField {
        key: "hook_delay_sec",
        label: "hookDelay",
        placeholder: "0",
        section: 5,
        kind: NumKind::Int,
    },
    TextField {
        key: "key_combo",
        label: "keyCombo",
        placeholder: "shift + tab",
        section: 5,
        kind: NumKind::Text,
    },
    TextField {
        key: "screenshot_combo",
        label: "screenshotCombo",
        placeholder: "f12",
        section: 5,
        kind: NumKind::Text,
    },
    TextField {
        key: "font_size",
        label: "fontSize",
        placeholder: "16",
        section: 5,
        kind: NumKind::Float,
    },
    TextField {
        key: "notification_duration_achievement",
        label: "notificationDuration",
        placeholder: "7",
        section: 5,
        kind: NumKind::Float,
    },
];

pub const BOOL_FIELDS: &[BoolField] = &[
    BoolField {
        key: "is_under_proton",
        label: "isUnderProton",
        section: 2,
        invert: false,
    },
    BoolField {
        key: "offline",
        label: "offline",
        section: 2,
        invert: false,
    },
    BoolField {
        key: "enable_account_avatar",
        label: "enableAccountAvatar",
        section: 2,
        invert: false,
    },
    BoolField {
        key: "new_app_ticket",
        label: "oldAppTicket",
        section: 2,
        invert: true,
    },
    BoolField {
        key: "disable_networking",
        label: "disableNetworking",
        section: 3,
        invert: false,
    },
    BoolField {
        key: "disable_lan_only",
        label: "disableLanOnly",
        section: 3,
        invert: false,
    },
    BoolField {
        key: "disable_lobby_creation",
        label: "disableLobbyCreation",
        section: 3,
        invert: false,
    },
    BoolField {
        key: "share_leaderboards_over_network",
        label: "shareLeaderboards",
        section: 3,
        invert: false,
    },
    BoolField {
        key: "record_playtime",
        label: "recordPlaytime",
        section: 4,
        invert: false,
    },
    BoolField {
        key: "pause_total_when_unfocused",
        label: "pauseTotal",
        section: 4,
        invert: false,
    },
    BoolField {
        key: "pause_session_when_unfocused",
        label: "pauseSession",
        section: 4,
        invert: false,
    },
    BoolField {
        key: "achievements_bypass",
        label: "achievementsBypass",
        section: 4,
        invert: false,
    },
    BoolField {
        key: "allow_unknown_stats",
        label: "allowUnknownStats",
        section: 4,
        invert: false,
    },
    BoolField {
        key: "enable_experimental_overlay",
        label: "enableExperimentalOverlay",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "enable_screenshot",
        label: "disableScreenshot",
        section: 5,
        invert: true,
    },
    BoolField {
        key: "overlay_always_show_fps",
        label: "overlayAlwaysShowFps",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "overlay_always_show_frametime",
        label: "overlayAlwaysShowFrametime",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "overlay_always_show_playtime",
        label: "overlayAlwaysShowPlaytime",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "overlay_always_show_user_info",
        label: "overlayAlwaysShowUserInfo",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "disable_achievement_notification",
        label: "disableAchievementNotification",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "disable_achievement_progress",
        label: "showAchievementProgress",
        section: 5,
        invert: true,
    },
    BoolField {
        key: "disable_friend_notification",
        label: "disableFriendNotification",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "disable_warning_any",
        label: "disableWarnings",
        section: 5,
        invert: false,
    },
    BoolField {
        key: "force_steamhttp_success",
        label: "forceSteamhttpSuccess",
        section: 6,
        invert: false,
    },
    BoolField {
        key: "enable_steam_preowned_ids",
        label: "enableSteamPreownedIds",
        section: 6,
        invert: false,
    },
    BoolField {
        key: "free_weekend",
        label: "freeWeekend",
        section: 6,
        invert: false,
    },
    BoolField {
        key: "disable_steamoverlaygameid_env_var",
        label: "controller.steamInput",
        section: CONTROLLER_SECTION,
        invert: false,
    },
];

pub fn field_text(label: &str) -> String {
    if label.contains('.') {
        format!("emulator.{}", label)
    } else {
        format!("emulator.fields.{}", label)
    }
}

fn controller_summary(r: &smd_core::services::game_data::controller::ControllerResult) -> String {
    let line = if r.sets == 0 {
        t("emulator.controller.nothing")
    } else if r.source == "file" {
        tf("emulator.controller.writtenFile", &[("count", &r.sets)])
    } else if r.glyphs > 0 {
        tf("emulator.controller.written", &[("count", &r.sets), ("glyphs", &r.glyphs)])
    } else {
        tf("emulator.controller.writtenNoGlyphs", &[("count", &r.sets)])
    };
    let mut lines = vec![line];
    for note in &r.notes {
        lines.push(t(&format!("emulator.{}", note)));
    }
    lines.join("\n")
}

fn placeholder(p: &str) -> String {
    match p.strip_prefix('@') {
        Some(key) => t(key),
        None => p.to_string(),
    }
}

pub fn depot_folder(path: &str) -> String {
    let norm = path.replace('\\', "/");
    match norm.find("/depots/") {
        Some(i) => {
            let rest = &norm[i + "/depots/".len()..];
            rest.split('/').next().unwrap_or("").to_string()
        }
        None => String::new(),
    }
}

pub fn pick_default_depot(files: &[ScannedFile]) -> Option<String> {
    let host_linux = cfg!(target_os = "linux");
    let mut groups: Vec<(String, Vec<&ScannedFile>)> = Vec::new();
    for f in files {
        let key = Some(depot_folder(&f.path))
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| "__root__".into());
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, g)) => g.push(f),
            None => groups.push((key, vec![f])),
        }
    }
    let mut best: Option<(String, i64)> = None;
    for (key, group) in groups {
        let mut score = group.len() as i64;
        if group
            .iter()
            .any(|f| (f.platform == Platform::Linux) == host_linux)
        {
            score += 1000;
        }
        if group.iter().any(|f| f.arch == "x64") {
            score += 100;
        }
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((key, score));
        }
    }
    best.map(|(k, _)| k)
}

fn file_label(path: &str) -> String {
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();
    let n = parts.len();
    if n >= 2 {
        format!("{}/{}", parts[n - 2], parts[n - 1])
    } else {
        path.to_string()
    }
}

fn outcome(success: usize, total: usize) -> &'static str {
    if total == 0 {
        "failed"
    } else if success == total {
        "complete"
    } else if success > 0 {
        "partial"
    } else {
        "failed"
    }
}

fn game_data_summary(r: &smd_core::services::game_data::GameDataResult) -> String {
    let mut parts = Vec::new();
    let mut add = |n: usize, key: &str| {
        if n > 0 {
            parts.push(tf(&format!("emulator.gameData.{}", key), &[("count", &n)]));
        }
    };
    add(r.languages, "languages");
    add(r.depots, "depots");
    add(r.branches, "branches");
    if r.achievements > 0 {
        parts.push(if r.achievement_languages > 1 {
            tf(
                "emulator.gameData.achievementLanguages",
                &[("count", &r.achievements), ("languages", &r.achievement_languages), ("icons", &r.icons)],
            )
        } else {
            tf("emulator.gameData.achievements", &[("count", &r.achievements), ("icons", &r.icons)])
        });
    }
    let mut add = |n: usize, key: &str| {
        if n > 0 {
            parts.push(tf(&format!("emulator.gameData.{}", key), &[("count", &n)]));
        }
    };
    add(r.stats, "stats");
    add(r.leaderboards, "leaderboards");
    add(r.items, "items");
    add(r.cloud_dirs, "cloud");
    add(r.watcher_schemas, "watcher");
    add(r.media_files, "mediaFiles");
    let mut lines = vec![if parts.is_empty() {
        t("emulator.gameData.nothing")
    } else {
        tf("emulator.gameData.written", &[("list", &parts.join(", "))])
    }];
    for note in &r.notes {
        lines.push(t(&format!("emulator.{}", note)));
    }
    lines.join("\n")
}

pub(super) fn count_bucket(n: usize) -> &'static str {
    match n {
        0 => "0",
        1 => "1",
        2..=3 => "2-3",
        4..=10 => "4-10",
        _ => ">10",
    }
}

fn platform_mix(targets: &[ScannedFile]) -> &'static str {
    let win = targets.iter().any(|t| t.platform == Platform::Windows);
    let linux = targets.iter().any(|t| t.platform == Platform::Linux);
    match (win, linux) {
        (true, true) => "mixed",
        (false, true) => "linux",
        (true, false) => "windows",
        _ => "none",
    }
}

impl App {
    fn emu_entry(&self) -> &'static str {
        match &self.emu {
            Some(e) if e.standalone => "standalone",
            Some(e) if e.edit_mode => "history",
            _ => "download",
        }
    }

    pub(super) fn enter_emulator(
        &mut self,
        mut emu: EmuState,
        scan: Vec<ScannedFile>,
        prefill: bool,
    ) {
        if prefill && !emu.edit_mode {
            if let Some(s) = self.prefs.last_emu_settings.clone() {
                emu.populate(&s);
            }
        }
        emu.refresh_patched(scan);
        self.emu = Some(emu);
        self.wiz.step = Step::Emulator;
        self.page = super::action::Page::Wizard;
        self.focus = None;
        self.emu_side_scans();
        self.hydrate_emu();
    }

    pub(super) fn go_to_emulator_step(&mut self) {
        let Some(dir) = self.wiz.result_dir.clone() else {
            return;
        };
        let app_id = self.wiz.main_app_id().unwrap_or_default();
        let emu = EmuState::new(dir.clone(), app_id);
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| {
                let scan = r.unwrap_or_default();
                app.enter_emulator(emu, scan, true);
            })
        });
    }

    fn load_controller_info(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        e.controller_info = None;
        e.controller_variant_set = false;
        if e.app_id.trim().is_empty() {
            return;
        }
        e.controller_checking = true;
        let app_id = e.app_id.clone();
        let core = self.core.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::controller_info(&core, app_id.clone()).await;
            apply(move |app| {
                let Some(e) = app.emu.as_mut() else { return };
                if e.app_id != app_id {
                    return;
                }
                e.controller_checking = false;
                e.controller_info = r.ok();
                e.pick_experimental_for_controller();
            })
        });
    }

    fn emu_write_controller(&mut self, paths: Vec<String>) {
        let Some(e) = self.emu.as_ref() else { return };
        if !e.controller || paths.is_empty() || e.app_id.is_empty() {
            return;
        }
        let app_id = e.app_id.clone();
        let file = Some(e.controller_file.trimmed()).filter(|f| !f.is_empty());
        let variant = if e.experimental { "experimental" } else { "regular" };
        let steam_input = e.bools.get("disable_steamoverlaygameid_env_var").copied().unwrap_or(false);
        let core = self.core.clone();
        let data = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::write_controller(&core, &data, paths, app_id, file).await;
            apply(move |app| {
                let (tone, line) = match &r {
                    Ok(res) => {
                        app.emit("controller_applied", Some(serde_json::json!({
                            "source": res.source,
                            "sets": count_bucket(res.sets),
                            "glyphs": res.glyphs > 0,
                            "variant": variant,
                            "steam_input": steam_input,
                            "notes": if res.notes.is_empty() { vec!["none".to_string()] } else { res.notes.iter().map(|n| n.split('.').next_back().unwrap_or(n).to_string()).collect::<Vec<_>>() },
                        })));
                        (Tone::Success, controller_summary(res))
                    }
                    Err(err) => (
                        Tone::Error,
                        tf("emulator.controller.failed", &[("message", &crate::i18n::localize_error(err))]),
                    ),
                };
                match app.emu.as_mut().and_then(|e| e.status.as_mut()) {
                    Some((t, msg)) if *t != Tone::Busy => {
                        msg.push_str("\n\n");
                        msg.push_str(&line);
                    }
                    _ => app.toast(tone, line),
                }
            })
        });
    }

    pub(super) fn emu_search_tick(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        let Some(at) = e.search_changed_at else { return };
        if at.elapsed() < std::time::Duration::from_millis(1500) {
            return;
        }
        e.search_changed_at = None;
        let query = e.settings_search.trimmed().to_lowercase();
        if query.is_empty() {
            return;
        }
        let results = e.matching_settings(&query);
        self.emit(
            "emu_settings_view",
            Some(serde_json::json!({ "action": "search", "results": count_bucket(results) })),
        );
    }

    fn load_drm_backups(&mut self) {
        let Some(e) = self.emu.as_ref() else { return };
        let dir = e.game_dir.clone();
        self.spawn(async move {
            let list = smd_core::ops::emulator::drm_backups(dir).await;
            apply(move |app| {
                if let Some(e) = app.emu.as_mut() {
                    e.drm_backups = list;
                }
            })
        });
    }

    fn load_patch_info(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        let Some(first) = e.edit_targets.first() else {
            e.patch_info = None;
            return;
        };
        let info = smd_core::ops::emulator::patch_info(&first.path);
        if let Some(v) = info.variant.as_deref() {
            e.experimental = v == "experimental";
        }
        e.patch_info = Some(info);
    }

    fn restore_drm(&mut self, i: usize) {
        let Some(b) = self.emu.as_ref().and_then(|e| e.drm_backups.get(i)).cloned() else { return };
        match smd_core::ops::emulator::restore_drm(&b.path) {
            Ok(()) => {
                self.emit("drm_restored", Some(serde_json::json!({ "where": "single" })));
                self.toast(Tone::Success, t("emulator.drmRestore.done"));
            }
            Err(e) => self.toast(Tone::Error, crate::i18n::localize_error(&e)),
        }
        self.load_drm_backups();
        if let Some(e) = self.emu.as_ref() {
            let d = e.game_dir.clone();
            self.spawn(async move {
                let r = tokio::task::spawn_blocking(move || {
                    smd_core::services::steamless::scan_game_dir_for_drm(std::path::Path::new(&d))
                })
                .await
                .unwrap_or_default();
                apply(move |app| {
                    if let Some(e) = app.emu.as_mut() {
                        e.drm = r;
                    }
                })
            });
        }
    }

    fn emu_update_gbe(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        if e.busy || e.edit_targets.is_empty() {
            return;
        }
        e.selected = e.edit_targets.iter().map(|t| t.path.clone()).collect();
        e.selection_explicit = true;
        let from = if e.patch_info.as_ref().is_some_and(|p| p.tag.is_some()) { "known" } else { "unknown" };
        self.emit("emu_gbe_updated", Some(serde_json::json!({ "from": from })));
        self.emu_apply();
    }

    fn load_cold_exes(&mut self) {
        let Some(e) = self.emu.as_ref() else { return };
        if e.edit_mode || !e.scan.iter().any(|f| f.platform == Platform::Windows && f.loader.is_none()) {
            return;
        }
        let dir = e.game_dir.clone();
        self.spawn(async move {
            let list = smd_core::ops::shortcuts::detect_executables(&dir).await.unwrap_or_default();
            apply(move |app| {
                let Some(e) = app.emu.as_mut() else { return };
                let exes: Vec<_> = list
                    .into_iter()
                    .filter(|x| x.platform == "windows" && !x.name.to_ascii_lowercase().starts_with("steamclient_loader"))
                    .collect();
                e.cold_idx = exes.iter().position(|x| x.recommended).unwrap_or(0);
                e.cold_exes = exes.into_iter().map(|x| x.path).collect();
                if e.cold_exes.is_empty() {
                    e.cold = false;
                }
            })
        });
    }

    fn note_coldclient(&mut self, results: &[ReplaceResult], fresh: bool) {
        let running = results.iter().any(|r| r.shortcut.as_deref() == Some("steam_running"));
        let links: usize = results.iter().map(|r| r.links).sum();
        if let Some(e) = self.emu.as_mut() {
            e.cold_steam_running = running;
        }
        if links > 0 {
            self.toast(Tone::Success, tf("emulator.method.linksDone", &[("count", &links)]));
        }
        if fresh {
            let ok = results.iter().any(|r| r.success && r.shortcut.is_some()) || results.iter().any(|r| r.success);
            let shortcut = results.iter().find_map(|r| r.shortcut.clone()).unwrap_or_else(|| "none".into());
            self.emit("coldclient_used", Some(serde_json::json!({ "action": "setup", "ok": ok, "shortcut": shortcut, "links": links > 0 })));
        }
    }

    fn cold_steam(&mut self) {
        let Some(e) = self.emu.as_ref() else { return };
        let dir = e.game_dir.clone();
        self.spawn(async move {
            let states = smd_core::ops::emulator::coldclient_shortcut(dir, false, true).await;
            apply(move |app| {
                let ok = states.iter().any(|s| s == "updated");
                app.emit("coldclient_used", Some(serde_json::json!({ "action": "shortcut", "ok": ok })));
                if ok {
                    if let Some(e) = app.emu.as_mut() {
                        e.cold_steam_running = false;
                    }
                    app.toast(Tone::Success, t("emulator.method.shortcutDone"));
                } else {
                    app.toast(Tone::Error, t("emulator.method.shortcutFailed"));
                }
            })
        });
    }

    fn emu_side_scans(&mut self) {
        self.load_controller_info();
        self.load_cold_exes();
        let Some(emu) = &self.emu else { return };
        let dir = emu.game_dir.clone();
        let app_id = emu.app_id.clone();
        let standalone = emu.standalone;
        let edit = emu.edit_mode;
        let core = self.core.clone();
        let data = self.data_dir.clone();
        self.load_drm_backups();
        self.load_patch_info();
        {
            let core = core.clone();
            let data = data.clone();
            self.spawn(async move {
                let r = smd_core::services::emulator::fetch_release_info(&core.http_client, &data)
                    .await;
                apply(move |app| {
                    if let Some(e) = app.emu.as_mut() {
                        e.release = Some(r);
                    }
                })
            });
        }
        if !edit {
            let d = dir.clone();
            self.spawn(async move {
                let r = tokio::task::spawn_blocking(move || {
                    smd_core::services::steamless::scan_game_dir_for_drm(std::path::Path::new(&d))
                })
                .await
                .unwrap_or_default();
                apply(move |app| {
                    if let Some(e) = app.emu.as_mut() {
                        e.drm = r;
                    }
                })
            });
        }
        if !standalone && !edit {
            let core = self.core.clone();
            self.spawn(async move {
                let r = smd_core::ops::emulator::scan_for_dlc_merge(
                    &core,
                    dir,
                    Some(app_id).filter(|s| !s.is_empty()),
                )
                .await;
                apply(move |app| {
                    if let (Some(e), Ok(Some(plan))) = (app.emu.as_mut(), r) {
                        if !plan.to_merge.is_empty() {
                            e.merge_plan = Some(plan);
                        }
                    }
                })
            });
        }
    }

    fn hydrate_emu(&mut self) {
        let Some(emu) = self.emu.as_mut() else { return };
        if emu.edit_targets.is_empty() {
            emu.settings_prefill_path = None;
            emu.bypass_initial = false;
            if emu.scan.iter().any(|f| f.platform == Platform::Windows) {
                emu.bypass = false;
            }
            return;
        }
        let first = emu.edit_targets[0].path.clone();
        let paths: Vec<String> = emu.edit_targets.iter().map(|t| t.path.clone()).collect();
        self.spawn(async move {
            let settings = smd_core::ops::emulator::read_emu_settings(&first);
            let installed = paths.iter().any(|p| {
                smd_core::services::steam_api_bypass::is_installed_for_target(std::path::Path::new(
                    p,
                ))
            });
            apply(move |app| {
                let Some(e) = app.emu.as_mut() else { return };
                match settings {
                    Ok(s) => {
                        e.populate(&s);
                        e.settings_prefill_path = Some(first);
                    }
                    Err(err) => {
                        e.settings_prefill_path = None;
                        e.status = Some((
                            Tone::Error,
                            tf("emulator.settingsReadError", &[("message", &err)]),
                        ));
                    }
                }
                e.bypass_initial = installed;
                if e.scan.iter().any(|f| f.platform == Platform::Windows) {
                    e.bypass = installed;
                }
            })
        });
    }

    fn refresh_emu_scan(&mut self) {
        let Some(emu) = &self.emu else { return };
        let dir = emu.game_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| {
                if let (Some(e), Ok(scan)) = (app.emu.as_mut(), r) {
                    e.refresh_patched(scan);
                    e.settings_prefill_path = e.edit_targets.first().map(|t| t.path.clone());
                }
                app.load_patch_info();
                app.load_drm_backups();
            })
        });
    }

    pub(super) fn on_emu_download_event(&mut self, v: Value) {
        let Some(e) = self.emu.as_mut() else { return };
        let platform = match v["platform"].as_str() {
            Some("windows") => "Windows",
            Some("linux") => "Linux",
            _ => "",
        };
        let done = v["downloaded"].as_u64().unwrap_or(0);
        let total = v["total"].as_u64().unwrap_or(0);
        let msg = if total > 0 && done >= total {
            tf(
                "emulator.emuExtracting",
                &[(
                    "platform",
                    &if platform.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", platform)
                    },
                )],
            )
        } else {
            let progress = match (done * 100).checked_div(total) {
                Some(pct) => format!(
                    "{}% · {}/{}",
                    pct,
                    widgets::fmt_bytes(done),
                    widgets::fmt_bytes(total)
                ),
                None => widgets::fmt_bytes(done),
            };
            tf(
                "emulator.emuDownloading",
                &[(
                    "progress",
                    &format!("{} {}", platform, progress).trim().to_string(),
                )],
            )
        };
        e.status = Some((Tone::Busy, msg));
    }

    pub(super) fn dispatch_emulator(&mut self, a: &Action) -> bool {
        match a {
            Action::EmuToggleFile(i) => {
                if let Some(e) = self.emu.as_mut() {
                    if let Some(f) = e.scan.get(*i) {
                        let p = f.path.clone();
                        if e.selected.is_empty() && !e.selection_explicit {
                            e.selected = default_selection(&e.scan);
                        }
                        e.selection_explicit = true;
                        if !e.selected.remove(&p) {
                            e.selected.insert(p);
                        }
                    }
                }
            }
            Action::EmuSelectAllFiles => {
                if let Some(e) = self.emu.as_mut() {
                    e.selection_explicit = true;
                    if e.selected.len() == e.scan.len() {
                        e.selected.clear();
                    } else {
                        e.selected = e.scan.iter().map(|f| f.path.clone()).collect();
                    }
                }
            }
            Action::EmuVariant(experimental) => {
                if let Some(e) = self.emu.as_mut() {
                    e.experimental = *experimental;
                }
            }
            Action::EmuToggleBypass => {
                if let Some(e) = self.emu.as_mut() {
                    if e.scan.iter().any(|f| f.platform == Platform::Windows) {
                        e.bypass = !e.bypass;
                    }
                }
            }
            Action::EmuSection(i) => {
                if let Some(e) = self.emu.as_mut() {
                    e.section = *i;
                    e.settings_search.set(String::new());
                    e.settings_changed_only = false;
                    e.search_changed_at = None;
                }
                if *i >= super::emu_extras::EXTRA_SAVES {
                    self.load_extra_section(*i);
                }
            }
            Action::EmuToggleGameData => {
                self.prefs.emu_game_data_off = !self.prefs.emu_game_data_off;
                self.save_prefs();
            }
            Action::EmuWriteGameData => {
                let Some(e) = self.emu.as_mut() else { return true };
                if e.busy {
                    return true;
                }
                let targets = e.selected_targets();
                if targets.is_empty() || e.app_id.is_empty() {
                    e.status = Some((Tone::Error, t("emulator.applyNoSelection")));
                    return true;
                }
                e.status = Some((Tone::Busy, t("emulator.gameData.busy")));
                let language = e.gather().and_then(|g| g.language);
                let app_id = e.app_id.clone();
                let paths = targets.iter().map(|t| t.path.clone()).collect();
                self.emu_write_game_data(paths, app_id, language);
            }
            Action::EmuToggleField(i) => {
                if let (Some(e), Some(f)) = (self.emu.as_mut(), BOOL_FIELDS.get(*i)) {
                    let v = e.bools.get(f.key).copied().unwrap_or(false);
                    e.bools.insert(f.key, !v);
                }
            }
            Action::EmuHardwareType(i) => {
                if let (Some(e), Some((value, _))) = (self.emu.as_mut(), HARDWARE_TYPES.get(*i)) {
                    e.hardware_type = value.to_string();
                }
            }
            Action::EmuSettingsChangedOnly => {
                let on = self.emu.as_mut().map(|e| {
                    e.settings_changed_only = !e.settings_changed_only;
                    e.settings_changed_only
                });
                if on == Some(true) {
                    self.emit("emu_settings_view", Some(serde_json::json!({ "action": "changed_only" })));
                }
            }
            Action::EmuSettingsReset => {
                if let Some(e) = self.emu.as_mut() {
                    e.reset_settings();
                }
                self.toast(Tone::Success, t("emulator.settingsResetDone"));
            }
            Action::EmuToggleController => {
                if let Some(e) = self.emu.as_mut() {
                    e.controller = !e.controller;
                    e.pick_experimental_for_controller();
                }
            }
            Action::EmuControllerType(i) => {
                if let (Some(e), Some((value, _))) = (self.emu.as_mut(), CONTROLLER_TYPES.get(*i)) {
                    e.controller_type = value.to_string();
                }
            }
            a if self.dispatch_emu_extras(a) => {}
            Action::EmuMethod(cold) => {
                if let Some(e) = self.emu.as_mut() {
                    e.cold = *cold && !e.cold_exes.is_empty();
                }
            }
            Action::EmuColdExe(i) => {
                if let Some(e) = self.emu.as_mut() {
                    e.cold_idx = *i;
                }
            }
            Action::EmuColdSteam => self.cold_steam(),
            Action::EmuUseLocal => {
                self.close_modal();
                if let Some(e) = self.emu.as_mut() {
                    e.prefer_local = true;
                }
                self.emu_apply();
            }
            Action::EmuApply => self.emu_apply(),
            Action::EmuDownloadConfirmed => {
                self.close_modal();
                if let Some(e) = self.emu.as_mut() {
                    e.allow_download = true;
                    e.status = Some((
                        Tone::Busy,
                        tf("emulator.emuDownloading", &[("progress", &"0%")]),
                    ));
                }
                self.emu_apply();
            }
            Action::EmuDownloadCancelled => {
                self.close_modal();
                let targets = self
                    .emu
                    .as_ref()
                    .map(|e| e.selected_targets())
                    .unwrap_or_default();
                let variant = if self.emu.as_ref().is_some_and(|e| e.experimental) {
                    "experimental"
                } else {
                    "regular"
                };
                self.emit(
                    "patch_applied",
                    Some(serde_json::json!({
                        "entry": self.emu_entry(), "outcome": "cancelled", "variant": variant,
                        "platforms": platform_mix(&targets), "targets": count_bucket(targets.len()),
                        "failures": "0", "fail_class": null,
                    })),
                );
                if let Some(e) = self.emu.as_mut() {
                    e.status = Some((Tone::Error, t("emulator.applyCancelled")));
                }
            }
            Action::EmuSave => self.emu_save(),
            Action::AskEmuRevert => {
                let count = self.emu.as_ref().map(|e| e.edit_targets.len()).unwrap_or(0);
                let mut m = Modal::confirm(
                    t("modals.emuRevert.title"),
                    t("modals.emuRevert.body"),
                    t("modals.emuRevert.yes"),
                    t("modals.emuRevert.no"),
                    true,
                    Action::EmuRevertConfirmed,
                );
                let loader = self.emu.as_ref().is_some_and(|e| e.edit_targets.iter().any(|t| t.loader.is_some()));
                if let Modal::Confirm(c) = &mut m {
                    let mut note = tf("modals.emuRevert.scope", &[("count", &count)]);
                    if loader {
                        note.push(' ');
                        note.push_str(&t("modals.emuRevert.loader"));
                    }
                    c.note = Some(note);
                }
                self.queue_modal(m);
            }
            Action::EmuRevertConfirmed => {
                self.close_modal();
                self.emu_revert();
            }
            Action::EmuMergeDlc => self.emu_merge(),
            Action::EmuRemoveDrm => self.emu_remove_drm(),
            Action::EmuRestoreDrm(i) => self.restore_drm(*i),
            Action::EmuUpdateGbe => self.emu_update_gbe(),
            Action::EmuDone => self.reset_wizard(),
            _ => return false,
        }
        true
    }

    fn emu_apply(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        if e.apply_complete {
            self.reset_wizard();
            return;
        }
        let cold_exe = if e.cold && !e.edit_mode { e.cold_exes.get(e.cold_idx).cloned() } else { None };
        let mut targets = e.selected_targets();
        if cold_exe.is_some() {
            targets.retain(|t| t.platform != Platform::Windows || t.loader.is_some());
        }
        if targets.is_empty() && cold_exe.is_none() {
            e.status = Some((Tone::Error, t("emulator.applyNoSelection")));
            return;
        }
        if e.app_id.is_empty() {
            e.status = Some((
                Tone::Error,
                tf("emulator.applyError", &[("message", &t("tui.errors.noAppId"))]),
            ));
            return;
        }
        e.busy = true;
        e.av_blocked = false;
        if !e.allow_download {
            e.status = Some((Tone::Busy, t("emulator.applying")));
        }
        let variant = if e.experimental {
            Variant::Experimental
        } else {
            Variant::Regular
        };
        let variant_name = if e.experimental {
            "experimental"
        } else {
            "regular"
        };
        let gathered = e.gather();
        let app_id = e.app_id.clone();
        let allow = e.allow_download;
        e.allow_download = false;
        let prefer_local = std::mem::take(&mut e.prefer_local);
        let fresh_cold = cold_exe.is_some();
        let installed: Vec<String> = self
            .wiz
            .parsed
            .as_ref()
            .map(|p| {
                p.all_app_ids
                    .iter()
                    .filter(|id| **id != app_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let core = self.core.clone();
        let sink = self.sink.clone();
        let data = self.data_dir.clone();
        let tgt = targets.clone();
        let settings = gathered.clone().unwrap_or_default();
        self.spawn(async move {
            let r = smd_core::ops::emulator::apply_replacement(
                &sink,
                &core,
                &data,
                tgt,
                variant,
                app_id,
                installed,
                Some(settings),
                allow,
                prefer_local,
                cold_exe,
            )
            .await;
            apply(move |app| {
                if let Ok(results) = &r {
                    app.note_coldclient(results, fresh_cold);
                }
                app.on_emu_applied(r, targets, variant_name, gathered)
            })
        });
    }

    fn on_emu_applied(
        &mut self,
        r: Result<Vec<ReplaceResult>, String>,
        targets: Vec<ScannedFile>,
        variant: &'static str,
        gathered: Option<smd_core::services::emulator::EmuSettings>,
    ) {
        let entry = self.emu_entry();
        match r {
            Ok(results) => {
                let total = results.len();
                let success = results.iter().filter(|r| r.success).count();
                let failed = total - success;
                let fail_class = results
                    .iter()
                    .find(|r| !r.success)
                    .map(|r| r.fail_class.clone().unwrap_or_else(|| "unknown".into()));
                self.emit(
                    "patch_applied",
                    Some(serde_json::json!({
                        "entry": entry, "outcome": outcome(success, total), "variant": variant,
                        "platforms": platform_mix(&targets), "targets": count_bucket(total),
                        "failures": count_bucket(failed), "fail_class": fail_class,
                    })),
                );
                if failed == 0 {
                    let game_language = gathered.as_ref().and_then(|g| g.language.clone());
                    self.prefs.last_emu_settings = Some(gathered.unwrap_or_default());
                    self.save_prefs();
                    let dlc_note = results.iter().find_map(|r| r.dlc_count).map(|n| {
                        if n == 0 {
                            t("emulator.dlcNone")
                        } else {
                            tf("emulator.dlcActivated", &[("count", &n)])
                        }
                    });
                    let targets_c = targets.clone();
                    let game_paths: Vec<String> = targets.iter().map(|t| t.path.clone()).collect();
                    self.sync_bypass(targets_c, move |app, extra| {
                        let Some(e) = app.emu.as_mut() else { return };
                        e.busy = false;
                        let mut msg = tf(
                            "emulator.applySuccess",
                            &[("count", &success), ("total", &total)],
                        );
                        if let Some(note) = &dlc_note {
                            msg.push('\n');
                            msg.push_str(note);
                        }
                        if !extra.is_empty() {
                            msg.push_str("\n\n");
                            msg.push_str(&extra);
                        }
                        e.status = Some((Tone::Success, msg));
                        let app_id = e.app_id.clone();
                        if e.edit_mode {
                            app.refresh_emu_scan();
                        } else if e.standalone {
                            app.refresh_emu_scan();
                        } else {
                            e.apply_complete = true;
                            app.focus = Some(Fid::new("emu.apply"));
                        }
                        if !app.prefs.emu_game_data_off {
                            app.emu_write_game_data(game_paths.clone(), app_id, game_language);
                        }
                        app.emu_write_controller(game_paths);
                    });
                } else {
                    let details = results
                        .iter()
                        .filter(|r| !r.success)
                        .map(|r| {
                            format!(
                                "{}\n    {}",
                                file_label(&r.path),
                                r.error
                                    .clone()
                                    .unwrap_or_else(|| t("emulator.applyReasonUnknown"))
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Some(e) = self.emu.as_mut() {
                        e.busy = false;
                        e.status = Some((
                            Tone::Error,
                            format!(
                                "{}{}\n\n{}",
                                tf(
                                    "emulator.applyPartial",
                                    &[("success", &success), ("failed", &failed)]
                                ),
                                emu_error_hint(&details),
                                details
                            ),
                        ));
                        if e.standalone {
                            self.refresh_emu_scan();
                        }
                    }
                }
            }
            Err(err) => {
                if let Some(rest) = err.strip_prefix(DOWNLOAD_CONFIRM_PREFIX) {
                    if let Some(e) = self.emu.as_mut() {
                        e.busy = false;
                        e.status = None;
                    }
                    let info: Value = serde_json::from_str(rest).unwrap_or_default();
                    let tag = info["tag"].as_str().unwrap_or_default().to_string();
                    let items: Vec<String> = info["platforms"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default()
                        .iter()
                        .map(|p| {
                            let name = match p["platform"].as_str() {
                                Some("windows") => "Windows".to_string(),
                                Some("linux") => "Linux".to_string(),
                                other => other.unwrap_or_default().to_string(),
                            };
                            match p["size"].as_u64().filter(|s| *s > 0) {
                                Some(sz) => format!("{} (~{})", name, widgets::fmt_bytes(sz)),
                                None => name,
                            }
                        })
                        .collect();
                    let cold = self.emu.as_ref().is_some_and(|e| e.cold);
                    let local = match self.emu.as_ref().and_then(|e| e.release.as_ref()) {
                        Some(Ok(info)) => info.local_tag.clone().filter(|l| *l != tag),
                        _ => None,
                    };
                    let mut body = format!(
                        "{}\nhttps://github.com/Detanup01/gbe_fork/releases/tag/{}",
                        crate::i18n::strip_html(&t("modals.emuDownload.body")),
                        tag
                    );
                    if cold {
                        body.push_str("\n\n");
                        body.push_str(&t("modals.emuDownload.coldNote"));
                    }
                    let no = match &local {
                        Some(l) => tf("modals.emuDownload.keepLocal", &[("tag", l)]),
                        None => t("modals.emuDownload.no"),
                    };
                    let mut m = Modal::confirm(
                        t("modals.emuDownload.title"),
                        body,
                        t("modals.emuDownload.yes"),
                        no,
                        false,
                        Action::EmuDownloadConfirmed,
                    );
                    if let Modal::Confirm(c) = &mut m {
                        c.on_no = Some(if local.is_some() { Action::EmuUseLocal } else { Action::EmuDownloadCancelled });
                        c.note = Some(tf(
                            "modals.emuDownload.scope",
                            &[("tag", &tag), ("items", &items.join(", "))],
                        ));
                    }
                    self.queue_modal(m);
                    return;
                }
                let class = if err.contains("AV_BLOCKED") {
                    "av_blocked"
                } else if err.contains("GitHub") {
                    "release_fetch_failed"
                } else if err.contains("download returned HTTP") {
                    "emu_download_failed"
                } else {
                    "unknown"
                };
                self.emit(
                    "patch_applied",
                    Some(serde_json::json!({
                        "entry": entry, "outcome": "failed", "variant": variant,
                        "platforms": platform_mix(&targets), "targets": count_bucket(targets.len()),
                        "failures": count_bucket(targets.len()), "fail_class": class,
                    })),
                );
                if let Some(e) = self.emu.as_mut() {
                    e.busy = false;
                    if err.contains("AV_BLOCKED") {
                        e.av_blocked = true;
                        e.status = Some((
                            Tone::Error,
                            format!(
                                "{}\n{}",
                                t("emulator.avBlockedTitle"),
                                t("emulator.avBlockedHint")
                            ),
                        ));
                    } else {
                        e.status = Some((
                            Tone::Error,
                            tf("emulator.applyError", &[("message", &err)]) + &emu_error_hint(&err),
                        ));
                    }
                }
            }
        }
    }

    fn emu_write_game_data(&mut self, paths: Vec<String>, app_id: String, language: Option<String>) {
        if paths.is_empty() || app_id.is_empty() {
            return;
        }
        let core = self.core.clone();
        let data = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::emulator::generate_game_data(&core, &data, paths, app_id, language).await;
            apply(move |app| {
                let line = match &r {
                    Ok(res) => {
                        app.emit("game_data_written", Some(serde_json::json!({
                            "source": res.source,
                            "achievements": count_bucket(res.achievements),
                            "achievement_languages": count_bucket(res.achievement_languages),
                            "stats": count_bucket(res.stats),
                            "languages": count_bucket(res.languages),
                            "depots": count_bucket(res.depots),
                            "branches": count_bucket(res.branches),
                            "leaderboards": count_bucket(res.leaderboards),
                            "items": count_bucket(res.items),
                            "cloud_dirs": count_bucket(res.cloud_dirs),
                            "watcher": res.watcher_schemas > 0,
                            "media": smd_core::services::game_data::MediaLevel::parse(&app.settings.game_data_media).as_str(),
                            "icons": if res.achievements > 0 { Some(res.icons >= res.achievements) } else { None },
                            "notes": if res.notes.is_empty() { vec!["none"] } else { res.notes.iter().map(|n| n.trim_start_matches("gameData.")).collect::<Vec<_>>() },
                        })));
                        game_data_summary(res)
                    }
                    Err(err) => {
                        let key = crate::i18n::error_key(err).unwrap_or_else(|| "unmatched".to_string());
                        if app.reported_errors.len() < 40 && app.reported_errors.insert(format!("game_data:{}", key)) {
                            app.emit("error_shown", Some(serde_json::json!({ "area": "game_data", "key": key })));
                        }
                        tf("emulator.gameData.failed", &[("message", err)])
                    }
                };
                if let Some(e) = app.emu.as_mut() {
                    match e.status.as_mut() {
                        Some((tone, msg)) if *tone != Tone::Busy => {
                            msg.push_str("\n\n");
                            msg.push_str(&line);
                        }
                        _ => {
                            let found = r.as_ref().is_ok_and(|res| res.achievements > 0 || res.languages > 0);
                            e.status = Some((if found { Tone::Success } else { Tone::Error }, line));
                        }
                    }
                }
            })
        });
    }

    fn sync_bypass(
        &mut self,
        targets: Vec<ScannedFile>,
        done: impl FnOnce(&mut App, String) + Send + 'static,
    ) {
        let Some(e) = self.emu.as_ref() else { return };
        let has_windows = e.scan.iter().any(|f| f.platform == Platform::Windows);
        let want = e.bypass;
        if !has_windows || want == e.bypass_initial {
            done(self, String::new());
            return;
        }
        let core = self.core.clone();
        let data = self.data_dir.clone();
        self.spawn(async move {
            let (ok, msg) = if want {
                let windows: Vec<ScannedFile> = targets
                    .into_iter()
                    .filter(|t| t.platform == Platform::Windows)
                    .collect();
                if windows.is_empty() {
                    (true, String::new())
                } else {
                    let mut results = Vec::new();
                    for t in &windows {
                        results.push(
                            smd_core::services::steam_api_bypass::apply_to_target(
                                &core.http_client,
                                &data,
                                std::path::Path::new(&t.path),
                                t.arch == "x64",
                            )
                            .await,
                        );
                    }
                    let success = results.iter().filter(|r| r.success).count();
                    let failed = results.len() - success;
                    if failed == 0 {
                        (true, tf("emulator.bypassSuccess", &[("count", &success)]))
                    } else {
                        let detail = results
                            .iter()
                            .find(|r| !r.success)
                            .and_then(|r| r.error.clone())
                            .map(|e| format!("\n{}", e))
                            .unwrap_or_default();
                        (
                            false,
                            tf(
                                "emulator.bypassPartial",
                                &[("success", &success), ("failed", &failed)],
                            ) + &detail,
                        )
                    }
                }
            } else {
                for t in &targets {
                    let _ = smd_core::services::steam_api_bypass::revert_for_target(
                        std::path::Path::new(&t.path),
                    );
                }
                (true, String::new())
            };
            apply(move |app| {
                app.emit("api_bypass", Some(serde_json::json!({ "action": if want { "apply" } else { "revert" }, "ok": ok })));
                if ok {
                    if let Some(e) = app.emu.as_mut() {
                        e.bypass_initial = want;
                    }
                }
                done(app, msg);
            })
        });
    }

    fn emu_save(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        if e.edit_targets.is_empty() || e.busy {
            return;
        }
        if !std::path::Path::new(&e.game_dir).exists() {
            self.reset_wizard();
            self.folder_missing();
            return;
        }
        e.busy = true;
        e.status = Some((Tone::Busy, t("emulator.savingSettings")));
        let settings = e.gather().unwrap_or_default();
        let mut seen = std::collections::HashSet::new();
        let (mut success, mut failed) = (0usize, 0usize);
        for target in &e.edit_targets {
            let dir = std::path::Path::new(&target.path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            if !seen.insert(dir) {
                continue;
            }
            match smd_core::ops::emulator::write_emu_settings(&target.path, &settings) {
                Ok(()) => success += 1,
                Err(_) => failed += 1,
            }
        }
        let targets = e.edit_targets.clone();
        let controller_paths: Vec<String> = targets.iter().map(|t| t.path.clone()).collect();
        let entry = self.emu_entry();
        self.emit(
            "patch_settings_saved",
            Some(serde_json::json!({
                "entry": entry, "outcome": outcome(success, success + failed),
                "platforms": platform_mix(&targets), "targets": count_bucket(success + failed),
                "failures": count_bucket(failed),
            })),
        );
        if failed > 0 {
            if let Some(e) = self.emu.as_mut() {
                e.busy = false;
                e.status = Some((
                    Tone::Error,
                    tf(
                        "emulator.savePartial",
                        &[("success", &success), ("failed", &failed)],
                    ),
                ));
            }
            return;
        }
        self.sync_bypass(targets, move |app, extra| {
            app.emu_write_controller(controller_paths);
            let Some(e) = app.emu.as_mut() else { return };
            e.busy = false;
            if !e.standalone {
                app.reset_wizard();
                app.toast(
                    Tone::Success,
                    tf("emulator.saveSuccess", &[("count", &success)]),
                );
                return;
            }
            let mut msg = tf("emulator.saveSuccess", &[("count", &success)]);
            if !extra.is_empty() {
                msg.push_str("\n\n");
                msg.push_str(&extra);
            }
            e.status = Some((Tone::Success, msg));
        });
    }

    fn emu_revert(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        if e.edit_targets.is_empty() || e.busy {
            return;
        }
        if !std::path::Path::new(&e.game_dir).exists() {
            self.reset_wizard();
            self.folder_missing();
            return;
        }
        e.busy = true;
        e.status = Some((Tone::Busy, t("emulator.reverting")));
        let targets = e.edit_targets.clone();
        let paths: Vec<String> = targets.iter().map(|t| t.path.clone()).collect();
        let game_dir = e.game_dir.clone();
        let entry = self.emu_entry();
        self.spawn(async move {
            let r = smd_core::ops::emulator::revert_replacement(paths.clone(), true).await;
            for p in &paths {
                let _ = smd_core::services::steam_api_bypass::revert_for_target(std::path::Path::new(p));
            }
            let mut drm_restored = 0;
            for b in smd_core::ops::emulator::drm_backups(game_dir).await {
                if smd_core::ops::emulator::restore_drm(&b.path).is_ok() {
                    drm_restored += 1;
                }
            }
            apply(move |app| {
                if drm_restored > 0 {
                    app.emit("drm_restored", Some(serde_json::json!({ "where": "revert" })));
                }
                let results = match r {
                    Ok(r) => r,
                    Err(err) => {
                        if let Some(e) = app.emu.as_mut() {
                            e.busy = false;
                            e.status = Some((Tone::Error, tf("emulator.revertError", &[("message", &err)])));
                        }
                        return;
                    }
                };
                let success = results.iter().filter(|r| r.success).count();
                let failed = results.len() - success;
                let details = results
                    .iter()
                    .filter(|r| !r.success)
                    .map(|r| {
                        let why = if r.fail_class.as_deref() == Some("steam_running") {
                            t("emulator.method.steamRunning")
                        } else {
                            r.error.clone().unwrap_or_else(|| t("emulator.applyReasonUnknown"))
                        };
                        format!("{}\n    {}", file_label(&r.path), why)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                if targets.iter().any(|t| t.loader.is_some()) {
                    let links: usize = results.iter().map(|r| r.links).sum();
                    if links > 0 {
                        app.toast(Tone::Success, tf("emulator.method.linksBack", &[("count", &links)]));
                    }
                    app.emit("coldclient_used", Some(serde_json::json!({ "action": "removed", "ok": failed == 0, "links": links > 0 })));
                }
                app.emit("patch_reverted", Some(serde_json::json!({
                    "entry": entry, "outcome": outcome(success, results.len()),
                    "platforms": platform_mix(&targets), "targets": count_bucket(results.len()),
                    "failures": count_bucket(failed),
                    "fail_class": results.iter().find(|r| !r.success).map(|r| r.fail_class.clone().unwrap_or_else(|| "unknown".into())),
                })));
                let Some(e) = app.emu.as_mut() else { return };
                e.busy = false;
                if !e.standalone {
                    if failed > 0 {
                        e.status = Some((Tone::Error, format!("{}\n\n{}", tf("emulator.revertPartial", &[("success", &success), ("failed", &failed)]), details)));
                        return;
                    }
                    app.reset_wizard();
                    app.toast(Tone::Success, tf("emulator.revertSuccess", &[("count", &success)]));
                    return;
                }
                e.bypass_initial = false;
                e.bypass = false;
                e.status = Some(if failed > 0 {
                    (Tone::Error, format!("{}\n\n{}", tf("emulator.revertPartial", &[("success", &success), ("failed", &failed)]), details))
                } else {
                    (Tone::Success, tf("emulator.revertSuccess", &[("count", &success)]))
                });
                let dir = e.game_dir.clone();
                app.spawn(async move {
                    let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
                    apply(move |app| {
                        if let (Some(e), Ok(scan)) = (app.emu.as_mut(), r) {
                            e.refresh_patched(scan);
                            e.selected = e.scan.iter().filter(|f| !f.is_patched).map(|f| f.path.clone()).collect();
                            e.selection_explicit = true;
                            e.settings_prefill_path = e.edit_targets.first().map(|t| t.path.clone());
                        }
                    })
                });
            })
        });
    }

    fn emu_merge(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        let Some(plan) = e.merge_plan.clone() else {
            return;
        };
        e.merge_busy = true;
        e.merge_status = Some((Tone::Busy, t("emulator.dlcMergeBusy")));
        self.spawn(async move {
            let r = smd_core::ops::emulator::merge_dlc_depots(
                plan.main_depot_dir.clone(),
                plan.dlc_depot_dirs.clone(),
            )
            .await;
            apply(move |app| {
                app.emit("dlc_merged", Some(serde_json::json!({ "ok": r.is_ok(), "depots": count_bucket(plan.dlc_depot_dirs.len()) })));
                let Some(e) = app.emu.as_mut() else { return };
                e.merge_busy = false;
                match r {
                    Ok(_) => {
                        e.merge_plan = None;
                        e.merge_status = Some((
                            Tone::Success,
                            tf(
                                "emulator.dlcMergeDone",
                                &[("count", &plan.dlc_depot_dirs.len())],
                            ),
                        ));
                        app.refresh_emu_scan();
                    }
                    Err(err) => {
                        e.merge_status = Some((
                            Tone::Error,
                            tf("emulator.dlcMergeError", &[("message", &err)]),
                        ))
                    }
                }
            })
        });
    }

    fn emu_remove_drm(&mut self) {
        let Some(e) = self.emu.as_mut() else { return };
        if e.drm.is_empty() || e.drm_busy {
            return;
        }
        e.drm_busy = true;
        e.drm_status = Some((Tone::Busy, t("emulator.drmRemoving")));
        let paths: Vec<String> = e.drm.iter().map(|d| d.path.clone()).collect();
        let core = self.core.clone();
        let data = self.data_dir.clone();
        self.spawn(async move {
            let mut results = Vec::new();
            let mut fatal = None;
            for p in &paths {
                match smd_core::services::steamless::unpack_with_steamless(
                    &core.http_client,
                    &data,
                    std::path::Path::new(p),
                )
                .await
                {
                    Ok(r) => results.push(r),
                    Err(e) => {
                        fatal = Some(e);
                        break;
                    }
                }
            }
            apply(move |app| {
                let ok = results.iter().filter(|r| r.success).count();
                let outcome = if fatal.is_some() || ok == 0 {
                    "failed"
                } else if ok == results.len() {
                    "complete"
                } else {
                    "partial"
                };
                app.emit("steamless_used", Some(serde_json::json!({ "outcome": outcome, "targets": count_bucket(paths.len()) })));
                let Some(e) = app.emu.as_mut() else { return };
                e.drm_busy = false;
                let mono_hint = |msg: &str| {
                    let lower = msg.to_lowercase();
                    let needs = [
                        "command not found",
                        "no such file",
                        "cannot run",
                        "exec format",
                    ]
                    .iter()
                    .any(|n| lower.contains(n))
                        && lower.contains("mono");
                    if needs {
                        format!("\n\n{}", t("emulator.drmMonoHint"))
                    } else {
                        String::new()
                    }
                };
                if let Some(err) = fatal {
                    let h = mono_hint(&err);
                    e.drm_status = Some((
                        Tone::Error,
                        tf("emulator.drmRemoveError", &[("message", &err)])
                            + &h
                            + &emu_error_hint(&err),
                    ));
                    return;
                }
                let success = results.iter().filter(|r| r.success).count();
                let failed = results.len() - success;
                if failed == 0 {
                    e.drm_status = Some((
                        Tone::Success,
                        tf("emulator.drmRemoveSuccess", &[("count", &success)]),
                    ));
                    e.drm.clear();
                } else {
                    let msg = results
                        .iter()
                        .find(|r| !r.success)
                        .and_then(|r| r.error.clone())
                        .unwrap_or_else(|| t("tui.errors.unknown"));
                    let h = mono_hint(&msg);
                    e.drm_status = Some((
                        Tone::Error,
                        format!(
                            "{}\n\n{}{}{}",
                            tf(
                                "emulator.drmRemovePartial",
                                &[("success", &success), ("failed", &failed)]
                            ),
                            msg,
                            h,
                            emu_error_hint(&msg)
                        ),
                    ));
                }
            })
        });
    }

    pub(super) fn folder_missing(&mut self) {
        self.dispatch(Action::Nav(super::action::Page::History));
        self.hist.banner = Some((
            Tone::Error,
            t("modals.folderMissing.body"),
            std::time::Instant::now(),
        ));
    }

    pub(super) fn render_emulator(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let tick = self.tick;
        let Some(e) = self.emu.as_mut() else {
            widgets::status(buf, area, Tone::Busy, &t("common.loading"), tick);
            return;
        };
        let mut rest = area;

        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        if let Some((tone, msg)) = &e.status {
            let h = widgets::status_height(msg, rest.width).min(8);
            let r = take_bottom(&mut rest, h);
            widgets::status(buf, r, *tone, msg, tick);
            let _ = take_bottom(&mut rest, 1);
        }

        let selected = e.selected_targets();
        let patched_count = e.edit_targets.len();
        let mut specs = Vec::new();
        let home_label = if e.edit_mode {
            t("emulator.backToHome")
        } else {
            t("emulator.goToHome")
        };
        specs.push(ButtonSpec::new(
            home_label,
            Fid::new("emu.home"),
            Action::EmuDone,
            Btn::Secondary,
        ));
        if patched_count > 0 {
            specs.push(
                ButtonSpec::new(
                    t("emulator.revert"),
                    Fid::new("emu.revert"),
                    Action::AskEmuRevert,
                    Btn::Danger,
                )
                .enabled(!e.busy),
            );
            let primary = e.edit_mode || selected.is_empty();
            specs.push(
                ButtonSpec::new(
                    t("emulator.saveSettings"),
                    Fid::new("emu.save"),
                    Action::EmuSave,
                    if primary {
                        Btn::Primary
                    } else {
                        Btn::Secondary
                    },
                )
                .enabled(!e.busy && e.settings_prefill_path.is_some()),
            );
        }
        if !e.edit_mode {
            let label = if e.apply_complete {
                t("emulator.goBackHome")
            } else if !selected.is_empty() && selected.iter().all(|t| e.patched.contains(&t.path)) {
                t("emulator.reapply")
            } else if e.av_blocked {
                t("emulator.avBlockedRetry")
            } else {
                t("emulator.apply")
            };
            specs.push(
                ButtonSpec::new(label, Fid::new("emu.apply"), Action::EmuApply, Btn::Primary)
                    .enabled(!e.busy && (!selected.is_empty() || e.apply_complete)),
            );
        }
        widgets::buttons(buf, ctx, footer, &specs, true);
        ctx.prefer(Fid::new(if e.edit_mode { "emu.save" } else { "emu.apply" }));

        let mut sv = ScrollView::begin(ctx, rest, 200, e.scroll);
        let full_w = sv.width();
        let x0 = sv.x();
        let two_col = full_w >= 110;
        let (lw, rw) = if two_col {
            (full_w * 52 / 100, full_w - full_w * 52 / 100 - 3)
        } else {
            (full_w, full_w)
        };

        let (title, desc) = if e.edit_mode {
            (t("emulator.editTitle"), t("emulator.editDescription"))
        } else {
            (t("emulator.title"), t("emulator.description"))
        };
        let r = sv.next(1);
        widgets::heading(&mut sv.buf, r, &title);
        let h = widgets::wrap_height(&desc, full_w);
        let r = sv.next(h);
        widgets::paragraph(&mut sv.buf, r, &desc, theme::dim());
        if !e.edit_mode {
            let (tone, msg) = match &e.release {
                None => (Tone::Busy, t("emulator.releaseLoading")),
                Some(Ok(info)) => (
                    Tone::Success,
                    tf("emulator.releaseReady", &[("tag", &info.tag)]),
                ),
                Some(Err(err)) => (Tone::Warning, err.clone()),
            };
            let r = sv.next(1);
            widgets::status(&mut sv.buf, r, tone, &msg, tick);
        }
        if !e.edit_mode && patched_count > 0 {
            let msg = if patched_count == e.scan.len() {
                t("emulator.patchedNoticeAll")
            } else {
                tf(
                    "emulator.patchedNoticeSome",
                    &[("patched", &patched_count), ("total", &e.scan.len())],
                )
            };
            let h = widgets::status_height(&msg, full_w);
            let r = sv.next(h);
            widgets::status(&mut sv.buf, r, Tone::Info, &msg, tick);
        }
        sv.gap(1);
        let top = sv.used();

        let mut col = Col {
            x: x0,
            w: lw,
            y: top,
        };
        if let Some(info) = e.patch_info.as_ref() {
            let variant = match info.variant.as_deref() {
                Some("experimental") => format!(" ({})", t("emulator.variantExperimental")),
                Some("regular") => format!(" ({})", t("emulator.variantRegular")),
                _ => String::new(),
            };
            let text = match &info.tag {
                Some(tag) => tf("emulator.patchedWith", &[("tag", tag), ("variant", &variant)]),
                None => t("emulator.patchedWithUnknown"),
            };
            let h = widgets::wrap_height(&text, lw);
            let r = col.next(h);
            widgets::paragraph(&mut sv.buf, r, &text, theme::dim());
            let latest = match &e.release {
                Some(Ok(rel)) => Some(rel.tag.clone()),
                _ => None,
            };
            if let Some(latest) = latest.filter(|l| info.tag.as_deref() != Some(l.as_str())) {
                let r = col.next(1);
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    r.x,
                    r.y,
                    r.width,
                    &ButtonSpec::new(
                        tf("emulator.updateGbe", &[("tag", &latest)]),
                        Fid::new("emu.updateGbe"),
                        Action::EmuUpdateGbe,
                        Btn::Primary,
                    )
                    .enabled(!e.busy),
                );
            }
            col.gap(1);
        }
        let r = col.next(1);
        widgets::heading(&mut sv.buf, r, &t("emulator.detectedLabel"));
        if e.scan.is_empty() {
            let r = col.next(1);
            widgets::text(&mut sv.buf, r, &t("emulator.noFiles"), theme::muted());
        } else {
            if !e.selection_explicit && e.selected.is_empty() {
                e.selected = default_selection(&e.scan);
                e.selection_explicit = true;
            }
            let dir = e.game_dir.clone();
            for (i, f) in e.scan.iter().enumerate() {
                let r = col.next(1);
                let rel = f
                    .path
                    .strip_prefix(&dir)
                    .unwrap_or(&f.path)
                    .trim_start_matches(['/', '\\'])
                    .to_string();
                let rel = match &f.loader {
                    Some(exe) => tf(
                        "emulator.method.loaderRow",
                        &[("exe", &exe.strip_prefix(&dir).unwrap_or(exe).trim_start_matches(['/', '\\']).to_string())],
                    ),
                    None => rel,
                };
                let checked = e.selected.contains(&f.path);
                let mut tags = vec![
                    widgets::badge(
                        &t(if f.platform == Platform::Linux {
                            "emulator.platformLinux"
                        } else {
                            "emulator.platformWindows"
                        }),
                        theme::get().accent,
                    ),
                    Span::raw(" "),
                    widgets::badge(
                        &t(if f.arch == "x64" {
                            "emulator.archX64"
                        } else {
                            "emulator.archX32"
                        }),
                        theme::get().text_dim,
                    ),
                ];
                if e.patched.contains(&f.path) {
                    tags.push(Span::raw(" "));
                    tags.push(widgets::badge(
                        &t("emulator.patchedBadge"),
                        theme::get().success,
                    ));
                }
                let tags_w: u16 = tags.iter().map(|s| widgets::width(&s.content)).sum();
                let cb_w = r.width.saturating_sub(tags_w + 1);
                widgets::checkbox(
                    &mut sv.buf,
                    ctx,
                    Rect::new(r.x, r.y, cb_w, 1),
                    checked,
                    &widgets::truncate_left(&rel, cb_w.saturating_sub(4)),
                    Fid::idx("emu.file", i),
                    Action::EmuToggleFile(i),
                    !e.edit_mode,
                );
                widgets::line(
                    &mut sv.buf,
                    Rect::new(r.right().saturating_sub(tags_w), r.y, tags_w, 1),
                    Line::from(tags),
                );
            }
            if e.scan.len() > 1 && !e.edit_mode {
                let r = col.next(1);
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    r.x,
                    r.y,
                    r.width,
                    &ButtonSpec::new(
                        t("tui.emu.toggleAll"),
                        Fid::new("emu.allFiles"),
                        Action::EmuSelectAllFiles,
                        Btn::Secondary,
                    ),
                );
            }
        }

        if !e.drm.is_empty() || e.drm_status.is_some() {
            col.gap(1);
            let r = col.next(1);
            widgets::heading(&mut sv.buf, r, &t("emulator.drmTitle"));
            if !e.drm.is_empty() {
                let d = t("emulator.drmDescription");
                let h = widgets::wrap_height(&d, lw);
                let r = col.next(h);
                widgets::paragraph(&mut sv.buf, r, &d, theme::dim());
                let dir = e.game_dir.clone();
                for d in &e.drm {
                    let r = col.next(1);
                    let rel = d
                        .path
                        .strip_prefix(&dir)
                        .unwrap_or(&d.path)
                        .trim_start_matches(['/', '\\'])
                        .to_string();
                    let size = widgets::fmt_bytes(d.size_bytes);
                    widgets::text(
                        &mut sv.buf,
                        Rect::new(
                            r.x + 2,
                            r.y,
                            r.width.saturating_sub(widgets::width(&size) + 3),
                            1,
                        ),
                        &rel,
                        theme::warning(),
                    );
                    widgets::text_right(&mut sv.buf, r, &size, theme::muted());
                }
                let r = col.next(1);
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    r.x,
                    r.y,
                    r.width,
                    &ButtonSpec::new(
                        t("emulator.drmRemove"),
                        Fid::new("emu.drm"),
                        Action::EmuRemoveDrm,
                        Btn::Primary,
                    )
                    .enabled(!e.drm_busy),
                );
                let r = col.next(1);
                widgets::text(
                    &mut sv.buf,
                    r,
                    &t("emulator.drmAttribution"),
                    theme::muted(),
                );
            }
            if let Some((tone, msg)) = &e.drm_status {
                let h = widgets::status_height(msg, lw).min(10);
                let r = col.next(h);
                widgets::status(&mut sv.buf, r, *tone, msg, tick);
            }
        }

        if !e.drm_backups.is_empty() {
            col.gap(1);
            let r = col.next(1);
            widgets::heading(&mut sv.buf, r, &t("emulator.drmRestore.title"));
            let d = t("emulator.drmRestore.hint");
            let h = widgets::wrap_height(&d, lw);
            let r = col.next(h);
            widgets::paragraph(&mut sv.buf, r, &d, theme::dim());
            let dir = e.game_dir.clone();
            let label = t("emulator.drmRestore.restore");
            let bw = widgets::width(&label) + 4;
            for (i, b) in e.drm_backups.iter().enumerate() {
                let r = col.next(1);
                let rel = b.path.strip_prefix(&dir).unwrap_or(&b.path).trim_start_matches(['/', '\\']).to_string();
                widgets::text(&mut sv.buf, Rect::new(r.x + 2, r.y, r.width.saturating_sub(bw + 3), 1), &rel, theme::text());
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    r.x + r.width.saturating_sub(bw),
                    r.y,
                    bw,
                    &ButtonSpec::new(label.clone(), Fid::idx("emu.drmRestore", i), Action::EmuRestoreDrm(i), Btn::Secondary),
                );
            }
        }

        if e.merge_plan.is_some() || e.merge_status.is_some() {
            col.gap(1);
            let r = col.next(1);
            widgets::heading(&mut sv.buf, r, &t("emulator.dlcMergeLabel"));
            if let Some(plan) = &e.merge_plan {
                let main = match &plan.main_label {
                    Some(l) => format!("{} ({})", l, plan.main_depot_id),
                    None => plan.main_depot_id.clone(),
                };
                let d = tf(
                    "emulator.dlcMergeHintDetail",
                    &[("count", &plan.to_merge.len()), ("main", &main)],
                );
                let h = widgets::wrap_height(&d, lw);
                let r = col.next(h);
                widgets::paragraph(&mut sv.buf, r, &d, theme::dim());
                for c in &plan.to_merge {
                    let r = col.next(1);
                    let label = c
                        .label
                        .clone()
                        .map(|l| format!(" — {}", l))
                        .unwrap_or_default();
                    widgets::line(
                        &mut sv.buf,
                        r,
                        Line::from(vec![
                            Span::styled(
                                format!("  • {}", c.depot_id),
                                theme::text().add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(label, theme::dim()),
                            Span::raw(" "),
                            widgets::badge(&role_label(&c.role), theme::get().purple),
                        ]),
                    );
                }
                if !plan.skipped.is_empty() {
                    let note = t("emulator.dlcMergeSkippedNote");
                    let h = widgets::wrap_height(&note, lw);
                    let r = col.next(h);
                    hint(&mut sv.buf, r, &note);
                    for c in &plan.skipped {
                        let r = col.next(1);
                        widgets::text(
                            &mut sv.buf,
                            r,
                            &format!(
                                "  • {} ({}, {})",
                                c.depot_id,
                                role_label(&c.role),
                                t("emulator.dlcMergeSkippedTag")
                            ),
                            theme::muted(),
                        );
                    }
                }
                let r = col.next(1);
                widgets::button(
                    &mut sv.buf,
                    ctx,
                    r.x,
                    r.y,
                    r.width,
                    &ButtonSpec::new(
                        t("emulator.dlcMergeButton"),
                        Fid::new("emu.merge"),
                        Action::EmuMergeDlc,
                        Btn::Primary,
                    )
                    .enabled(!e.merge_busy),
                );
            }
            if let Some((tone, msg)) = &e.merge_status {
                let h = widgets::status_height(msg, lw);
                let r = col.next(h);
                widgets::status(&mut sv.buf, r, *tone, msg, tick);
            }
        }

        let plain_windows = e.scan.iter().any(|f| f.platform == Platform::Windows && f.loader.is_none());
        if !e.edit_mode && plain_windows {
            col.gap(1);
            let r = col.next(1);
            widgets::heading(&mut sv.buf, r, &t("emulator.method.label"));
            let r = col.next(1);
            widgets::radio(&mut sv.buf, ctx, r, !e.cold, &format!("{} — {}", t("emulator.method.replace"), t("emulator.method.replaceHint")), Fid::new("emu.methodReplace"), Action::EmuMethod(false));
            let r = col.next(1);
            if e.cold_exes.is_empty() {
                widgets::text(&mut sv.buf, r, &format!("  {} — {}", t("emulator.method.coldclient"), t("emulator.method.coldclientHint")), theme::muted());
            } else {
                widgets::radio(&mut sv.buf, ctx, r, e.cold, &format!("{} — {}", t("emulator.method.coldclient"), t("emulator.method.coldclientHint")), Fid::new("emu.methodCold"), Action::EmuMethod(true));
            }
            if e.cold {
                let r = col.next(1);
                widgets::text(&mut sv.buf, Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), 1), &t("emulator.method.exe"), theme::text());
                let dir = e.game_dir.clone();
                for (i, exe) in e.cold_exes.iter().enumerate().take(12) {
                    let r = col.next(1);
                    let rel = exe.strip_prefix(&dir).unwrap_or(exe).trim_start_matches(['/', '\\']).to_string();
                    widgets::radio(&mut sv.buf, ctx, Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), 1), e.cold_idx == i, &rel, Fid::idx("emu.coldExe", i), Action::EmuColdExe(i));
                }
                let h = widgets::wrap_height(&t("emulator.method.exeHint"), lw.saturating_sub(4));
                let r = col.next(h);
                hint(&mut sv.buf, Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height), &t("emulator.method.exeHint"));
            }
        }
        if e.cold_steam_running {
            col.gap(1);
            let msg = t("emulator.method.steamRunning");
            let h = widgets::wrap_height(&msg, lw);
            let r = col.next(h);
            widgets::paragraph(&mut sv.buf, r, &msg, theme::warning());
            let r = col.next(1);
            widgets::button(&mut sv.buf, ctx, r.x, r.y, r.width, &ButtonSpec::new(t("emulator.method.closeSteam"), Fid::new("emu.coldSteam"), Action::EmuColdSteam, Btn::Primary));
        }
        if !e.edit_mode && !e.cold {
            col.gap(1);
            let r = col.next(1);
            widgets::heading(&mut sv.buf, r, &t("emulator.variantLabel"));
            let r = col.next(1);
            widgets::radio(
                &mut sv.buf,
                ctx,
                r,
                !e.experimental,
                &format!(
                    "{} — {}",
                    t("emulator.variantRegular"),
                    t("emulator.variantRegularHint")
                ),
                Fid::new("emu.regular"),
                Action::EmuVariant(false),
            );
            let r = col.next(1);
            widgets::radio(
                &mut sv.buf,
                ctx,
                r,
                e.experimental,
                &format!(
                    "{} — {}",
                    t("emulator.variantExperimental"),
                    t("emulator.variantExperimentalHint")
                ),
                Fid::new("emu.experimental"),
                Action::EmuVariant(true),
            );
        }
        col.gap(1);
        let has_windows = e.scan.iter().any(|f| f.platform == Platform::Windows);
        let r = col.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r,
            e.bypass && has_windows,
            &t("emulator.bypassLabel"),
            Fid::new("emu.bypass"),
            Action::EmuToggleBypass,
            has_windows,
        );
        let bh = if has_windows {
            t("emulator.bypassHint")
        } else {
            t("emulator.bypassLinuxNote")
        };
        let h = widgets::wrap_height(&bh, lw.saturating_sub(4));
        let r = col.next(h);
        hint(
            &mut sv.buf,
            Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
            &bh,
        );
        col.gap(1);
        let game_data_on = !self.prefs.emu_game_data_off;
        let r = col.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r,
            game_data_on,
            &t("emulator.gameData.label"),
            Fid::new("emu.gameData"),
            Action::EmuToggleGameData,
            true,
        );
        let gh = t("emulator.gameData.hint");
        let h = widgets::wrap_height(&gh, lw.saturating_sub(4));
        let r = col.next(h);
        hint(
            &mut sv.buf,
            Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
            &gh,
        );
        let r = col.next(1);
        widgets::button(
            &mut sv.buf,
            ctx,
            r.x + 4,
            r.y,
            r.width.saturating_sub(4),
            &ButtonSpec::new(
                t("emulator.gameData.generate"),
                Fid::new("emu.gameDataNow"),
                Action::EmuWriteGameData,
                Btn::Secondary,
            ),
        );
        let left_end = col.y;

        let mut col = if two_col {
            Col {
                x: x0 + lw + 3,
                w: rw,
                y: top,
            }
        } else {
            Col {
                x: x0,
                w: lw,
                y: left_end + 1,
            }
        };
        let r = col.next(1);
        widgets::heading(
            &mut sv.buf,
            r,
            &format!(
                "{} ({})",
                t("emulator.settingsLabel"),
                t("emulator.settingsOptional")
            ),
        );
        let hint_text = if !e.edit_targets.is_empty() {
            t("emulator.settingsHintPatched")
        } else if e.standalone {
            t("emulator.settingsHintStandalone")
        } else {
            t("emulator.settingsHint")
        };
        let h = widgets::wrap_height(&hint_text, col.w);
        let r = col.next(h);
        hint(&mut sv.buf, r, &hint_text);
        col.gap(1);
        let watcher = t("tui.emu.achievementWatcher");
        let h = widgets::wrap_height(&watcher, col.w);
        let r = col.next(h);
        hint(&mut sv.buf, r, &watcher);
        col.gap(1);
        let query = e.settings_search.trimmed().to_lowercase();
        let changed_only = e.settings_changed_only;
        let filtering = !query.is_empty() || changed_only;
        let r = col.next(3);
        let search_label = t("emulator.settingsSearch");
        widgets::input(
            &mut sv.buf,
            ctx,
            r,
            &mut e.settings_search,
            InputSpec {
                label: &search_label,
                placeholder: "",
                fid: Fid::new("emu.settingsSearch"),
                id: InputId::EmuSettingsSearch,
                error: false,
            },
        );
        let r = col.next(1);
        widgets::checkbox(
            &mut sv.buf,
            ctx,
            r,
            changed_only,
            &t("emulator.settingsOnlyChanged"),
            Fid::new("emu.settingsChanged"),
            Action::EmuSettingsChangedOnly,
            true,
        );
        let r = col.next(1);
        widgets::button(
            &mut sv.buf,
            ctx,
            r.x,
            r.y,
            r.width,
            &ButtonSpec::new(
                t("emulator.settingsReset"),
                Fid::new("emu.settingsReset"),
                Action::EmuSettingsReset,
                Btn::Secondary,
            ),
        );
        col.gap(1);
        let has_target = e.extra_target().is_some();
        let patched = e.game_patched();
        if e.section >= super::emu_extras::EXTRA_SAVES && (!has_target || (e.section == super::emu_extras::EXTRA_SAVES && !patched)) {
            e.section = 0;
        }
        let tabs: Vec<ButtonSpec> = SECTIONS
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < super::emu_extras::EXTRA_SAVES || (has_target && (*i != super::emu_extras::EXTRA_SAVES || patched)))
            .map(|(i, key)| {
                let kind = if e.section == i && !filtering {
                    Btn::Primary
                } else {
                    Btn::Secondary
                };
                let count = e.changed_in_section(i);
                let label = if count > 0 { format!("{} ({})", t(key), count) } else { t(key) };
                ButtonSpec::new(
                    label,
                    Fid::idx("emu.section", i),
                    Action::EmuSection(i),
                    kind,
                )
            })
            .collect();
        let rows = widgets::button_rows(&tabs, col.w);
        let r = col.next(rows);
        widgets::buttons_wrapped(&mut sv.buf, ctx, r, &tabs);
        col.gap(1);
        let mut shown = 0usize;
        for (i, f) in TEXT_FIELDS.iter().enumerate() {
            let changed = !e.fields[i].trimmed().is_empty();
            if !e.setting_visible(f.section, f.key, f.label, changed, &query) {
                continue;
            }
            shown += 1;
            let r = col.next(3);
            let label = t(&field_text(f.label));
            widgets::input(
                &mut sv.buf,
                ctx,
                r,
                &mut e.fields[i],
                InputSpec {
                    label: &label,
                    placeholder: &placeholder(f.placeholder),
                    fid: Fid::idx("emu.field", i),
                    id: InputId::EmuField(i),
                    error: false,
                },
            );
            let hk = t(&format!("{}Hint", field_text(f.label)));
            let h = widgets::wrap_height(&hk, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &hk);
        }
        if e.setting_visible(SYSTEM_SECTION, "steam_hardware_type", "steamHardwareType", !e.hardware_type.is_empty(), &query) {
            shown += 1;
            let r = col.next(1);
            widgets::paragraph(&mut sv.buf, r, &t("emulator.fields.steamHardwareType"), theme::text());
            for (i, (value, label)) in HARDWARE_TYPES.iter().enumerate() {
                let r = col.next(1);
                widgets::radio(
                    &mut sv.buf,
                    ctx,
                    r,
                    e.hardware_type == *value,
                    &t(&format!("emulator.fields.{}", label)),
                    Fid::idx("emu.hardware", i),
                    Action::EmuHardwareType(i),
                );
            }
            let hk = t("emulator.fields.steamHardwareTypeHint");
            let h = widgets::wrap_height(&hk, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &hk);
            col.gap(1);
        }
        if !filtering && e.section >= super::emu_extras::EXTRA_SAVES {
            render_extras(e, &mut sv.buf, ctx, &mut col, tick);
        }
        let show_controller = !filtering && e.section == CONTROLLER_SECTION;
        if show_controller {
            let r = col.next(1);
            widgets::checkbox(
                &mut sv.buf,
                ctx,
                r,
                e.controller,
                &t("emulator.controller.label"),
                Fid::new("emu.controller"),
                Action::EmuToggleController,
                true,
            );
            let ch = crate::i18n::strip_html(&t("emulator.controller.hint"));
            let h = widgets::wrap_height(&ch, col.w.saturating_sub(4));
            let r = col.next(h);
            hint(&mut sv.buf, Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height), &ch);
            col.gap(1);
            let status_key = if e.controller_checking {
                "checking"
            } else {
                match &e.controller_info {
                    Some(i) if i.layout => "statusLayout",
                    Some(i) if matches!(i.support.as_deref(), Some("full") | Some("partial")) => "statusSupport",
                    Some(_) => "statusNone",
                    None => "statusUnknown",
                }
            };
            let st = t(&format!("emulator.controller.{}", status_key));
            let h = widgets::wrap_height(&st, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &st);
            if e.controller_needs_experimental() {
                let (text, style) = if e.experimental {
                    (t("emulator.controller.experimentalNote"), theme::dim())
                } else {
                    (t("emulator.controller.regularWarning"), theme::warning())
                };
                let h = widgets::wrap_height(&text, col.w);
                let r = col.next(h);
                widgets::paragraph(&mut sv.buf, r, &text, style);
            }
            col.gap(1);
            let r = col.next(3);
            let label = t("emulator.controller.layoutFile");
            widgets::input(
                &mut sv.buf,
                ctx,
                r,
                &mut e.controller_file,
                InputSpec {
                    label: &label,
                    placeholder: &t("emulator.controller.noFile"),
                    fid: Fid::new("emu.controllerFile"),
                    id: InputId::EmuControllerFile,
                    error: false,
                },
            );
            let lh = crate::i18n::strip_html(&t("emulator.controller.layoutFileHint"));
            let h = widgets::wrap_height(&lh, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &lh);
            col.gap(1);
            let r = col.next(1);
            widgets::paragraph(&mut sv.buf, r, &t("emulator.fields.controllerType"), theme::text());
            for (i, (value, name)) in CONTROLLER_TYPES.iter().enumerate() {
                let r = col.next(1);
                widgets::radio(
                    &mut sv.buf,
                    ctx,
                    r,
                    e.controller_type.eq_ignore_ascii_case(value),
                    &placeholder(name),
                    Fid::idx("emu.controllerType", i),
                    Action::EmuControllerType(i),
                );
            }
            let th = t("emulator.fields.controllerTypeHint");
            let h = widgets::wrap_height(&th, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &th);
            let xh = t("emulator.controller.xinputHint");
            let h = widgets::wrap_height(&xh, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &xh);
            col.gap(1);
        }
        for (i, f) in BOOL_FIELDS.iter().enumerate() {
            let on = e.bools.get(f.key).copied().unwrap_or(false);
            if !e.setting_visible(f.section, f.key, f.label, on, &query) {
                continue;
            }
            shown += 1;
            let r = col.next(1);
            widgets::checkbox(
                &mut sv.buf,
                ctx,
                r,
                on,
                &t(&field_text(f.label)),
                Fid::idx("emu.bool", i),
                Action::EmuToggleField(i),
                true,
            );
            let hk = t(&format!("{}Hint", field_text(f.label)));
            let h = widgets::wrap_height(&hk, col.w.saturating_sub(4));
            let r = col.next(h);
            hint(
                &mut sv.buf,
                Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
                &hk,
            );
        }
        if filtering && shown == 0 {
            let r = col.next(1);
            hint(&mut sv.buf, r, &t("emulator.settingsNoMatch"));
        }
        let right_end = col.y;
        sv.advance_to(left_end.max(right_end));

        let mut scroll = e.scroll;
        sv.finish(buf, ctx, &mut scroll, ScrollTarget::Page);
        if let Some(e) = self.emu.as_mut() {
            e.scroll = scroll;
        }
    }
}

fn extra_button(buf: &mut Buffer, ctx: &mut Ctx, col: &mut Col, label: String, fid: Fid, action: Action) {
    let r = col.next(1);
    let w = (widgets::width(&label) + 4).min(r.width);
    widgets::button(buf, ctx, r.x, r.y, w, &ButtonSpec::new(label, fid, action, Btn::Secondary));
}

fn extra_buttons(buf: &mut Buffer, ctx: &mut Ctx, col: &mut Col, specs: Vec<ButtonSpec>) {
    let rows = widgets::button_rows(&specs, col.w);
    let r = col.next(rows);
    widgets::buttons_wrapped(buf, ctx, r, &specs);
}

fn extra_hint(buf: &mut Buffer, col: &mut Col, text: &str) {
    let text = crate::i18n::strip_html(text);
    let h = widgets::wrap_height(&text, col.w);
    let r = col.next(h);
    hint(buf, r, &text);
}

fn extra_input(buf: &mut Buffer, ctx: &mut Ctx, col: &mut Col, input: &mut crate::ui::input::TextInput, label: &str, placeholder: &str, fid: Fid, id: InputId) {
    let r = col.next(3);
    widgets::input(buf, ctx, r, input, InputSpec { label, placeholder, fid, id, error: false });
}

fn render_extras(e: &mut EmuState, buf: &mut Buffer, ctx: &mut Ctx, col: &mut Col, tick: u64) {
    use super::emu_extras::{EXTRA_MODS, EXTRA_NETWORK, EXTRA_PROFILE, EXTRA_SAVES};
    let x = &mut e.extras;
    match e.section {
        EXTRA_SAVES => {
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.saves.folder"));
            if x.saves.is_empty() {
                extra_hint(buf, col, &t("emulator.saves.none"));
            }
            for (i, f) in x.saves.iter().enumerate() {
                let meta = if f.exists {
                    tf("emulator.saves.meta", &[("files", &f.files), ("size", &widgets::fmt_bytes(f.size_bytes)), ("date", &f.modified.and_then(|m| chrono::DateTime::from_timestamp(m as i64, 0)).map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "-".into()))])
                } else {
                    t("emulator.saves.empty")
                };
                let place = t(&format!("emulator.saves.place.{}", if f.portable { "portable" } else { f.place.as_str() }));
                let r = col.next(1);
                widgets::radio(buf, ctx, r, x.save_idx == i, &format!("{} [{}]", f.path, place), Fid::idx("emu.save", i), Action::EmuSaveSelect(i));
                let r = col.next(1);
                widgets::text(buf, Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), 1), &meta, theme::muted());
            }
            extra_hint(buf, col, &t("emulator.saves.hint"));
            let zip_label = t("tui.emu.extras.zipPath");
            extra_input(buf, ctx, col, &mut x.zip_path, &zip_label, "~/saves-backup.zip", Fid::new("emu.extraZip"), InputId::EmuExtraZip);
            let exists = x.saves.get(x.save_idx).is_some_and(|f| f.exists);
            extra_buttons(buf, ctx, col, vec![
                ButtonSpec::new(t("emulator.saves.open"), Fid::new("emu.savesOpen"), Action::EmuSavesOpen, Btn::Secondary).enabled(exists),
                ButtonSpec::new(t("tui.emu.extras.backup"), Fid::new("emu.savesBackup"), Action::EmuSavesBackup, Btn::Secondary).enabled(exists),
                ButtonSpec::new(t("tui.emu.extras.restore"), Fid::new("emu.savesRestore"), Action::EmuSavesRestore, Btn::Secondary).enabled(!x.saves.is_empty()),
            ]);
            col.gap(1);
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.achievements.title"));
            let rows = x.ach.as_ref().map(|a| a.rows.clone()).unwrap_or_default();
            let stats = x.ach.as_ref().map(|a| a.stats_files).unwrap_or(0);
            if rows.is_empty() {
                extra_hint(buf, col, &t("emulator.achievements.none"));
            } else {
                let earned = rows.iter().filter(|r| r.earned).count();
                extra_hint(buf, col, &tf("emulator.achievements.count", &[("earned", &earned), ("total", &rows.len())]));
                for a in rows.iter().take(200) {
                    let r = col.next(1);
                    let mark = if a.earned { "✓" } else { "·" };
                    let name = if a.hidden && !a.earned { t("emulator.achievements.hidden") } else { a.display_name.clone() };
                    widgets::text(buf, r, &format!("{} {}", crate::ui::glyphs::text(mark), name), if a.earned { theme::success() } else { theme::dim() });
                }
            }
            extra_buttons(buf, ctx, col, vec![
                ButtonSpec::new(t("emulator.achievements.resetAll"), Fid::new("emu.achReset"), Action::EmuAchResetAll, Btn::Secondary).enabled(rows.iter().any(|r| r.earned)),
                ButtonSpec::new(t("emulator.achievements.resetStats"), Fid::new("emu.statsReset"), Action::EmuStatsReset, Btn::Secondary).enabled(stats > 0),
            ]);
        }
        EXTRA_PROFILE => {
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.profile.title"));
            let file_label = t("tui.emu.extras.filePath");
            extra_input(buf, ctx, col, &mut x.asset_path, &file_label, "~/picture.png", Fid::new("emu.extraAsset"), InputId::EmuExtraAsset);
            extra_hint(buf, col, &t("tui.emu.extras.fileHint"));
            let rows: [(&'static str, String, Option<String>, String); 4] = [
                ("avatar", t("emulator.profile.avatar"), x.assets.avatar.clone(), t("emulator.profile.avatarHint")),
                ("font", t("emulator.profile.font"), x.assets.font.clone(), t("emulator.profile.overlayHint")),
                ("achievement", t("emulator.profile.achievementSound"), x.assets.achievement_sound.as_ref().map(|_| t("emulator.profile.ownSound")), String::new()),
                ("friend", t("emulator.profile.friendSound"), x.assets.friend_sound.as_ref().map(|_| t("emulator.profile.ownSound")), String::new()),
            ];
            for (ri, (kind, label, value, hint_text)) in rows.into_iter().enumerate() {
                col.gap(1);
                let r = col.next(1);
                let current = value.clone().unwrap_or_else(|| {
                    if kind == "font" { t("emulator.profile.defaultFont") } else if kind == "avatar" { "-".into() } else { t("emulator.profile.noSound") }
                });
                widgets::text(buf, r, &format!("{}: {}", label, current), theme::text());
                if !hint_text.is_empty() {
                    extra_hint(buf, col, &hint_text);
                }
                let mut specs = vec![ButtonSpec::new(t("tui.emu.extras.useFile"), Fid::idx("emu.assetSet", ri), Action::EmuAssetSet(kind), Btn::Secondary)];
                if value.is_some() {
                    specs.push(ButtonSpec::new(t("emulator.profile.remove"), Fid::idx("emu.assetClear", ri), Action::EmuAssetClear(kind), Btn::Secondary));
                }
                extra_buttons(buf, ctx, col, specs);
            }
        }
        EXTRA_NETWORK => {
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.network.title"));
            let label = t("emulator.network.broadcasts");
            extra_input(buf, ctx, col, &mut x.broadcasts, &label, "100.64.0.2, 100.64.0.3", Fid::new("emu.broadcasts"), InputId::EmuBroadcasts);
            extra_hint(buf, col, &t("emulator.network.broadcastsHint"));
            extra_button(buf, ctx, col, t("emulator.network.save"), Fid::new("emu.broadcastsSave"), Action::EmuBroadcastsSave);
            col.gap(1);
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.network.lobbyTitle"));
            if e.edit_mode || e.apply_complete || e.scan.iter().any(|f| f.is_patched) {
                extra_hint(buf, col, &t("emulator.network.lobbyHint"));
                extra_button(buf, ctx, col, t("emulator.network.lobbyStart"), Fid::new("emu.lobby"), Action::EmuLobbyConnect(false));
            } else {
                extra_hint(buf, col, &t("emulator.network.lobbyLater"));
            }
        }
        EXTRA_MODS => {
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.mods.title"));
            if x.mods.is_empty() {
                extra_hint(buf, col, &t("emulator.mods.empty"));
            }
            let label = t("emulator.mods.remove");
            let bw = widgets::width(&label) + 4;
            for (i, m) in x.mods.iter().enumerate() {
                let r = col.next(1);
                let meta = tf("emulator.mods.meta", &[("id", &m.id), ("files", &m.files), ("size", &widgets::fmt_bytes(m.size_bytes))]);
                widgets::text(buf, Rect::new(r.x, r.y, r.width.saturating_sub(bw + 2), 1), &format!("{} · {}", m.title, meta), theme::text());
                widgets::button(buf, ctx, r.x + r.width.saturating_sub(bw), r.y, bw, &ButtonSpec::new(label.clone(), Fid::idx("emu.modRemove", i), Action::EmuModRemove(i), Btn::Secondary));
            }
            col.gap(1);
            let r = col.next(1);
            widgets::heading(buf, r, &t("emulator.mods.add"));
            let id_label = t("emulator.mods.idPlaceholder");
            extra_input(buf, ctx, col, &mut x.mod_id, &id_label, "123456789", Fid::new("emu.modId"), InputId::EmuModId);
            let path_label = t("tui.emu.extras.modPath");
            extra_input(buf, ctx, col, &mut x.mod_path, &path_label, "~/mods/my_mod", Fid::new("emu.modPath"), InputId::EmuModPath);
            extra_hint(buf, col, &t("emulator.mods.hint"));
            extra_button(buf, ctx, col, t("tui.emu.extras.addMod"), Fid::new("emu.modAdd"), Action::EmuModAdd);
        }
        _ => {}
    }
    if let Some((tone, msg)) = &x.status {
        col.gap(1);
        let h = widgets::status_height(msg, col.w).min(8);
        let r = col.next(h);
        widgets::status(buf, r, *tone, msg, tick);
    }
    col.gap(1);
}

struct Col {
    x: u16,
    w: u16,
    y: u16,
}

impl Col {
    fn next(&mut self, h: u16) -> Rect {
        let r = Rect::new(self.x, self.y, self.w, h);
        self.y += h;
        r
    }

    fn gap(&mut self, h: u16) {
        self.y += h;
    }
}

fn default_selection(scan: &[ScannedFile]) -> std::collections::HashSet<String> {
    let depots: std::collections::HashSet<String> = scan
        .iter()
        .map(|f| depot_folder(&f.path))
        .filter(|d| !d.is_empty())
        .collect();
    if depots.len() > 1 {
        let best = pick_default_depot(scan);
        scan.iter()
            .filter(|f| Some(depot_folder(&f.path)) == best)
            .map(|f| f.path.clone())
            .collect()
    } else {
        scan.iter().map(|f| f.path.clone()).collect()
    }
}

fn role_label(role: &str) -> String {
    let key = format!("depots.role_{}", role);
    let s = t(&key);
    if s == key {
        role.to_string()
    } else {
        s
    }
}

fn emu_error_hint(err: &str) -> String {
    let lower = err.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    let key = if has(&[
        "os error 225",
        "virus",
        "unwanted software",
        "unerwünschte software",
    ]) {
        "emulator.hintAntivirusFile"
    } else if has(&[
        "os error 5)",
        "os error 13)",
        "access is denied",
        "zugriff verweigert",
        "permission denied",
    ]) {
        "emulator.hintPermission"
    } else if has(&[
        "github fetch failed",
        "github returned http",
        "download returned http",
        "error sending request",
        "steamless download",
        "not present in release",
    ]) {
        "emulator.hintDownload"
    } else {
        return String::new();
    };
    format!("\n\n{}", t(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emu_error_hints_match_common_failures() {
        crate::i18n::set_language("en");
        assert!(
            emu_error_hint("copy emulator binary: Access is denied. (os error 5)")
                .contains("protected place")
        );
        assert!(emu_error_hint("Operation did not complete successfully because the file contains a virus (os error 225)")
            .contains("antivirus"));
        assert!(
            emu_error_hint("GitHub fetch failed: error sending request for url").contains("GitHub")
        );
        assert!(emu_error_hint("generate_interfaces failed: bad file").is_empty());
    }

    fn file(path: &str, platform: Platform, arch: &str) -> ScannedFile {
        ScannedFile {
            path: path.into(),
            platform,
            arch: arch.into(),
            is_patched: false,
            loader: None,
        }
    }

    #[test]
    fn extracts_depot_folder() {
        assert_eq!(
            depot_folder("/dl/220 - HL2/depots/221 - Content/bin/steam_api.dll"),
            "221 - Content"
        );
        assert_eq!(depot_folder("C:\\dl\\depots\\5\\steam_api64.dll"), "5");
        assert_eq!(depot_folder("/games/x/steam_api.dll"), "");
    }

    #[test]
    fn prefers_host_platform_depot() {
        let files = vec![
            file("/d/depots/1/steam_api.dll", Platform::Windows, "x32"),
            file("/d/depots/2/libsteam_api.so", Platform::Linux, "x64"),
            file("/d/depots/3/steam_api64.dll", Platform::Windows, "x64"),
        ];
        let best = pick_default_depot(&files).unwrap();
        if cfg!(target_os = "linux") {
            assert_eq!(best, "2");
        } else {
            assert_eq!(best, "3");
        }
    }

    #[test]
    fn single_depot_selects_everything() {
        let files = vec![
            file("/d/depots/1/steam_api.dll", Platform::Windows, "x32"),
            file("/d/depots/1/steam_api64.dll", Platform::Windows, "x64"),
        ];
        assert_eq!(default_selection(&files).len(), 2);
    }
}
