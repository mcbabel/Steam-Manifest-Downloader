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

pub struct TextField {
    pub key: &'static str,
    pub label: &'static str,
    pub placeholder: &'static str,
    pub section: usize,
    pub float: bool,
}

pub struct BoolField {
    pub key: &'static str,
    pub label: &'static str,
    pub section: usize,
}

pub const SECTIONS: [&str; 5] = [
    "emulator.sections.account",
    "emulator.sections.saves",
    "emulator.sections.network",
    "emulator.sections.stats",
    "emulator.sections.overlay",
];

pub const TEXT_FIELDS: &[TextField] = &[
    TextField {
        key: "account_name",
        label: "accountName",
        placeholder: "@emulator.fields.accountNamePlaceholder",
        section: 0,
        float: false,
    },
    TextField {
        key: "account_steamid",
        label: "accountSteamId",
        placeholder: "76561198000000000",
        section: 0,
        float: false,
    },
    TextField {
        key: "language",
        label: "language",
        placeholder: "english",
        section: 0,
        float: false,
    },
    TextField {
        key: "ip_country",
        label: "ipCountry",
        placeholder: "US",
        section: 0,
        float: false,
    },
    TextField {
        key: "local_save_path",
        label: "localSavePath",
        placeholder: "@emulator.fields.localSavePathPlaceholder",
        section: 1,
        float: false,
    },
    TextField {
        key: "saves_folder_name",
        label: "savesFolderName",
        placeholder: "",
        section: 1,
        float: false,
    },
    TextField {
        key: "key_combo",
        label: "keyCombo",
        placeholder: "ShiftTab",
        section: 4,
        float: false,
    },
    TextField {
        key: "font_size",
        label: "fontSize",
        placeholder: "16",
        section: 4,
        float: true,
    },
];

pub const BOOL_FIELDS: &[BoolField] = &[
    BoolField {
        key: "offline",
        label: "offline",
        section: 2,
    },
    BoolField {
        key: "steam_deck",
        label: "steamDeck",
        section: 2,
    },
    BoolField {
        key: "disable_networking",
        label: "disableNetworking",
        section: 2,
    },
    BoolField {
        key: "disable_lan_only",
        label: "disableLanOnly",
        section: 2,
    },
    BoolField {
        key: "record_playtime",
        label: "recordPlaytime",
        section: 3,
    },
    BoolField {
        key: "achievements_bypass",
        label: "achievementsBypass",
        section: 3,
    },
    BoolField {
        key: "force_steamhttp_success",
        label: "forceSteamhttpSuccess",
        section: 3,
    },
    BoolField {
        key: "enable_steam_preowned_ids",
        label: "enableSteamPreownedIds",
        section: 3,
    },
    BoolField {
        key: "free_weekend",
        label: "freeWeekend",
        section: 3,
    },
    BoolField {
        key: "enable_experimental_overlay",
        label: "enableExperimentalOverlay",
        section: 4,
    },
    BoolField {
        key: "disable_achievement_notification",
        label: "disableAchievementNotification",
        section: 4,
    },
    BoolField {
        key: "overlay_always_show_fps",
        label: "overlayAlwaysShowFps",
        section: 4,
    },
    BoolField {
        key: "overlay_always_show_playtime",
        label: "overlayAlwaysShowPlaytime",
        section: 4,
    },
];

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

fn count_bucket(n: usize) -> &'static str {
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

    fn emu_side_scans(&mut self) {
        let Some(emu) = &self.emu else { return };
        let dir = emu.game_dir.clone();
        let app_id = emu.app_id.clone();
        let standalone = emu.standalone;
        let edit = emu.edit_mode;
        let core = self.core.clone();
        let data = self.data_dir.clone();
        if !edit {
            self.spawn(async move {
                let r = smd_core::services::emulator::fetch_release_info(&core.http_client, &data)
                    .await;
                apply(move |app| {
                    if let Some(e) = app.emu.as_mut() {
                        e.release = Some(r);
                    }
                })
            });
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
                }
            }
            Action::EmuToggleField(i) => {
                if let (Some(e), Some(f)) = (self.emu.as_mut(), BOOL_FIELDS.get(*i)) {
                    let v = e.bools.get(f.key).copied().unwrap_or(false);
                    e.bools.insert(f.key, !v);
                }
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
                if let Modal::Confirm(c) = &mut m {
                    c.note = Some(tf("modals.emuRevert.scope", &[("count", &count)]));
                }
                self.queue_modal(m);
            }
            Action::EmuRevertConfirmed => {
                self.close_modal();
                self.emu_revert();
            }
            Action::EmuMergeDlc => self.emu_merge(),
            Action::EmuRemoveDrm => self.emu_remove_drm(),
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
        let targets = e.selected_targets();
        if targets.is_empty() {
            e.status = Some((Tone::Error, t("emulator.applyNoSelection")));
            return;
        }
        if e.app_id.is_empty() {
            e.status = Some((
                Tone::Error,
                tf("emulator.applyError", &[("message", &"missing app id")]),
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
            )
            .await;
            apply(move |app| app.on_emu_applied(r, targets, variant_name, gathered))
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
                    self.prefs.last_emu_settings = Some(gathered.unwrap_or_default());
                    self.save_prefs();
                    let targets_c = targets.clone();
                    self.sync_bypass(targets_c, move |app, extra| {
                        let Some(e) = app.emu.as_mut() else { return };
                        e.busy = false;
                        let mut msg = tf(
                            "emulator.applySuccess",
                            &[("count", &success), ("total", &total)],
                        );
                        if !extra.is_empty() {
                            msg.push_str("\n\n");
                            msg.push_str(&extra);
                        }
                        e.status = Some((Tone::Success, msg));
                        if e.standalone {
                            app.refresh_emu_scan();
                        } else {
                            e.apply_complete = true;
                            app.focus = Some(Fid::new("emu.apply"));
                        }
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
                                "{}\n\n{}",
                                tf(
                                    "emulator.applyPartial",
                                    &[("success", &success), ("failed", &failed)]
                                ),
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
                    let mut m = Modal::confirm(
                        t("modals.emuDownload.title"),
                        format!(
                            "{}\nhttps://github.com/Detanup01/gbe_fork/releases/tag/{}",
                            t("modals.emuDownload.body"),
                            tag
                        ),
                        t("modals.emuDownload.yes"),
                        t("modals.emuDownload.no"),
                        false,
                        Action::EmuDownloadConfirmed,
                    );
                    if let Modal::Confirm(c) = &mut m {
                        c.on_no = Some(Action::EmuDownloadCancelled);
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
                        e.status =
                            Some((Tone::Error, tf("emulator.applyError", &[("message", &err)])));
                    }
                }
            }
        }
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
        let entry = self.emu_entry();
        self.spawn(async move {
            let r = smd_core::ops::emulator::revert_replacement(paths.clone()).await;
            for p in &paths {
                let _ = smd_core::services::steam_api_bypass::revert_for_target(std::path::Path::new(p));
            }
            apply(move |app| {
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
                    .map(|r| format!("{}\n    {}", file_label(&r.path), r.error.clone().unwrap_or_else(|| t("emulator.applyReasonUnknown"))))
                    .collect::<Vec<_>>()
                    .join("\n");
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
                        tf("emulator.drmRemoveError", &[("message", &err)]) + &h,
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
                        .unwrap_or_else(|| "unknown error".into());
                    let h = mono_hint(&msg);
                    e.drm_status = Some((
                        Tone::Error,
                        format!(
                            "{}\n\n{}{}",
                            tf(
                                "emulator.drmRemovePartial",
                                &[("success", &success), ("failed", &failed)]
                            ),
                            msg,
                            h
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

        if !e.edit_mode {
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
        let tabs: Vec<ButtonSpec> = SECTIONS
            .iter()
            .enumerate()
            .map(|(i, key)| {
                let kind = if e.section == i {
                    Btn::Primary
                } else {
                    Btn::Secondary
                };
                ButtonSpec::new(
                    t(key),
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
        for (i, f) in TEXT_FIELDS.iter().enumerate() {
            if f.section != e.section {
                continue;
            }
            let r = col.next(3);
            let label = t(&format!("emulator.fields.{}", f.label));
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
            let hk = t(&format!("emulator.fields.{}Hint", f.label));
            let h = widgets::wrap_height(&hk, col.w);
            let r = col.next(h);
            hint(&mut sv.buf, r, &hk);
        }
        for (i, f) in BOOL_FIELDS.iter().enumerate() {
            if f.section != e.section {
                continue;
            }
            let r = col.next(1);
            let on = e.bools.get(f.key).copied().unwrap_or(false);
            widgets::checkbox(
                &mut sv.buf,
                ctx,
                r,
                on,
                &t(&format!("emulator.fields.{}", f.label)),
                Fid::idx("emu.bool", i),
                Action::EmuToggleField(i),
                true,
            );
            let hk = t(&format!("emulator.fields.{}Hint", f.label));
            let h = widgets::wrap_height(&hk, col.w.saturating_sub(4));
            let r = col.next(h);
            hint(
                &mut sv.buf,
                Rect::new(r.x + 4, r.y, r.width.saturating_sub(4), r.height),
                &hk,
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, platform: Platform, arch: &str) -> ScannedFile {
        ScannedFile {
            path: path.into(),
            platform,
            arch: arch.into(),
            is_patched: false,
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
