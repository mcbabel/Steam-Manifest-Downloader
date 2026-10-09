use dryoc::dryocbox::{DryocBox, PublicKey};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
const SCHEMA_VERSION: u32 = 3;
const FLUSH_INTERVAL: Duration = Duration::from_secs(300);
const BUFFER_FLUSH_AT: usize = 20;
const QUEUE_FILE: &str = "telemetry_queue.json";
const QUEUE_CAP: usize = 200;
const REPLAY_BATCH: usize = 25;
const CRASH_FILE: &str = "telemetry_crash.json";
const CRASH_CAP: usize = 5;
const ACTIVE_DIR: &str = "telemetry_active";
const HEARTBEAT_EVERY: Duration = Duration::from_secs(15 * 60);

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
    frontend: &'static str,
    locale: String,
    started: std::time::Instant,
    focused_since: Option<std::time::Instant>,
    focused_total: Duration,
    tracks_focus: bool,
    last_heartbeat: std::time::Instant,
    ended: bool,
}

impl Inner {
    fn session_props(&self, app_data_dir: &Path) -> serde_json::Value {
        let open = self.started.elapsed();
        let active = self.focused_total
            + self.focused_since.map(|t| t.elapsed()).unwrap_or_default();
        let mut props = serde_json::json!({
            "uptime_min": open.as_secs() / 60,
            "downloading": downloads_running(app_data_dir),
        });
        if self.tracks_focus {
            props["active_min"] = serde_json::json!(active.min(open).as_secs() / 60);
        }
        props
    }
}

fn downloads_running(app_data_dir: &Path) -> bool {
    std::fs::read_dir(app_data_dir.join(ACTIVE_DIR))
        .map(|mut d| d.next().is_some())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueuedBatch {
    session_id: String,
    app_version: String,
    events: Vec<Event>,
}

impl Telemetry {
    pub fn new(
        app_data_dir: PathBuf,
        app_version: String,
        channel: String,
        frontend: &'static str,
    ) -> Self {
        let inner = Inner {
            buffer: Vec::new(),
            session_id: Uuid::new_v4().to_string(),
            app_version,
            channel,
            frontend,
            locale: String::new(),
            started: std::time::Instant::now(),
            focused_since: None,
            focused_total: Duration::ZERO,
            tracks_focus: false,
            last_heartbeat: std::time::Instant::now(),
            ended: false,
        };
        Self {
            inner: Arc::new(Mutex::new(inner)),
            http: crate::services::net::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            app_data_dir,
        }
    }

    pub async fn set_focused(&self, focused: bool) {
        let mut inner = self.inner.lock().await;
        inner.tracks_focus = true;
        match (focused, inner.focused_since) {
            (true, None) => inner.focused_since = Some(std::time::Instant::now()),
            (false, Some(since)) => {
                inner.focused_total += since.elapsed();
                inner.focused_since = None;
            }
            _ => {}
        }
    }

    pub async fn end_session(&self, reason: &str) {
        let props = {
            let mut inner = self.inner.lock().await;
            if inner.ended {
                None
            } else {
                inner.ended = true;
                let mut p = inner.session_props(&self.app_data_dir);
                p["reason"] = serde_json::json!(safe_label(reason));
                Some(p)
            }
        };
        if let Some(p) = props {
            self.emit(Event::new("session_end").with_props(p)).await;
        }
        self.flush().await;
    }

    async fn heartbeat_if_due(&self) {
        let props = {
            let mut inner = self.inner.lock().await;
            if inner.ended || inner.last_heartbeat.elapsed() < HEARTBEAT_EVERY {
                return;
            }
            inner.last_heartbeat = std::time::Instant::now();
            inner.session_props(&self.app_data_dir)
        };
        self.emit(Event::new("heartbeat").with_props(props)).await;
    }

    pub async fn set_locale(&self, locale: &str) {
        let clean: String = locale
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(8)
            .collect();
        self.inner.lock().await.locale = clean.to_ascii_lowercase();
    }

    pub async fn emit(&self, mut event: Event) {
        let settings = settings_service::load_settings(&self.app_data_dir).await;
        if !consented(&settings) {
            return;
        }
        let mut extra: Vec<Event> = Vec::new();
        if event.kind == "app_start" {
            let mut props = match std::mem::take(&mut event.props) {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            if let Some(locale) = props.remove("locale") {
                self.set_locale(locale.as_str().unwrap_or("")).await;
            }
            props.insert("settings".to_string(), settings_snapshot(&settings));
            event.props = serde_json::Value::Object(props);
            extra.extend(
                take_crashes(&self.app_data_dir)
                    .into_iter()
                    .map(|c| Event::new("crash").with_props(c)),
            );
            extra.extend(
                take_interrupted(&self.app_data_dir)
                    .into_iter()
                    .map(|p| Event::new("download_interrupted").with_props(p)),
            );
        }
        let mut inner = self.inner.lock().await;
        inner.buffer.push(event);
        inner.buffer.extend(extra);
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
            self.heartbeat_if_due().await;
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

    pub async fn is_enabled(&self) -> bool {
        consented(&settings_service::load_settings(&self.app_data_dir).await)
    }

    async fn build_payload(
        &self,
        events: &[Event],
        replay: Option<(&str, &str)>,
    ) -> serde_json::Value {
        let install_id = ensure_installation_id(&self.app_data_dir).await;
        let inner = self.inner.lock().await;
        let (session_id, app_version) = replay.unwrap_or((&inner.session_id, &inner.app_version));
        let mut payload = serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "installation_id": install_id,
            "session_id": session_id,
            "app_version": app_version,
            "channel": inner.channel,
            "frontend": inner.frontend,
            "package": package_kind(),
            "locale": inner.locale,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "events": events,
        });
        if replay.is_some() {
            payload["replay"] = serde_json::json!(true);
        }
        payload
    }

    async fn send_batch(&self, events: Vec<Event>) {
        let payload = self.build_payload(&events, None).await;

        if self.send_payload(&payload).await.is_err() {
            let (session_id, app_version) = {
                let inner = self.inner.lock().await;
                (inner.session_id.clone(), inner.app_version.clone())
            };
            let _ = self
                .queue_payload(QueuedBatch {
                    session_id,
                    app_version,
                    events,
                })
                .await;
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

    fn queue_path(&self) -> PathBuf {
        self.app_data_dir.join(QUEUE_FILE)
    }

    async fn read_queue(&self) -> Vec<QueuedBatch> {
        let Ok(bytes) = fs::read(self.queue_path()).await else {
            return Vec::new();
        };
        parse_queue(&bytes)
    }

    async fn write_queue(&self, batches: &[QueuedBatch]) -> Result<(), ()> {
        let path = self.queue_path();
        if batches.iter().all(|b| b.events.is_empty()) {
            let _ = fs::remove_file(&path).await;
            return Ok(());
        }
        let serialized = serde_json::to_vec(batches).map_err(|_| ())?;
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }
        fs::write(&path, serialized).await.map_err(|_| ())
    }

    async fn queue_payload(&self, batch: QueuedBatch) -> Result<(), ()> {
        let mut queued = self.read_queue().await;
        match queued
            .iter_mut()
            .find(|b| b.session_id == batch.session_id && b.app_version == batch.app_version)
        {
            Some(existing) => existing.events.extend(batch.events),
            None => queued.push(batch),
        }
        cap_queue(&mut queued, QUEUE_CAP);
        self.write_queue(&queued).await
    }

    async fn retry_queue(&self) {
        let mut queued = self.read_queue().await;
        if queued.is_empty() {
            return;
        }
        while let Some(batch) = queued.first_mut() {
            while !batch.events.is_empty() {
                let take = batch.events.len().min(REPLAY_BATCH);
                let chunk: Vec<Event> = batch.events[..take].to_vec();
                let payload = self
                    .build_payload(&chunk, Some((&batch.session_id, &batch.app_version)))
                    .await;
                if self.send_payload(&payload).await.is_err() {
                    let _ = self.write_queue(&queued).await;
                    return;
                }
                batch.events.drain(..take);
            }
            queued.remove(0);
        }
        let _ = self.write_queue(&queued).await;
    }
}

fn parse_queue(bytes: &[u8]) -> Vec<QueuedBatch> {
    if let Ok(batches) = serde_json::from_slice::<Vec<QueuedBatch>>(bytes) {
        return batches;
    }
    match serde_json::from_slice::<Vec<Event>>(bytes) {
        Ok(events) if !events.is_empty() => vec![QueuedBatch {
            session_id: String::new(),
            app_version: String::new(),
            events,
        }],
        _ => Vec::new(),
    }
}

fn cap_queue(batches: &mut Vec<QueuedBatch>, cap: usize) {
    let mut total: usize = batches.iter().map(|b| b.events.len()).sum();
    while total > cap {
        let Some(first) = batches.first_mut() else {
            break;
        };
        let drop = first.events.len().min(total - cap);
        first.events.drain(..drop);
        total -= drop;
        if first.events.is_empty() {
            batches.remove(0);
        }
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
    "crash",
    "error_shown",
    "download_interrupted",
    "library_added",
    "game_launched",
    "proxy_tested",
    "diagnostics_copied",
    "bug_report_opened",
    "download_paused",
    "shutdown_after",
    "followup",
    "queue_action",
    "steamless_used",
    "api_bypass",
    "history_action",
    "updates_found",
    "manifest_tool",
    "dlc_merged",
    "shortcut_key",
    "settings_saved",
    "cli_command",
    "heartbeat",
    "session_end",
    "game_data_written",
];

pub fn env_consent() -> Option<bool> {
    let value = std::env::var("SMD_TELEMETRY").ok()?;
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "on" | "true" | "yes" => Some(true),
        "0" | "off" | "false" | "no" => Some(false),
        _ => None,
    }
}

fn consented(settings: &settings_service::Settings) -> bool {
    env_consent().unwrap_or(settings.telemetry_consent == TelemetryConsent::Accepted)
}

pub fn is_safe_kind(kind: &str) -> bool {
    ALLOWED_EVENT_KINDS.contains(&kind)
}

pub fn sanitize_props(kind: &str, props: serde_json::Value) -> serde_json::Value {
    let serde_json::Value::Object(mut map) = props else {
        return props;
    };
    if let Some(serde_json::Value::String(key)) = map.get("err_key") {
        let clean = safe_label(key);
        map.insert("err_key".to_string(), serde_json::Value::String(clean));
    }
    match kind {
        "crash" => {
            for field in ["message", "location"] {
                if let Some(serde_json::Value::String(text)) = map.get(field) {
                    let clean = if field == "message" {
                        sanitize_message(text)
                    } else {
                        sanitize_location(text)
                    };
                    map.insert(field.to_string(), serde_json::Value::String(clean));
                }
            }
        }
        "error_shown" => {
            for field in ["key", "area"] {
                let clean = map
                    .get(field)
                    .and_then(|v| v.as_str())
                    .map(safe_label)
                    .unwrap_or_else(|| "unknown".to_string());
                map.insert(field.to_string(), serde_json::Value::String(clean));
            }
        }
        "settings_saved" => {
            let keys: Vec<serde_json::Value> = map
                .get("keys")
                .and_then(|v| v.as_array())
                .map(|list| {
                    list.iter()
                        .filter_map(|k| k.as_str())
                        .map(safe_label)
                        .filter(|k| k != "unmatched")
                        .take(32)
                        .map(serde_json::Value::String)
                        .collect()
                })
                .unwrap_or_default();
            map.insert("keys".to_string(), serde_json::Value::Array(keys));
        }
        "proxy_tested" => {
            if let Some(serde_json::Value::String(key)) = map.get("key") {
                let clean = safe_label(key);
                map.insert("key".to_string(), serde_json::Value::String(clean));
            }
        }
        _ => {}
    }
    serde_json::Value::Object(map)
}

fn safe_label(text: &str) -> String {
    let ok = !text.is_empty()
        && text.len() <= 64
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
    if ok {
        text.to_string()
    } else {
        "unmatched".to_string()
    }
}

pub fn sanitize_message(text: &str) -> String {
    let line = text.lines().next().unwrap_or("");
    let mut out = String::new();
    let mut in_quotes = false;
    for c in line.chars() {
        if c == '"' || c == '\'' || c == '`' {
            if !in_quotes {
                out.push(c);
                out.push('_');
            } else {
                out.push(c);
            }
            in_quotes = !in_quotes;
            continue;
        }
        if !in_quotes {
            out.push(c);
        }
    }
    let words: Vec<String> = out
        .split_whitespace()
        .map(|w| {
            if w.contains('/') || w.contains('\\') || w.contains(":\\") || w.contains('@') {
                "<path>".to_string()
            } else if w.chars().any(|c| c.is_ascii_digit()) {
                let mut s = String::new();
                let mut last_digit = false;
                for c in w.chars() {
                    if c.is_ascii_digit() {
                        if !last_digit {
                            s.push('N');
                        }
                        last_digit = true;
                    } else {
                        s.push(c);
                        last_digit = false;
                    }
                }
                s
            } else {
                w.to_string()
            }
        })
        .collect();
    words.join(" ").chars().take(160).collect()
}

pub fn sanitize_location(text: &str) -> String {
    let normalized = text.replace('\\', "/");
    let mut path = normalized.as_str();
    let mut numbers: Vec<&str> = Vec::new();
    while numbers.len() < 2 {
        match path.rsplit_once(':') {
            Some((p, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => {
                numbers.push(n);
                path = p;
            }
            _ => break,
        }
    }
    let line = numbers.last().map(|n| n.to_string());
    let short = ["src-core/", "src-tauri/", "src-tui/", "public/js/"]
        .iter()
        .find_map(|root| path.find(root).map(|i| path[i..].to_string()))
        .or_else(|| {
            path.find("/registry/src/").and_then(|i| {
                let rest = &path[i + "/registry/src/".len()..];
                rest.split_once('/').map(|(_, crate_path)| crate_path.to_string())
            })
        })
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or("").to_string());
    let short: String = short
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '.' | '-'))
        .take(96)
        .collect();
    match line {
        Some(l) => format!("{}:{}", short, l),
        None => short,
    }
}

pub fn proxy_kind(proxy: &str) -> &'static str {
    let p = proxy.trim().to_ascii_lowercase();
    if p.is_empty() {
        "none"
    } else if p.starts_with("socks") {
        "socks"
    } else if p.starts_with("https") {
        "https"
    } else {
        "http"
    }
}

pub fn package_kind() -> &'static str {
    if std::env::var_os("SMD_OUTPUT_DIR").is_some() || Path::new("/.dockerenv").exists() {
        return "docker";
    }
    if cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some() {
        return "appimage";
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    if cfg!(target_os = "windows") {
        return match exe_dir {
            Some(dir) if dir.join("third-party").is_dir() => "portable",
            Some(dir) if dir.join("uninstall.exe").is_file() => "installer",
            _ => "exe",
        };
    }
    match crate::ops::updater::detect_install_method() {
        "flatpak" => "flatpak",
        "snap" => "snap",
        "system" => "system",
        _ => "binary",
    }
}

pub fn install_crash_hook(app_data_dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let thread = if std::thread::current().name() == Some("main") {
            "main"
        } else {
            "worker"
        };
        record_crash(
            &app_data_dir,
            serde_json::json!({
                "source": "panic",
                "location": sanitize_location(&location),
                "message": sanitize_message(&message),
                "thread": thread,
            }),
        );
        previous(info);
    }));
}

fn record_crash(app_data_dir: &Path, record: serde_json::Value) {
    let path = app_data_dir.join(CRASH_FILE);
    let mut list: Vec<serde_json::Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if list.len() >= CRASH_CAP {
        return;
    }
    list.push(record);
    if let Ok(bytes) = serde_json::to_vec(&list) {
        let _ = std::fs::write(&path, bytes);
    }
}

fn take_crashes(app_data_dir: &Path) -> Vec<serde_json::Value> {
    let path = app_data_dir.join(CRASH_FILE);
    let list: Vec<serde_json::Value> = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    list
}

pub struct ActiveDownload {
    path: PathBuf,
    props: serde_json::Value,
}

impl ActiveDownload {
    pub fn start(app_data_dir: &Path, job_id: &str, props: serde_json::Value) -> Self {
        let dir = app_data_dir.join(ACTIVE_DIR);
        let _ = std::fs::create_dir_all(&dir);
        let name: String = job_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .take(64)
            .collect();
        let marker = Self {
            path: dir.join(format!("{}.json", name)),
            props,
        };
        marker.write();
        marker
    }

    pub fn step(&mut self, step: &str) {
        self.props["step"] = serde_json::json!(step);
        self.write();
    }

    fn write(&self) {
        if let Ok(bytes) = serde_json::to_vec(&self.props) {
            let _ = std::fs::write(&self.path, bytes);
        }
    }
}

impl Drop for ActiveDownload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn clear_active_downloads(app_data_dir: &Path) {
    let _ = std::fs::remove_dir_all(app_data_dir.join(ACTIVE_DIR));
}

fn take_interrupted(app_data_dir: &Path) -> Vec<serde_json::Value> {
    let dir = app_data_dir.join(ACTIVE_DIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(props) = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        {
            out.push(props);
        }
        let _ = std::fs::remove_file(&path);
    }
    out.truncate(CRASH_CAP);
    out
}


pub async fn ensure_installation_id(app_data_dir: &std::path::Path) -> String {
    let mut settings = settings_service::load_settings(app_data_dir).await;
    if settings.installation_id.is_empty() {
        settings.installation_id = Uuid::new_v4().to_string();
        let _ = settings_service::save_settings(app_data_dir, &settings).await;
    }
    settings.installation_id
}

fn settings_snapshot(s: &settings_service::Settings) -> serde_json::Value {
    let limit = s.download_speed_limit.trim();
    serde_json::json!({
        "engine": if s.use_native_downloader { "native" } else { "ddm" },
        "like_steam": s.auto_select_depots,
        "auto_start": s.auto_start_download,
        "include_dlc": s.include_dlc,
        "proxy": proxy_kind(&s.proxy),
        "speed_limit": !limit.is_empty() && limit != "0",
        "sources": crate::services::diag::count_bucket(s.depot_sources.len()),
        "custom_sources": s.depot_sources != s.pristine_default_sources,
        "hubcap_key": !s.hubcap_api_key.trim().is_empty(),
        "ryuu_key": !s.ryuu_api_key.trim().is_empty(),
        "steam_web_api_key": !s.steam_web_api_key.trim().is_empty(),
        "game_data_media": if s.game_data_media.is_empty() { "off" } else { s.game_data_media.as_str() },
        "auto_update": s.auto_update,
        "steam_path_set": !s.steam_path.trim().is_empty(),
        "max_retries": s.max_retries.min(20),
        "chunk_concurrency": s.native_chunk_concurrency.min(64),
        "keep_files_on_cancel": s.cancel_keep_files,
        "game_language": game_language_label(&s.game_language),
        "target_platform": if crate::services::depot_select::PLATFORMS.contains(&s.target_platform.as_str()) { s.target_platform.as_str() } else { "auto" },
    })
}

fn game_language_label(value: &str) -> &'static str {
    let v = value.trim();
    if v.is_empty() {
        return "auto";
    }
    crate::services::depot_select::STEAM_LANGUAGES
        .iter()
        .find(|l| **l == v)
        .copied()
        .unwrap_or("other")
}

pub fn diagnostic_id(installation_id: &str) -> String {
    installation_id
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(8)
        .collect::<String>()
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_messages_lose_paths_quotes_and_numbers() {
        let msg = sanitize_message(
            "called `Result::unwrap()` on an `Err` value: Os { code: 2, message: \"No such file\" } at C:\\Users\\bob\\x.txt\nsecond line",
        );
        assert!(!msg.contains("bob"));
        assert!(!msg.contains("No such file"));
        assert!(!msg.contains("second"));
        assert!(msg.contains("code: N,"));
    }

    #[test]
    fn crash_locations_keep_only_the_project_path() {
        assert_eq!(
            sanitize_location("/home/bob/build/src-core/src/ops/download.rs:812"),
            "src-core/src/ops/download.rs:812"
        );
        assert_eq!(
            sanitize_location("C:\\Users\\bob\\.cargo\\registry\\src\\index.crates.io-6f17d22bba15001f\\tokio-1.40.0\\src\\rt.rs:55"),
            "tokio-1.40.0/src/rt.rs:55"
        );
        assert_eq!(
            sanitize_location("tauri://localhost/js/app.js:1234:5"),
            "app.js:1234"
        );
    }

    #[test]
    fn error_keys_are_plain_labels() {
        let props = sanitize_props(
            "error_shown",
            serde_json::json!({ "key": "backend.noSources", "area": "C:/Users/bob" }),
        );
        assert_eq!(props["key"], "backend.noSources");
        assert_eq!(props["area"], "unmatched");
    }

    #[test]
    fn old_queues_are_still_read_and_the_cap_drops_the_oldest() {
        let legacy = serde_json::to_vec(&vec![Event::new("app_start"), Event::new("search_performed")]).unwrap();
        let mut batches = parse_queue(&legacy);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].events.len(), 2);
        batches.push(QueuedBatch {
            session_id: "b".into(),
            app_version: "1".into(),
            events: vec![Event::new("download_started")],
        });
        cap_queue(&mut batches, 2);
        assert_eq!(batches.iter().map(|b| b.events.len()).sum::<usize>(), 2);
        assert_eq!(batches[0].events[0].kind, "search_performed");
    }

    #[test]
    fn interrupted_downloads_are_reported_once() {
        let dir = std::env::temp_dir().join(format!("smd-tel-active-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut marker = ActiveDownload::start(&dir, "job1", serde_json::json!({ "engine": "native" }));
        marker.step("download");
        std::mem::forget(marker);
        let found = take_interrupted(&dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["step"], "download");
        assert!(take_interrupted(&dir).is_empty());
        {
            let _done = ActiveDownload::start(&dir, "job2", serde_json::json!({}));
        }
        assert!(take_interrupted(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn sessions_report_open_and_focused_minutes() {
        let dir = std::env::temp_dir().join(format!("smd-tel-session-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let tel = Telemetry::new(dir.clone(), "1.0.0".into(), "dev-local".into(), "gui");
        {
            let mut inner = tel.inner.lock().await;
            inner.started -= Duration::from_secs(30 * 60);
        }
        tel.set_focused(true).await;
        {
            let mut inner = tel.inner.lock().await;
            inner.focused_since = inner.focused_since.map(|t| t - Duration::from_secs(10 * 60));
        }
        tel.set_focused(false).await;
        let props = tel.inner.lock().await.session_props(&dir);
        assert_eq!(props["uptime_min"], 30);
        assert_eq!(props["active_min"], 10);
        assert_eq!(props["downloading"], false);
        let cli = Telemetry::new(dir.clone(), "1.0.0".into(), "dev-local".into(), "cli");
        assert!(cli.inner.lock().await.session_props(&dir).get("active_min").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diagnostic_id_is_short_and_upper_case() {
        assert_eq!(diagnostic_id("3f2a9c1e-77aa-4b4b-9c3d-000000000000"), "3F2A9C1E");
    }
}
