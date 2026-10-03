use dryoc::dryocbox::{DryocBox, PublicKey};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::services::settings::{self as settings_service, TelemetryConsent};

// Rotate: generate a fresh X25519 keypair, replace these bytes, ship the new
// private half to the server. Old clients keep sending against the old pubkey
// until they update.
const SERVER_PUBLIC_KEY: [u8; 32] = [
    0x78, 0xc6, 0xed, 0x23, 0xd8, 0x2f, 0x86, 0xab, 0x49, 0xf8, 0x08, 0xe7,
    0x9b, 0x75, 0x5c, 0xf8, 0x55, 0xb2, 0x28, 0x4f, 0x34, 0x4e, 0x62, 0xde,
    0xc8, 0x16, 0xfa, 0x49, 0x41, 0x89, 0x7b, 0x72,
];

const ENDPOINT_URL: &str = "https://analytics-smd.mcbabel.de/v1/events";
const SCHEMA_VERSION: u32 = 2;
const FLUSH_INTERVAL: Duration = Duration::from_secs(300);
const BUFFER_FLUSH_AT: usize = 20;
const QUEUE_FILE: &str = "telemetry_queue.json";
const QUEUE_CAP: usize = 200;
const REPLAY_BATCH: usize = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: String,
    pub ts: i64,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub props: serde_json::Value,
}

impl Event {
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            ts: chrono::Utc::now().timestamp(),
            props: serde_json::Value::Null,
        }
    }

    pub fn with_props(mut self, props: serde_json::Value) -> Self {
        self.props = props;
        self
    }
}

#[derive(Clone)]
pub struct Telemetry {
    inner: Arc<Mutex<Inner>>,
    http: reqwest::Client,
    app_data_dir: PathBuf,
}

struct Inner {
    buffer: Vec<Event>,
    session_id: String,
    app_version: String,
    channel: String,
}

impl Telemetry {
    pub fn new(app_data_dir: PathBuf, app_version: String, channel: String) -> Self {
        let inner = Inner {
            buffer: Vec::new(),
            session_id: Uuid::new_v4().to_string(),
            app_version,
            channel,
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            app_data_dir,
        }
    }

    pub async fn emit(&self, event: Event) {
        if !self.is_enabled().await {
            return;
        }
        let mut inner = self.inner.lock().await;
        inner.buffer.push(event);
        if inner.buffer.len() >= BUFFER_FLUSH_AT {
            let drained: Vec<Event> = inner.buffer.drain(..).collect();
            drop(inner);
            self.send_batch(drained).await;
        }
    }

    pub async fn run_background_flush(self) {
        let mut interval = tokio::time::interval(FLUSH_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            self.flush().await;
        }
    }

    pub async fn flush(&self) {
        if !self.is_enabled().await {
            return;
        }
        let drained: Vec<Event> = {
            let mut inner = self.inner.lock().await;
            inner.buffer.drain(..).collect()
        };
        if !drained.is_empty() {
            self.send_batch(drained).await;
        }
        self.retry_queue().await;
    }

    async fn is_enabled(&self) -> bool {
        let s = settings_service::load_settings(&self.app_data_dir).await;
        s.telemetry_consent == TelemetryConsent::Accepted
    }

    async fn build_payload(&self, events: &[Event], replay: bool) -> serde_json::Value {
        let install_id = ensure_installation_id(&self.app_data_dir).await;
        let inner = self.inner.lock().await;
        let mut payload = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "installation_id": install_id,
            "session_id": inner.session_id,
            "app_version": inner.app_version,
            "channel": inner.channel,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "events": events,
        });
        if replay {
            payload["replay"] = serde_json::json!(true);
        }
        payload
    }

    async fn send_batch(&self, events: Vec<Event>) {
        let payload = self.build_payload(&events, false).await;

        match self.send_payload(&payload).await {
            Ok(()) => {}
            Err(_) => {
                let _ = self.queue_payload(&events).await;
            }
        }
    }

    async fn send_payload(&self, payload: &serde_json::Value) -> Result<(), ()> {
        let body = serde_json::to_vec(payload).map_err(|_| ())?;
        let pubkey = PublicKey::from(SERVER_PUBLIC_KEY);
        let sealed = DryocBox::seal_to_vecbox(&body, &pubkey).map_err(|_| ())?;
        let ciphertext = sealed.to_vec();

        let resp = self
            .http
            .post(ENDPOINT_URL)
            .header("Content-Type", "application/octet-stream")
            .body(ciphertext)
            .send()
            .await
            .map_err(|_| ())?;
        if !resp.status().is_success() {
            return Err(());
        }
        Ok(())
    }

    async fn queue_path(&self) -> PathBuf {
        self.app_data_dir.join(QUEUE_FILE)
    }

    async fn queue_payload(&self, events: &[Event]) -> Result<(), ()> {
        let path = self.queue_path().await;
        let mut queued: Vec<Event> = match fs::read(&path).await {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        queued.extend_from_slice(events);
        if queued.len() > QUEUE_CAP {
            let drop = queued.len() - QUEUE_CAP;
            queued.drain(0..drop);
        }
        let serialized = serde_json::to_vec(&queued).map_err(|_| ())?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }
        fs::write(&path, serialized).await.map_err(|_| ())
    }

    async fn retry_queue(&self) {
        let path = self.queue_path().await;
        let bytes = match fs::read(&path).await {
            Ok(b) => b,
            Err(_) => return,
        };
        let queued: Vec<Event> = match serde_json::from_slice::<Vec<Event>>(&bytes) {
            Ok(q) if !q.is_empty() => q,
            _ => return,
        };

        let mut remaining = queued;
        while !remaining.is_empty() {
            let take = remaining.len().min(REPLAY_BATCH);
            let batch: Vec<Event> = remaining.drain(..take).collect();
            let payload = self.build_payload(&batch, true).await;
            if self.send_payload(&payload).await.is_err() {
                let mut unsent = batch;
                unsent.extend(remaining);
                if let Ok(serialized) = serde_json::to_vec(&unsent) {
                    let _ = fs::write(&path, serialized).await;
                }
                return;
            }
        }
        let _ = fs::remove_file(&path).await;
    }
}

// Allowlist enforced at the command boundary so the JS layer can't invent
// new event kinds and break the server-side schema.
const ALLOWED_EVENT_KINDS: &[&str] = &[
    "app_start",
    "settings_opened",
    "theme_toggled",
    "search_performed",
    "lua_parsed",
    "download_started",
    "download_completed",
    "download_abandoned",
    "patch_applied",
    "patch_reverted",
    "patch_settings_saved",
    "shortcut_created",
    "update_checked",
    "update_installed",
    "consent_accepted",
];

pub fn is_safe_kind(kind: &str) -> bool {
    ALLOWED_EVENT_KINDS.contains(&kind)
}


pub async fn ensure_installation_id(app_data_dir: &std::path::Path) -> String {
    let mut settings = settings_service::load_settings(app_data_dir).await;
    if settings.installation_id.is_empty() {
        settings.installation_id = Uuid::new_v4().to_string();
        let _ = settings_service::save_settings(app_data_dir, &settings).await;
    }
    settings.installation_id
}