use std::sync::Arc;

pub const DOWNLOAD_PROGRESS: &str = "download-progress";
pub const EMU_DOWNLOAD_PROGRESS: &str = "emu-download-progress";

pub trait EventSink: Send + Sync {
    fn emit(&self, channel: &str, payload: serde_json::Value);
}

pub type Sink = Arc<dyn EventSink>;

pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _channel: &str, _payload: serde_json::Value) {}
}
