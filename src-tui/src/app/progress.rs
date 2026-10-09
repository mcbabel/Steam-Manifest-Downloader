use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Sparkline, Widget};
use serde_json::Value;
use smd_core::services::followup::PendingFollowup;
use smd_core::services::history::HistoryEntry;

use super::action::{Action, ListId, Page, ScrollTarget, Step};
use super::state::{DepotState, Finished, LogKind, LogLine, Wizard};
use super::{apply, App, Modal};
use crate::i18n::{t, tf};
use crate::theme;
use crate::ui::widgets::{self, Btn, ButtonSpec, Tone};
use crate::ui::{split_h, take_bottom, take_top, Ctx, Fid, Kind};

const SPEED_WINDOW: Duration = Duration::from_millis(3000);
const SPEED_MIN_WINDOW: Duration = Duration::from_millis(1500);
const ETA_INTERVAL: Duration = Duration::from_millis(1500);
const LOG_CAP: usize = 5000;
const SHUTDOWN_DELAY: Duration = Duration::from_secs(60);
const SPEED_HISTORY: usize = 120;

fn s(v: &Value, k: &str) -> Option<String> {
    match v.get(k)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn u(v: &Value, k: &str) -> Option<u64> {
    v.get(k)
        .and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f as u64)))
}

fn format_eta(secs: f64) -> String {
    if !secs.is_finite() || !(0.0..=86400.0).contains(&secs) {
        return t("tui.progress.calculating");
    }
    let secs = secs as u64;
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    let span = if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    };
    tf("tui.progress.remaining", &[("time", &span)])
}

fn format_elapsed(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

fn speed_text(bps: f64) -> String {
    let mbps = bps / (1024.0 * 1024.0);
    if mbps >= 1.0 {
        format!("↓ {:.1} MB/s", mbps)
    } else {
        format!("↓ {:.0} KB/s", bps / 1024.0)
    }
}

impl App {
    pub(super) fn log(&mut self, kind: LogKind, text: impl Into<String>) {
        if let Some(p) = self.wiz.progress.as_mut() {
            for line in text.into().split('\n') {
                p.log.push(LogLine {
                    text: line.to_string(),
                    kind,
                });
            }
            if p.log.len() > LOG_CAP {
                let drop = p.log.len() - LOG_CAP;
                p.log.drain(..drop);
            }
        }
    }

    pub(super) fn scroll_log(&mut self, delta: i64) {
        if let Some(p) = self.wiz.progress.as_mut() {
            let max = p.log.len().saturating_sub(1) as i64;
            let off = (p.log_offset as i64 - delta).clamp(0, max);
            p.log_offset = off as usize;
            p.log_follow = off == 0;
        }
    }

    fn depot_status(&mut self, depot_id: &str, state: DepotState, text: impl Into<String>) {
        if let Some(p) = self.wiz.progress.as_mut() {
            if let Some(d) = p.depots.iter_mut().find(|d| d.depot_id == depot_id) {
                d.state = state;
                d.text = text.into();
            }
        }
    }

    fn overall(&mut self, current: usize, total: usize) {
        if total == 0 {
            return;
        }
        if let Some(p) = self.wiz.progress.as_mut() {
            let pct = ((current as f64 / total as f64) * 100.0).round().min(99.0);
            p.overall = pct / 100.0;
        }
    }

    pub(super) fn on_download_event(&mut self, msg: Value) {
        if self.wiz.progress.is_none() {
            return;
        }
        let job = s(&msg, "jobId");
        if self.wiz.job_id.is_none()
            && self
                .wiz
                .progress
                .as_ref()
                .is_some_and(|p| p.finished.is_none())
        {
            if let Some(j) = &job {
                if j != "native" {
                    self.wiz.job_id = Some(j.clone());
                }
            }
        }
        if job.is_some() && job != self.wiz.job_id {
            return;
        }
        match msg.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "status" => {
                if s(&msg, "step").as_deref() == Some("disk_space") {
                    let free = msg.get("freeGB").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let drive = s(&msg, "drive").unwrap_or_default();
                    if let Some(p) = self.wiz.progress.as_mut() {
                        p.disk = Some((free, drive));
                    }
                } else {
                    self.on_status(&msg);
                }
            }
            "output" => self.on_output(&msg),
            "depot_complete" => {
                let id = s(&msg, "depotId").unwrap_or_default();
                self.depot_status(&id, DepotState::Done, t("history.statusComplete"));
                let (cur, tot) = (
                    u(&msg, "current").unwrap_or(0) as usize,
                    u(&msg, "total").unwrap_or(0) as usize,
                );
                self.overall(cur, tot);
                if let Some(p) = self.wiz.progress.as_mut() {
                    p.depot_ratio = 1.0;
                    p.depot_text = "100%".into();
                }
            }
            "manifest_source" => {
                let source = s(&msg, "source").unwrap_or_default();
                let kind = match source.as_str() {
                    "manifesthub_fallback" => LogKind::Warn,
                    "manifesthub_unavailable" => LogKind::Stderr,
                    _ => LogKind::Info,
                };
                let label_key = format!("events.src.label.{}", source);
                let label = match t(&label_key) {
                    l if l == label_key => source.clone(),
                    l => l,
                };
                let line = tf(
                    "events.line",
                    &[
                        ("label", &label),
                        ("depot", &s(&msg, "depotId").unwrap_or_default()),
                        ("text", &crate::i18n::event_text(&msg)),
                    ],
                );
                self.log(kind, line);
                if source == "manifesthub_unavailable" {
                    if let Some(p) = self.wiz.progress.as_mut() {
                        p.suggest_mh_key = true;
                    }
                }
            }
            "complete" => self.on_complete(&msg),
            "error" => self.on_error(&msg),
            "cancelled" => self.on_cancelled(&msg),
            _ => {}
        }
    }

    fn on_status(&mut self, msg: &Value) {
        let step = s(msg, "step").unwrap_or_default();
        let depot = s(msg, "depotId").unwrap_or_default();
        let current = u(msg, "current").map(|n| n as usize);
        let total = u(msg, "total").map(|n| n as usize);
        if let Some(p) = self.wiz.progress.as_mut() {
            p.stage = step.clone();
        }
        let set_status = |app: &mut App, text: String| {
            if let Some(p) = app.wiz.progress.as_mut() {
                p.status = text;
            }
        };
        match step.as_str() {
            "checking_branch" => {
                let app_id = s(msg, "appId").unwrap_or_default();
                set_status(self, tf("tui.progress.checkingBranch", &[("app", &app_id)]));
                self.log(
                    LogKind::Info,
                    tf("tui.progress.checkingBranch", &[("app", &app_id)]),
                );
            }
            "branch_found" => {
                let line = if msg.get("key").is_some() {
                    crate::i18n::event_text(msg)
                } else {
                    tf(
                        "tui.progress.branchFound",
                        &[("info", &s(msg, "lastUpdated").unwrap_or_default())],
                    )
                };
                self.log(LogKind::Success, format!("✓ {}", line));
            }
            "downloading_manifests" => {
                set_status(
                    self,
                    tf(
                        "tui.progress.downloadingManifests",
                        &[("total", &total.unwrap_or(0))],
                    ),
                );
            }
            "downloading_manifest" => {
                if let (Some(c), Some(tot)) = (current, total) {
                    set_status(
                        self,
                        tf(
                            "tui.progress.downloadingManifest",
                            &[("current", &c), ("total", &tot), ("depot", &depot)],
                        ),
                    );
                    self.overall(c.saturating_sub(1), tot * 2);
                }
                self.depot_status(&depot, DepotState::Active, t("tui.progress.manifestActive"));
                if let Some(f) = s(msg, "filename") {
                    self.log(
                        LogKind::Info,
                        tf("tui.progress.downloadingFile", &[("file", &f)]),
                    );
                }
            }
            "downloading_manifest_hub" => {
                let mid = s(msg, "manifestId").unwrap_or_default();
                set_status(self, tf("tui.progress.manifestHub", &[("depot", &depot)]));
                self.depot_status(
                    &depot,
                    DepotState::Active,
                    tf("tui.progress.customManifest", &[("id", &mid)]),
                );
                self.log(
                    LogKind::Info,
                    tf(
                        "tui.progress.manifestHubLine",
                        &[("depot", &depot), ("id", &mid)],
                    ),
                );
            }
            "manifest_hub_rate_limited" | "retrying_depot" => {
                if msg.get("message").is_some() {
                    self.log(LogKind::Warn, crate::i18n::event_text(msg));
                }
            }
            "depot_up_to_date" | "removed_stale_files" => {
                if msg.get("message").is_some() {
                    self.log(LogKind::Info, crate::i18n::event_text(msg));
                }
            }
            "generating_keys" => {
                set_status(self, t("tui.progress.generatingKeys"));
                self.log(LogKind::Info, t("tui.progress.generatingKeys"));
            }
            "keys_generated" => {
                let n = u(msg, "depotCount").unwrap_or(0);
                self.log(
                    LogKind::Success,
                    format!("✓ {}", tf("tui.progress.keysGenerated", &[("count", &n)])),
                );
            }
            "starting_downloader" => {
                set_status(
                    self,
                    tf(
                        "tui.progress.startingDownloader",
                        &[("total", &total.unwrap_or(0))],
                    ),
                );
            }
            "running_downloader" => {
                let size = self
                    .wiz
                    .parsed
                    .as_ref()
                    .and_then(|p| p.depots.iter().find(|d| d.depot_id == depot))
                    .and_then(|d| d.size_bytes)
                    .unwrap_or(0);
                let base = self.wiz.selected.len();
                if let Some(p) = self.wiz.progress.as_mut() {
                    p.byte_samples.clear();
                    p.percent_samples.clear();
                    p.last_update = Instant::now();
                    p.last_speed_display = None;
                    if !depot.is_empty() {
                        p.current_depot = Some(depot.clone());
                        p.current_depot_size = size;
                    }
                    p.depot_ratio = 0.0;
                    p.depot_text = "0%".into();
                }
                if let (Some(c), Some(tot)) = (current, total) {
                    set_status(
                        self,
                        tf(
                            "tui.progress.runningDownloader",
                            &[("current", &c), ("total", &tot), ("depot", &depot)],
                        ),
                    );
                    self.overall(base + c - 1, base + tot);
                }
                self.depot_status(&depot, DepotState::Active, t("tui.progress.depotActive"));
                if let Some(cmd) = s(msg, "command") {
                    let text = if msg.get("key").is_some() {
                        crate::i18n::event_text(msg)
                    } else {
                        cmd
                    };
                    self.log(LogKind::Info, format!("> {}", text));
                }
            }
            "paused" | "resumed" => {}
            _ => {}
        }
    }

    fn on_output(&mut self, msg: &Value) {
        let completed = u(msg, "completedBytes");
        let total = u(msg, "totalBytes");
        if let (Some(done), Some(tot)) = (completed, total) {
            if tot > 0 {
                let pct = msg
                    .get("percent")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(done as f64 * 100.0 / tot as f64);
                let net = u(msg, "networkBytes");
                self.depot_bytes(pct, done, tot, net);
                let skipped = u(msg, "skippedChunks").unwrap_or(0);
                if skipped > 0 {
                    let milestone = skipped / 25;
                    let shown = self
                        .wiz
                        .progress
                        .as_ref()
                        .map(|p| p.last_skipped_shown)
                        .unwrap_or(0);
                    if milestone > shown {
                        if let Some(p) = self.wiz.progress.as_mut() {
                            p.last_skipped_shown = milestone;
                        }
                        let mb = format!(
                            "{:.1}",
                            u(msg, "skippedBytes").unwrap_or(0) as f64 / (1024.0 * 1024.0)
                        );
                        self.log(
                            LogKind::Success,
                            format!(
                                "✓ {}",
                                tf(
                                    "progress.resumedChunks",
                                    &[("count", &skipped), ("mb", &mb)]
                                )
                            ),
                        );
                    }
                }
                return;
            }
        }
        let text = s(msg, "output").or_else(|| s(msg, "line"));
        if let Some(text) = text {
            let kind = if s(msg, "stream").as_deref() == Some("stderr") {
                LogKind::Stderr
            } else {
                LogKind::Stdout
            };
            for line in text.lines() {
                let trimmed = line.trim_start();
                if let Some(pct) = trimmed
                    .split('%')
                    .next()
                    .and_then(|p| p.parse::<f64>().ok())
                {
                    if trimmed.contains('%') && pct <= 100.0 {
                        self.depot_percent(pct);
                    }
                }
            }
            self.log(kind, text);
        }
    }

    fn depot_bytes(&mut self, pct: f64, done: u64, total: u64, net: Option<u64>) {
        let Some(p) = self.wiz.progress.as_mut() else {
            return;
        };
        p.depot_ratio = (pct / 100.0).min(1.0);
        p.depot_text = format!(
            "{:.1}% — {} / {}",
            pct,
            widgets::fmt_bytes(done),
            widgets::fmt_bytes(total)
        );
        let now = Instant::now();
        p.last_update = now;
        p.byte_samples.push_back((now, net.unwrap_or(done), done));
        while p.byte_samples.len() > 2
            && p.byte_samples
                .front()
                .is_some_and(|s| now.duration_since(s.0) > SPEED_WINDOW)
        {
            p.byte_samples.pop_front();
        }
        if p.byte_samples.len() < 2 {
            return;
        }
        let (t0, n0, d0) = *p.byte_samples.front().unwrap();
        let (t1, n1, d1) = *p.byte_samples.back().unwrap();
        let dt = t1.duration_since(t0);
        if dt < SPEED_MIN_WINDOW || n1 <= n0 {
            return;
        }
        let secs = dt.as_secs_f64();
        let net_bps = (n1 - n0) as f64 / secs;
        let dec_bps = d1.saturating_sub(d0) as f64 / secs;
        let remaining = total.saturating_sub(done) as f64;
        let eta = if dec_bps > 0.0 {
            remaining / dec_bps
        } else {
            f64::INFINITY
        };
        if p.last_speed_display
            .is_some_and(|l| now.duration_since(l) < ETA_INTERVAL)
        {
            return;
        }
        p.last_speed_display = Some(now);
        p.speed = Some(speed_text(net_bps));
        p.eta = Some(format_eta(eta));
        p.speed_history.push_back(net_bps as u64);
        while p.speed_history.len() > SPEED_HISTORY {
            p.speed_history.pop_front();
        }
    }

    fn depot_percent(&mut self, pct: f64) {
        let Some(p) = self.wiz.progress.as_mut() else {
            return;
        };
        p.depot_ratio = (pct / 100.0).min(1.0);
        p.depot_text = format!("{:.1}%", pct);
        let now = Instant::now();
        p.last_update = now;
        p.percent_samples.push_back((now, pct));
        while p.percent_samples.len() > 10 {
            p.percent_samples.pop_front();
        }
        if p.percent_samples.len() < 2 {
            return;
        }
        let (t0, p0) = *p.percent_samples.front().unwrap();
        let (t1, p1) = *p.percent_samples.back().unwrap();
        let dt = t1.duration_since(t0).as_secs_f64();
        if dt <= 0.0 || p1 <= p0 {
            return;
        }
        if p.last_speed_display
            .is_some_and(|l| now.duration_since(l) < ETA_INTERVAL)
        {
            return;
        }
        p.last_speed_display = Some(now);
        let per_sec = (p1 - p0) / dt;
        p.eta = Some(format_eta((100.0 - pct) / per_sec));
        if p.current_depot_size > 0 {
            let bps = p.current_depot_size as f64 * (p1 - p0) / 100.0 / dt;
            p.speed = Some(speed_text(bps));
            p.speed_history.push_back(bps as u64);
            while p.speed_history.len() > SPEED_HISTORY {
                p.speed_history.pop_front();
            }
        } else {
            p.speed = Some(format!("↓ {:.2}%/s", per_sec));
        }
    }

    pub(super) fn progress_tick(&mut self) {
        let Some(p) = self.wiz.progress.as_mut() else {
            return;
        };
        if p.finished.is_some() || p.current_depot.is_none() || p.paused {
            return;
        }
        let idle = p.last_update.elapsed();
        if idle > Duration::from_secs(5) {
            p.speed = Some(t("tui.progress.largeFile"));
            p.eta = Some(tf(
                "tui.progress.waiting",
                &[("time", &format_elapsed(idle.as_secs()))],
            ));
        }
    }

    fn on_complete(&mut self, msg: &Value) {
        let outcome = msg
            .get("diag")
            .and_then(|d| d.get("outcome"))
            .and_then(|o| o.as_str())
            .unwrap_or("complete")
            .to_string();
        let all_ok = outcome == "complete";
        let results: Vec<Value> = msg
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let message = crate::i18n::event_text(msg);
        if let Some(p) = self.wiz.progress.as_mut() {
            p.overall = 1.0;
            p.depot_ratio = 1.0;
            p.speed = None;
            p.eta = None;
            p.status = match outcome.as_str() {
                "complete" => t("tui.progress.complete"),
                "partial" => t("tui.progress.incomplete"),
                _ => t("tui.progress.failed"),
            };
            p.results = results.clone();
        }
        let mut props = serde_json::json!({ "success": all_ok });
        if let (Some(obj), Some(diag)) = (
            props.as_object_mut(),
            msg.get("diag").and_then(|d| d.as_object()),
        ) {
            for (k, v) in diag {
                obj.insert(k.clone(), v.clone());
            }
        }
        self.emit("download_completed", Some(props));
        for r in &results {
            let id = s(r, "depotId").unwrap_or_default();
            let ok = r.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
            self.depot_status(
                &id,
                if ok {
                    DepotState::Done
                } else {
                    DepotState::Error
                },
                if ok {
                    t("history.statusComplete")
                } else {
                    t("history.statusFailed")
                },
            );
        }
        self.log(
            if all_ok {
                LogKind::Success
            } else {
                LogKind::Error
            },
            format!("\n{}", message),
        );
        let repairing = self
            .wiz
            .main_app_id()
            .is_some_and(|id| self.wiz.active_repair(&id).is_some());
        if repairing {
            let repaired: u64 = results
                .iter()
                .filter_map(|r| r.get("downloadedBytes").and_then(|v| v.as_u64()))
                .sum();
            let line = if repaired > 0 {
                tf(
                    "progress.repairDone",
                    &[("size", &widgets::fmt_bytes(repaired))],
                )
            } else {
                t("progress.repairClean")
            };
            self.log(LogKind::Success, format!("✓ {}", line));
        }
        self.record_success_history(&results);
        self.finish_download(all_ok, message);
        if outcome == "partial" {
            if let Some(f) = self.wiz.progress.as_mut().and_then(|p| p.finished.as_mut()) {
                f.tone = Tone::Warning;
            }
        }
        self.check_emulator_support();
        if !self.queue_after(super::queue::QueueOutcome::Finished) {
            self.schedule_shutdown();
        }
    }

    fn record_success_history(&mut self, results: &[Value]) {
        let Some(dir) = self.wiz.result_dir.clone() else {
            return;
        };
        let total = if results.is_empty() {
            self.wiz.selected.len()
        } else {
            results.len()
        };
        let ok = if results.is_empty() {
            total
        } else {
            results
                .iter()
                .filter(|r| r.get("success").and_then(|v| v.as_bool()).unwrap_or(false))
                .count()
        };
        if ok == 0 {
            return;
        }
        let depot_ids: Vec<String> = if results.is_empty() {
            self.wiz.selected.iter().cloned().collect()
        } else {
            results.iter().filter_map(|r| s(r, "depotId")).collect()
        };
        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            app_id: self.wiz.main_app_id().unwrap_or_default(),
            game_name: self.wiz.game_name.clone(),
            header_image: self.wiz.header_image.clone(),
            depot_count: total,
            depots_downloaded: ok,
            status: if ok == total { "complete" } else { "partial" }.into(),
            download_dir: dir,
            started_at: self
                .wiz
                .download_started_at
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            source_repo: self.wiz.search_repo.clone(),
            depot_ids,
            resume_payload: None,
            size_bytes: None,
        };
        let data = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::services::history::add_entry(&data, entry).await;
            apply(move |app| {
                if let Err(e) = r {
                    app.toast(Tone::Error, crate::i18n::localize_error(&e));
                }
            })
        });
    }

    fn on_error(&mut self, msg: &Value) {
        let message = crate::i18n::event_text(msg);
        let depot = s(msg, "depotId");
        if let Some(d) = &depot {
            self.depot_status(d, DepotState::Error, t("tui.progress.error"));
        }
        self.log(LogKind::Error, tf("progress.errorLine", &[("message", &message)]));
        if depot.is_none() {
            let mut props = serde_json::json!({ "success": false });
            if let (Some(obj), Some(diag)) = (
                props.as_object_mut(),
                msg.get("diag").and_then(|d| d.as_object()),
            ) {
                for (k, v) in diag {
                    obj.insert(k.clone(), v.clone());
                }
            }
            self.emit("download_completed", Some(props));
            self.finish_download(false, message);
            if !self.queue_after(super::queue::QueueOutcome::Failed) {
                self.schedule_shutdown();
            }
        }
    }

    fn schedule_shutdown(&mut self) {
        if !self.shutdown_after {
            return;
        }
        self.shutdown_deadline = Some(Instant::now() + SHUTDOWN_DELAY);
        let mut modal = Modal::confirm(
            t("modals.shutdown.title"),
            tf("modals.shutdown.body", &[("seconds", &SHUTDOWN_DELAY.as_secs())]),
            t("modals.shutdown.now"),
            t("modals.shutdown.abort"),
            true,
            Action::ShutdownNow,
        );
        if let Modal::Confirm(c) = &mut modal {
            c.on_no = Some(Action::ShutdownAbort);
        }
        self.show_modal(modal);
    }

    fn shutdown_modal_body(&mut self, body: String) {
        if let Some(Modal::Confirm(c)) = self.modal.as_mut() {
            if c.on_yes == Action::ShutdownNow {
                c.body = body;
            }
        }
    }

    pub(super) fn shutdown_tick(&mut self) {
        let Some(deadline) = self.shutdown_deadline else {
            return;
        };
        let now = Instant::now();
        if now >= deadline {
            self.shutdown_now();
            return;
        }
        let secs = (deadline - now).as_secs() + 1;
        self.shutdown_modal_body(tf("modals.shutdown.body", &[("seconds", &secs)]));
    }

    pub(super) fn pending_followup(&self) -> Option<PendingFollowup> {
        let success = self
            .wiz
            .progress
            .as_ref()
            .and_then(|p| p.finished.as_ref())
            .is_some_and(|f| f.success);
        let has_next =
            self.shortcut_supported || self.steam_install.is_some() || self.emulator_available;
        if !success || !has_next {
            return None;
        }
        Some(PendingFollowup {
            app_id: self.wiz.main_app_id()?,
            game_name: self.wiz.game_name.clone(),
            header_image: self.wiz.header_image.clone(),
            download_dir: self.wiz.result_dir.clone()?,
            created_at: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    pub(super) fn offer_followup(&mut self, followup: PendingFollowup) {
        let name = followup
            .game_name
            .clone()
            .unwrap_or_else(|| format!("App {}", followup.app_id));
        let mut modal = Modal::confirm(
            t("modals.followup.title"),
            tf("modals.followup.body", &[("name", &name)]),
            t("modals.followup.resume"),
            t("modals.followup.later"),
            false,
            Action::FollowupResume,
        );
        if let Modal::Confirm(c) = &mut modal {
            c.on_no = Some(Action::FollowupLater);
            c.check = Some((t("modals.followup.dontAsk"), false));
        }
        self.followup = Some(followup);
        self.queue_modal(modal);
    }

    fn followup_dont_ask(&self) -> bool {
        matches!(&self.modal, Some(Modal::Confirm(c))
            if c.on_yes == Action::FollowupResume
                && c.check.as_ref().is_some_and(|(_, on)| *on))
    }

    fn clear_followup_file(&mut self, download_dir: &str) {
        let dir = self.data_dir.clone();
        let game = download_dir.to_string();
        self.spawn(async move {
            smd_core::services::followup::remove(&dir, &game).await;
            apply(|_| {})
        });
    }

    fn resume_followup(&mut self, followup: PendingFollowup) {
        self.page = Page::Wizard;
        self.wiz.result_dir = Some(followup.download_dir.clone());
        self.wiz.game_name = followup.game_name;
        self.wiz.header_image = followup.header_image;
        self.wiz.search_app_id = Some(followup.app_id);
        let dir = followup.download_dir;
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| {
                app.emulator_available = matches!(&r, Ok(v) if !v.is_empty());
                if app.shortcut_supported {
                    app.go_to_shortcut_step();
                } else if app.steam_install.is_some() {
                    app.go_to_steam_step();
                } else if app.emulator_available {
                    app.go_to_emulator_step();
                }
            })
        });
    }

    fn shutdown_now(&mut self) {
        self.shutdown_deadline = None;
        self.shutdown_modal_body(t("modals.shutdown.running"));
        let followup = self.pending_followup();
        let data = self.data_dir.clone();
        let tel = self.telemetry.clone();
        self.spawn(async move {
            let saved = match &followup {
                Some(f) => smd_core::services::followup::save(&data, f).await.is_ok(),
                None => false,
            };
            if let Some(tel) = tel {
                let _ = tokio::time::timeout(Duration::from_secs(3), tel.flush()).await;
            }
            let r = tokio::task::spawn_blocking(smd_core::ops::system::power_off)
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r);
            if let (Err(_), true, Some(f)) = (&r, saved, &followup) {
                smd_core::services::followup::remove(&data, &f.download_dir).await;
            }
            apply(move |app| {
                if let Err(e) = r {
                    app.shutdown_after = false;
                    if matches!(&app.modal, Some(Modal::Confirm(c)) if c.on_yes == Action::ShutdownNow) {
                        app.close_modal();
                    }
                    app.toast(Tone::Error, tf("modals.shutdown.failed", &[("message", &e)]));
                }
            })
        });
    }

    fn on_cancelled(&mut self, msg: &Value) {
        let text = match s(msg, "step").as_deref() {
            Some("cancelled_kept") => t("progress.cancelledKept"),
            Some("cancelled_cleanup") => t("progress.cancelledCleanup"),
            _ if msg.get("message").is_some() => crate::i18n::event_text(msg),
            _ => t("progress.cancelledCleanup"),
        };
        if let Some(p) = self.wiz.progress.as_mut() {
            p.overall = 0.0;
            p.status = t("progress.cancelledStatus");
            p.speed = None;
            p.eta = None;
        }
        self.log(LogKind::Error, format!("\n{}", text));
        self.emit(
            "download_completed",
            Some(serde_json::json!({ "success": false, "outcome": "cancelled" })),
        );
        self.finish_download(false, text);
        self.queue_after(super::queue::QueueOutcome::Cancelled);
    }

    pub(super) fn finish_download(&mut self, success: bool, message: String) {
        self.wiz.job_id = None;
        if let Some(p) = self.wiz.progress.as_mut() {
            if p.finished.is_some() {
                return;
            }
            p.finished = Some(Finished {
                success,
                message,
                tone: if success { Tone::Success } else { Tone::Error },
            });
            p.cancelling = false;
        }
        self.focus = Some(Fid::new("progress.next"));
    }

    fn check_emulator_support(&mut self) {
        self.emulator_available = false;
        let Some(dir) = self.wiz.result_dir.clone() else {
            return;
        };
        self.spawn(async move {
            let r = smd_core::ops::emulator::scan_game_dir(&dir).await;
            apply(move |app| {
                app.emulator_available = matches!(&r, Ok(v) if !v.is_empty());
            })
        });
    }

    pub(super) fn dispatch_progress(&mut self, a: &Action) -> bool {
        match a {
            Action::PauseResume => self.pause_resume(),
            Action::AskCancel => {
                if self.wiz.job_id.is_none() || !self.download_active() {
                    return true;
                }
                let keep = self.settings.cancel_keep_files;
                self.queue_modal(Modal::confirm(
                    t("modals.cancel.title"),
                    t(if keep {
                        "modals.cancel.bodyKeep"
                    } else {
                        "modals.cancel.body"
                    }),
                    t(if keep {
                        "modals.cancel.yesKeep"
                    } else {
                        "modals.cancel.yes"
                    }),
                    t("modals.cancel.no"),
                    true,
                    Action::CancelConfirmed,
                ));
            }
            Action::CancelConfirmed => {
                self.close_modal();
                self.cancel_download();
            }
            Action::ProgressNext => self.progress_next(),
            Action::Home => self.reset_wizard(),
            Action::ToggleShutdownAfter => self.shutdown_after = !self.shutdown_after,
            Action::ShutdownAbort => {
                self.shutdown_deadline = None;
                self.shutdown_after = false;
                self.close_modal();
            }
            Action::ShutdownNow => self.shutdown_now(),
            Action::FollowupResume => {
                self.close_modal();
                if let Some(f) = self.followup.take() {
                    self.clear_followup_file(&f.download_dir);
                    self.resume_followup(f);
                }
            }
            Action::FollowupLater => {
                let discard = self.followup_dont_ask();
                self.close_modal();
                if let Some(f) = self.followup.take().filter(|_| discard) {
                    self.clear_followup_file(&f.download_dir);
                }
            }
            _ => return false,
        }
        true
    }

    fn pause_resume(&mut self) {
        let Some(job) = self.wiz.job_id.clone() else {
            return;
        };
        let Some(p) = self.wiz.progress.as_ref() else {
            return;
        };
        if !p.engine_native || p.finished.is_some() {
            return;
        }
        let pause = !p.paused;
        let core = self.core.clone();
        let sink = self.sink.clone();
        self.spawn(async move {
            let r = smd_core::ops::download::pause_download(&sink, &core, job, pause).await;
            apply(move |app| {
                if r.is_ok() {
                    if let Some(p) = app.wiz.progress.as_mut() {
                        p.paused = pause;
                        p.last_update = Instant::now();
                    }
                    app.log(
                        LogKind::Info,
                        t(if pause {
                            "progress.pausedLine"
                        } else {
                            "progress.resumedLine"
                        }),
                    );
                }
            })
        });
    }

    fn cancel_download(&mut self) {
        let Some(job) = self.wiz.job_id.clone() else {
            return;
        };
        if let Some(p) = self.wiz.progress.as_mut() {
            p.cancelling = true;
        }
        self.log(LogKind::Info, t("tui.progress.cancelling"));
        let core = self.core.clone();
        let sink = self.sink.clone();
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = smd_core::ops::download::cancel_download(&sink, &core, &dir, job).await;
            apply(move |app| {
                if let Err(e) = r {
                    let lower = e.to_lowercase();
                    if lower.contains("not found") || lower.contains("not running") {
                        app.log(LogKind::Info, t("tui.progress.noLongerRunning"));
                        app.finish_download(false, t("tui.progress.ended"));
                    } else {
                        app.log(LogKind::Error, tf("progress.cancelFailed", &[("message", &e)]));
                        if let Some(p) = app.wiz.progress.as_mut() {
                            p.cancelling = false;
                        }
                    }
                }
            })
        });
    }

    fn progress_next(&mut self) {
        let Some(p) = self.wiz.progress.as_ref() else {
            return;
        };
        let Some(fin) = &p.finished else { return };
        if !fin.success {
            let suggest = p.suggest_mh_key;
            self.wiz.step = Step::Select;
            if suggest {
                self.wiz.mh_hint = Some(t("select.manifestHubAfterFailure"));
                self.focus = Some(Fid::new("select.mh"));
            } else {
                self.focus = None;
            }
            return;
        }
        if self.shortcut_supported {
            self.go_to_shortcut_step();
        } else if self.steam_install.is_some() {
            self.go_to_steam_step();
        } else if self.emulator_available {
            self.go_to_emulator_step();
        } else {
            self.reset_wizard();
        }
    }

    pub(super) fn reset_wizard(&mut self) {
        if self.download_active() {
            if let Some(job) = self.wiz.job_id.clone() {
                let core = self.core.clone();
                let sink = self.sink.clone();
                let dir = self.data_dir.clone();
                tokio::spawn(async move {
                    let _ = smd_core::ops::download::cancel_download(&sink, &core, &dir, job).await;
                });
            }
        }
        let tab = self.wiz.tab;
        let dl_dir = self.wiz.download_dir.trimmed();
        self.wiz = Wizard::new(&self.settings, &self.prefs.mh_api_key);
        self.wiz.tab = tab;
        if !dl_dir.is_empty() {
            self.wiz.download_dir.set(dl_dir);
        }
        self.emu = None;
        self.emulator_available = false;
        self.page = Page::Wizard;
        self.focus = None;
    }

    pub(super) fn render_progress(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        let shutdown_after = self.shutdown_after;
        let tick = self.tick;
        let Some(p) = self.wiz.progress.as_mut() else {
            return;
        };
        let mut rest = area;

        let (icon, style) = match p.finished.as_ref().map(|f| f.tone) {
            Some(Tone::Success) => ("✓".to_string(), theme::success()),
            Some(Tone::Warning) => ("!".to_string(), theme::warning()),
            Some(_) => ("✗".to_string(), theme::error()),
            None if p.paused => ("‖".to_string(), theme::warning()),
            None => (widgets::spinner(tick).to_string(), theme::accent()),
        };
        let status = if p.paused {
            format!("{} — {}", p.status, t("progress.pausedLine"))
        } else {
            p.status.clone()
        };
        widgets::line(
            buf,
            take_top(&mut rest, 1),
            Line::from(vec![
                Span::styled(format!("{} ", icon), style.add_modifier(Modifier::BOLD)),
                Span::styled(status, theme::text().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("   {}", format_elapsed(p.started.elapsed().as_secs())),
                    theme::muted(),
                ),
            ]),
        );
        let _ = take_top(&mut rest, 1);

        let label_w = 10u16;
        let pct_w = 6u16;
        let bar_row = |buf: &mut Buffer, row: Rect, label: &str, ratio: f64, right: &str| {
            widgets::text(
                buf,
                Rect::new(row.x, row.y, label_w, 1),
                label,
                theme::dim(),
            );
            let right_w = (widgets::width(right) + 1).max(pct_w).min(row.width / 2);
            let bar = Rect::new(
                row.x + label_w,
                row.y,
                row.width.saturating_sub(label_w + right_w + 1),
                1,
            );
            widgets::gauge(buf, bar, ratio);
            widgets::text(
                buf,
                Rect::new(bar.right() + 1, row.y, right_w, 1),
                right,
                theme::text(),
            );
        };
        let overall = p.overall;
        bar_row(
            buf,
            take_top(&mut rest, 1),
            &t("tui.progress.overall"),
            overall,
            &format!("{:.0}%", overall * 100.0),
        );
        let depot_label = p
            .current_depot
            .clone()
            .map(|d| format!("{} {}", t("tui.progress.depot"), d))
            .unwrap_or_else(|| t("tui.progress.depot"));
        bar_row(
            buf,
            take_top(&mut rest, 1),
            &widgets::truncate(&depot_label, label_w - 1),
            p.depot_ratio,
            &p.depot_text,
        );

        let row = take_top(&mut rest, 2);
        let mut info: Vec<Span> = Vec::new();
        if let Some(sp) = &p.speed {
            info.push(Span::styled(sp.clone(), theme::accent_bold()));
        }
        if let Some(eta) = &p.eta {
            info.push(Span::styled(format!("   {}", eta), theme::dim()));
        }
        widgets::line(
            buf,
            Rect::new(row.x + label_w, row.y, row.width.saturating_sub(label_w), 1),
            Line::from(info),
        );
        if let Some((free, drive)) = &p.disk {
            let (txt, st) = if *free < 2.0 {
                (
                    tf(
                        "tui.progress.diskCritical",
                        &[("free", free), ("drive", drive)],
                    ),
                    theme::error(),
                )
            } else if *free < 10.0 {
                (
                    tf("tui.progress.diskLow", &[("free", free), ("drive", drive)]),
                    theme::warning(),
                )
            } else {
                (
                    tf("tui.progress.disk", &[("free", free), ("drive", drive)]),
                    theme::muted(),
                )
            };
            widgets::text(
                buf,
                Rect::new(
                    row.x + label_w,
                    row.y + 1,
                    row.width.saturating_sub(label_w),
                    1,
                ),
                &txt,
                st,
            );
        }
        if row.width > 90 && !p.speed_history.is_empty() {
            let w = (row.width / 4).min(40);
            let spark_rect = Rect::new(row.right().saturating_sub(w), row.y, w, 2);
            let data: Vec<u64> = p
                .speed_history
                .iter()
                .rev()
                .take(w as usize)
                .rev()
                .copied()
                .collect();
            Sparkline::default()
                .data(data)
                .style(Style::default().fg(theme::get().purple))
                .render(spark_rect, buf);
        }
        let _ = take_top(&mut rest, 1);

        let footer = take_bottom(&mut rest, 1);
        let _ = take_bottom(&mut rest, 1);
        if let Some(f) = &p.finished {
            let h = widgets::status_height(&f.message, rest.width).min(4);
            let r = take_bottom(&mut rest, h);
            widgets::status(buf, r, f.tone, &f.message, tick);
            let _ = take_bottom(&mut rest, 1);
        } else if p.engine_native {
            let note = t("tui.progress.nativeNote");
            let r = take_bottom(&mut rest, 1);
            widgets::text(buf, r, &note, theme::muted());
        }

        let (left, right) = if rest.width >= 80 {
            split_h(rest, 34, 1)
        } else {
            (Rect::new(rest.x, rest.y, 0, 0), rest)
        };
        if left.width > 0 {
            let inner = widgets::card(
                buf,
                left,
                Some(&t("tui.progress.depots")),
                ctx.is_focused(Fid::new("progress.depots")),
            );
            let range = widgets::list_window(
                p.selected_depot,
                &mut p.depot_offset,
                p.depots.len(),
                inner.height as usize,
            );
            for (row, i) in range.enumerate() {
                let d = &p.depots[i];
                let (icon, st) = match d.state {
                    DepotState::Pending => ("●", theme::muted()),
                    DepotState::Active => (widgets::spinner(tick), theme::accent()),
                    DepotState::Done => ("✓", theme::success()),
                    DepotState::Error => ("✗", theme::error()),
                };
                let r = Rect::new(
                    inner.x + 1,
                    inner.y + row as u16,
                    inner.width.saturating_sub(2),
                    1,
                );
                buf.set_string(r.x, r.y, icon, st);
                let text = if d.text.is_empty() {
                    t("tui.progress.waitingDepot")
                } else {
                    d.text.clone()
                };
                widgets::line(
                    buf,
                    Rect::new(r.x + 2, r.y, r.width.saturating_sub(2), 1),
                    Line::from(vec![
                        Span::styled(d.depot_id.clone(), theme::text()),
                        Span::styled(format!("  {}", text), theme::muted()),
                    ]),
                );
            }
            widgets::scrollbar(
                buf,
                inner,
                p.depots.len(),
                inner.height as usize,
                p.depot_offset,
            );
            ctx.focusable(
                Fid::new("progress.depots"),
                left,
                Kind::List(ListId::DepotProgress),
                None,
            );
            ctx.scroll_region(left, ScrollTarget::List(ListId::DepotProgress));
        }

        let log_fid = Fid::new("progress.log");
        let follow = if p.log_follow { "" } else { "  ⇣ End" };
        let title = format!("{}{}", t("progress.terminalTitle"), follow);
        let inner = widgets::card(buf, right, Some(&title), ctx.is_focused(log_fid));
        let inner = Rect::new(
            inner.x + 1,
            inner.y,
            inner.width.saturating_sub(2),
            inner.height,
        );
        let mut y = inner.bottom();
        let end = p.log.len().saturating_sub(p.log_offset);
        for line in p.log[..end].iter().rev() {
            let st = match line.kind {
                LogKind::Stdout => theme::text(),
                LogKind::Stderr => theme::warning(),
                LogKind::Info => theme::accent(),
                LogKind::Success => theme::success(),
                LogKind::Warn => theme::warning(),
                LogKind::Error => theme::error(),
            };
            let wrapped = widgets::wrap(&line.text, inner.width);
            for l in wrapped.iter().rev() {
                if y <= inner.y {
                    break;
                }
                y -= 1;
                buf.set_stringn(inner.x, y, l, inner.width as usize, st);
            }
            if y <= inner.y {
                break;
            }
        }
        widgets::scrollbar(
            buf,
            Rect::new(inner.x, inner.y, inner.width + 1, inner.height),
            p.log.len().max(1),
            (inner.height as usize).min(p.log.len().max(1)),
            p.log
                .len()
                .saturating_sub(p.log_offset + inner.height as usize),
        );
        ctx.focusable(log_fid, right, Kind::List(ListId::Log), None);
        ctx.scroll_region(right, ScrollTarget::List(ListId::Log));

        let mut specs = Vec::new();
        match &p.finished {
            None => {
                if p.engine_native {
                    let label = if p.paused {
                        t("progress.resume")
                    } else {
                        t("progress.pause")
                    };
                    specs.push(
                        ButtonSpec::new(
                            label,
                            Fid::new("progress.pause"),
                            Action::PauseResume,
                            Btn::Secondary,
                        )
                        .enabled(!p.cancelling),
                    );
                }
                specs.push(ButtonSpec::new(
                    t(if shutdown_after {
                        "tui.progress.shutdownOn"
                    } else {
                        "tui.progress.shutdownOff"
                    }),
                    Fid::new("progress.shutdown"),
                    Action::ToggleShutdownAfter,
                    Btn::Secondary,
                ));
                let label = if p.cancelling {
                    t("progress.cancelling")
                } else {
                    t("progress.cancel")
                };
                specs.push(
                    ButtonSpec::new(
                        label,
                        Fid::new("progress.cancel"),
                        Action::AskCancel,
                        Btn::Danger,
                    )
                    .enabled(!p.cancelling),
                );
                ctx.prefer(Fid::new("progress.cancel"));
            }
            Some(f) => {
                let has_next = self.shortcut_supported
                    || self.steam_install.is_some()
                    || self.emulator_available;
                let label = if !f.success {
                    t("progress.backToSelection")
                } else if has_next {
                    t("common.next")
                } else {
                    t("emulator.goToHome")
                };
                if f.success && self.wiz.result_dir.is_some() {
                    specs.push(ButtonSpec::new(
                        t("tui.history.openFolder"),
                        Fid::new("progress.open"),
                        Action::HistoryOpenFolder(usize::MAX),
                        Btn::Secondary,
                    ));
                }
                specs.push(ButtonSpec::new(
                    t("emulator.startNewDownload"),
                    Fid::new("progress.home"),
                    Action::Home,
                    Btn::Secondary,
                ));
                specs.push(ButtonSpec::new(
                    label,
                    Fid::new("progress.next"),
                    Action::ProgressNext,
                    Btn::Primary,
                ));
                ctx.prefer(Fid::new("progress.next"));
            }
        }
        widgets::buttons(buf, ctx, footer, &specs, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eta_formatting() {
        crate::i18n::set_language("en");
        assert!(format_eta(f64::INFINITY).contains("calculating"));
        assert!(format_eta(75.0).contains("1m 15s"));
        assert!(format_eta(3700.0).contains("1h 1m"));
        assert_eq!(format_elapsed(65), "1m 5s");
    }

    #[test]
    fn speed_units() {
        assert_eq!(speed_text(2.5 * 1024.0 * 1024.0), "↓ 2.5 MB/s");
        assert_eq!(speed_text(512.0 * 1024.0), "↓ 512 KB/s");
    }
}
