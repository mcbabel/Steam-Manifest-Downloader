pub mod commands;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use smd_core::services::events::{EventSink, Sink};
use smd_core::services::telemetry::Telemetry;
use smd_core::services::AppState;

pub type Reply = Result<Value, String>;
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
pub type Wake = Arc<dyn Fn() + Send + Sync>;

pub const EXIT_EVENT: &str = "app-exit";

pub trait Backend: Send + Sync + 'static {
    fn invoke(&self, name: &str, args: Value) -> BoxFuture<Reply>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppEvent {
    pub channel: String,
    pub payload: Value,
}

#[derive(Clone)]
pub struct Events {
    tx: Sender<AppEvent>,
    wake: Arc<Mutex<Option<Wake>>>,
}

impl Events {
    pub fn channel() -> (Self, Receiver<AppEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (Self { tx, wake: Arc::new(Mutex::new(None)) }, rx)
    }

    pub fn set_wake(&self, wake: Wake) {
        *self.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(wake);
    }

    pub fn emit(&self, channel: &str, payload: Value) {
        let _ = self.tx.send(AppEvent { channel: channel.to_string(), payload });
        let wake = self.wake.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(wake) = wake {
            wake();
        }
    }
}

impl EventSink for Events {
    fn emit(&self, channel: &str, payload: Value) {
        Events::emit(self, channel, payload);
    }
}

struct CxInner {
    state: Arc<AppState>,
    data_dir: PathBuf,
    events: Events,
    version: String,
}

#[derive(Clone)]
pub struct Cx {
    inner: Arc<CxInner>,
}

impl Cx {
    pub fn new(state: Arc<AppState>, data_dir: PathBuf, events: Events, version: &str) -> Self {
        Self { inner: Arc::new(CxInner { state, data_dir, events, version: version.to_string() }) }
    }

    pub fn state(&self) -> Arc<AppState> {
        self.inner.state.clone()
    }

    pub fn data_dir(&self) -> PathBuf {
        self.inner.data_dir.clone()
    }

    pub fn sink(&self) -> Sink {
        Arc::new(self.inner.events.clone())
    }

    pub fn emit(&self, channel: &str, payload: Value) {
        self.inner.events.emit(channel, payload);
    }

    pub fn version(&self) -> &str {
        &self.inner.version
    }

    pub fn telemetry(&self) -> Option<Telemetry> {
        self.inner.state.telemetry.clone()
    }

    pub async fn end_session(&self, reason: &str, timeout_secs: u64) {
        if let Some(telemetry) = self.telemetry() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), telemetry.end_session(reason)).await;
        }
    }

    pub fn request_exit(&self) {
        self.emit(EXIT_EVENT, Value::Null);
    }
}

pub(crate) fn parse<T: DeserializeOwned>(args: Value) -> Result<T, String> {
    let args = if args.is_null() { Value::Object(Default::default()) } else { args };
    serde_json::from_value(args).map_err(|e| format!("invalid arguments: {}", e))
}

pub(crate) fn reply<T: Serialize>(result: Result<T, String>) -> Reply {
    result.and_then(|value| serde_json::to_value(value).map_err(|e| e.to_string()))
}

pub(crate) fn plain<T: Serialize>(value: T) -> Reply {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

pub struct CoreBackend {
    cx: Cx,
}

impl CoreBackend {
    pub fn new(cx: Cx) -> Self {
        Self { cx }
    }

    pub fn cx(&self) -> &Cx {
        &self.cx
    }
}

impl Backend for CoreBackend {
    fn invoke(&self, name: &str, args: Value) -> BoxFuture<Reply> {
        let cx = self.cx.clone();
        let name = name.to_string();
        Box::pin(async move { commands::dispatch(&cx, &name, args).await })
    }
}
