use egui::viewport::ResizeDirection;
use egui::{CursorIcon, ViewportCommand};
use serde_json::{json, Value};

use super::App;

const EDGE: f32 = 5.0;

impl App {
    pub(super) fn window_frame(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.close_allowed {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.request_close();
        }
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return;
        }
        let window = ctx.content_rect();
        let Some(pointer) = ctx.pointer_hover_pos() else { return };
        let left = pointer.x - window.left() <= EDGE;
        let right = window.right() - pointer.x <= EDGE;
        let top = pointer.y - window.top() <= EDGE;
        let bottom = window.bottom() - pointer.y <= EDGE;
        let direction = match (left, right, top, bottom) {
            (true, _, true, _) => Some((ResizeDirection::NorthWest, CursorIcon::ResizeNwSe)),
            (_, true, true, _) => Some((ResizeDirection::NorthEast, CursorIcon::ResizeNeSw)),
            (true, _, _, true) => Some((ResizeDirection::SouthWest, CursorIcon::ResizeNeSw)),
            (_, true, _, true) => Some((ResizeDirection::SouthEast, CursorIcon::ResizeNwSe)),
            (true, ..) => Some((ResizeDirection::West, CursorIcon::ResizeHorizontal)),
            (_, true, ..) => Some((ResizeDirection::East, CursorIcon::ResizeHorizontal)),
            (_, _, true, _) => Some((ResizeDirection::North, CursorIcon::ResizeVertical)),
            (_, _, _, true) => Some((ResizeDirection::South, CursorIcon::ResizeVertical)),
            _ => None,
        };
        if let Some((direction, cursor)) = direction {
            ctx.set_cursor_icon(cursor);
            if ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            }
        }
    }

    pub fn minimize(&self) {
        self.ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }

    pub fn toggle_maximize(&self) {
        let maximized = self.ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        self.ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }

    pub fn start_window_drag(&self) {
        self.ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    pub fn set_window_progress(&mut self, percent: Option<u8>, paused: bool, title: String) {
        self.platform.set_progress(percent, paused);
        self.ctx.send_viewport_cmd(ViewportCommand::Title(title));
    }

    pub fn restart(&mut self) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).args(std::env::args_os().skip(1)).spawn();
        }
        self.close_allowed = true;
        self.ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    pub fn request_close(&mut self) {
        if self.close_pending {
            return;
        }
        self.close_pending = true;
        self.tasks.call("has_active_downloads", Value::Null, |app, active: Result<bool, String>| {
            app.close_pending = false;
            if active.unwrap_or(false) {
                app.ui.show("close-modal");
            } else {
                app.close_now();
            }
        });
    }

    pub fn close_now(&mut self) {
        super::logic::journey::exit(self);
        let tasks = self.tasks.clone();
        self.tasks.spawn(async move {
            let _ = tasks.invoke_raw("end_session", json!({ "reason": "close", "clearDownloads": true })).await;
            tasks.post(|app| {
                app.close_allowed = true;
                app.ctx.send_viewport_cmd(ViewportCommand::Close);
            });
        });
    }
}
