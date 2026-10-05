mod file_ops;
mod search;
mod download;
mod settings;
mod system;
mod window;
mod updater;
mod history;
mod queue;
mod shortcuts;
mod telemetry;
mod emulator;
mod steam_library;
mod steamless;
mod steam_api_bypass;
mod native_download;

pub use file_ops::*;
pub use search::*;
pub use download::*;
pub use settings::*;
pub use system::*;
pub use window::*;
pub use updater::*;
pub use history::*;
pub use queue::*;
pub use shortcuts::*;
pub use telemetry::*;
pub use emulator::*;
pub use steam_library::*;
pub use steamless::*;
pub use steam_api_bypass::*;
pub use native_download::*;

use std::path::PathBuf;
use std::sync::Arc;

use smd_core::services::events::{EventSink, Sink};
use tauri::{AppHandle, Emitter, Manager};

pub(crate) fn app_data_dir(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."))
}

struct TauriSink(AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, channel: &str, payload: serde_json::Value) {
        if let Err(e) = self.0.emit(channel, payload) {
            eprintln!("[Events] Failed to emit {} event: {}", channel, e);
        }
    }
}

pub(crate) fn sink(app: &AppHandle) -> Sink {
    Arc::new(TauriSink(app.clone()))
}
