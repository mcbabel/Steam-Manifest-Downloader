use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use smd_core::ops::download::DownloadConfig;
use smd_core::services::download_queue::{self, QueuedDownload};

use super::action::{Action, Step};
use super::state::LogKind;
use super::{apply, App};
use crate::i18n::{t, tf};
use crate::ui::widgets::{self, Btn, ButtonSpec, Tone};
use crate::ui::{Ctx, Fid};

const NEXT_DELAY: Duration = Duration::from_secs(5);

pub enum QueueOutcome {
    Finished,
    Failed,
    Cancelled,
}

fn display_name(item: &QueuedDownload) -> String {
    item.game_name
        .clone()
        .unwrap_or_else(|| format!("App {}", item.app_id))
}

impl App {
    pub(super) fn load_queue(&mut self) {
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let q = download_queue::load(&dir).await;
            apply(move |app| app.queue = q)
        });
    }

    pub(super) fn queue_add(&mut self) {
        let size = self.selected_bytes();
        let Some((config, _, _)) = self.build_download() else {
            return;
        };
        let item = QueuedDownload {
            id: String::new(),
            app_id: config.app_id.clone(),
            game_name: config.game_name.clone(),
            header_image: config.header_image.clone(),
            depot_count: config.depots.len(),
            size_bytes: Some(size).filter(|s| *s > 0),
            depots: serde_json::to_value(&config.depots).unwrap_or_default(),
            config: serde_json::to_value(&config).unwrap_or_default(),
            added_at: Some(chrono::Utc::now().to_rfc3339()),
        };
        let name = display_name(&item);
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let r = download_queue::add(&dir, item).await;
            apply(move |app| match r {
                Ok(q) => {
                    let count = q.len();
                    app.queue = q;
                    app.toast(
                        Tone::Success,
                        tf("queue.added", &[("name", &name), ("count", &count)]),
                    );
                    app.reset_wizard();
                    app.wiz.step = Step::Source;
                }
                Err(e) => app.toast(Tone::Error, e),
            })
        });
    }

    pub(super) fn dispatch_queue(&mut self, a: &Action) -> bool {
        match a {
            Action::QueueStart => {
                if !self.queue_running && !self.download_active() && !self.queue.is_empty() {
                    self.queue_running = true;
                    self.queue_next_at = Some(Instant::now());
                }
            }
            Action::QueueStop => self.stop_queue(),
            Action::QueueClear => {
                let dir = self.data_dir.clone();
                self.spawn(async move {
                    let _ = download_queue::clear(&dir).await;
                    apply(|app| app.queue.clear())
                });
            }
            _ => return false,
        }
        true
    }

    fn stop_queue(&mut self) {
        self.queue_running = false;
        self.queue_next_at = None;
        self.toast(Tone::Info, t("queue.stopped"));
    }

    pub(super) fn queue_tick(&mut self) {
        if !self.queue_running {
            return;
        }
        let Some(at) = self.queue_next_at else {
            return;
        };
        if Instant::now() < at || self.download_active() {
            return;
        }
        self.queue_next_at = None;
        let dir = self.data_dir.clone();
        self.spawn(async move {
            let next = download_queue::take_next(&dir).await.ok().flatten();
            let rest = download_queue::load(&dir).await;
            apply(move |app| {
                app.queue = rest;
                match next {
                    Some(item) => app.start_queued(item),
                    None => app.queue_running = false,
                }
            })
        });
    }

    fn start_queued(&mut self, item: QueuedDownload) {
        let Ok(config) = serde_json::from_value::<DownloadConfig>(item.config.clone()) else {
            self.toast(Tone::Error, tf("queue.broken", &[("name", &display_name(&item))]));
            self.queue_next_at = Some(Instant::now());
            return;
        };
        self.reset_wizard();
        self.wiz.game_name = config.game_name.clone();
        self.wiz.header_image = config.header_image.clone();
        self.wiz.search_app_id = Some(config.app_id.clone());
        self.wiz.update_dir = config.update_dir.clone();
        self.wiz.update_app_id = config.update_dir.as_ref().map(|_| config.app_id.clone());
        let ids: Vec<String> = config.depots.iter().map(|d| d.depot_id.clone()).collect();
        self.wiz.selected = ids.iter().cloned().collect();
        let native = self.settings.use_native_downloader;
        self.begin_download(config, ids, native);
    }

    pub(super) fn queue_after(&mut self, outcome: QueueOutcome) -> bool {
        if !self.queue_running {
            return false;
        }
        if matches!(outcome, QueueOutcome::Cancelled) {
            self.stop_queue();
            return true;
        }
        if self.queue.is_empty() {
            self.queue_running = false;
            self.toast(Tone::Success, t("queue.finished"));
            return false;
        }
        if matches!(outcome, QueueOutcome::Finished) {
            if let Some(f) = self.pending_followup() {
                let dir = self.data_dir.clone();
                self.spawn(async move {
                    let _ = smd_core::services::followup::save(&dir, &f).await;
                    apply(|_| {})
                });
            }
        }
        self.log(
            LogKind::Info,
            tf("queue.nextIn", &[("seconds", &NEXT_DELAY.as_secs())]),
        );
        self.queue_next_at = Some(Instant::now() + NEXT_DELAY);
        true
    }

    pub(super) fn queue_bar_height(&self) -> u16 {
        if self.queue.is_empty() && !self.queue_running {
            0
        } else {
            2
        }
    }

    pub(super) fn render_queue_bar(&mut self, buf: &mut Buffer, area: Rect, ctx: &mut Ctx) {
        if area.height == 0 {
            return;
        }
        let names: Vec<String> = self
            .queue
            .iter()
            .enumerate()
            .map(|(i, q)| format!("{}. {}", i + 1, display_name(q)))
            .collect();
        let head = if self.queue_running {
            t("queue.runningShort")
        } else {
            t("queue.title")
        };
        let toggle = if self.queue_running {
            ButtonSpec::new(t("queue.stop"), Fid::new("queue.toggle"), Action::QueueStop, Btn::Secondary)
        } else {
            ButtonSpec::new(t("queue.start"), Fid::new("queue.toggle"), Action::QueueStart, Btn::Primary)
                .enabled(!self.queue.is_empty() && !self.download_active())
        };
        let clear = ButtonSpec::new(t("queue.clear"), Fid::new("queue.clear"), Action::QueueClear, Btn::Danger)
            .enabled(!self.queue.is_empty());
        let tw = widgets::button_width(&toggle.label);
        let cw = widgets::button_width(&clear.label);
        let text_w = area.width.saturating_sub(tw + cw + 4);
        let text = format!("{} ({}): {}", head, self.queue.len(), names.join("  ·  "));
        widgets::text(
            buf,
            Rect::new(area.x, area.y, text_w, 1),
            &text,
            crate::theme::text(),
        );
        widgets::button(buf, ctx, area.x + text_w + 2, area.y, tw, &toggle);
        widgets::button(buf, ctx, area.x + text_w + tw + 4, area.y, cw, &clear);
    }
}
