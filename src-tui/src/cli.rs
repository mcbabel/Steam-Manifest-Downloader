use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Args;
use serde_json::Value;
use smd_core::ops::download::{DepotConfig, DownloadConfig};
use smd_core::services::events::{EventSink, DOWNLOAD_PROGRESS};
use smd_core::services::history::HistoryEntry;
use smd_core::services::settings as settings_service;
use smd_core::services::AppState;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Notify;

#[derive(Args, Debug)]
pub struct DownloadArgs {
    pub source: String,
    #[arg(long, value_delimiter = ',')]
    pub depots: Vec<String>,
    #[arg(long, short, env = "SMD_OUTPUT_DIR")]
    pub out: Option<PathBuf>,
    #[arg(long, env = "SMD_MANIFESTHUB_KEY", hide_env_values = true)]
    pub mh_key: Option<String>,
    #[arg(long, default_value_t = 0)]
    pub result: usize,
    #[arg(long = "manifest", value_name = "DEPOT=MANIFEST")]
    pub manifests: Vec<String>,
    #[arg(long)]
    pub list: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long)]
    pub manifests: Option<usize>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(long)]
    pub json: bool,
}

macro_rules! errln {
    ($($arg:tt)*) => {
        crate::term::write_original_stderr(&format!("{}\n", format!($($arg)*)))
    };
}

struct ChannelSink(UnboundedSender<Value>);

impl EventSink for ChannelSink {
    fn emit(&self, channel: &str, payload: Value) {
        if channel == DOWNLOAD_PROGRESS {
            let _ = self.0.send(payload);
        }
    }
}

fn is_numeric(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

type PlannedDepot = (String, Option<String>, Option<String>, Option<u64>);

struct Plan {
    app_id: String,
    depots: Vec<PlannedDepot>,
    key_vdf: Option<HashMap<String, String>>,
    source_type: Option<String>,
    repo: Option<String>,
    game_name: Option<String>,
    header_image: Option<String>,
}

async fn plan_from_source(
    core: &AppState,
    dir: &std::path::Path,
    args: &DownloadArgs,
) -> Result<Plan, String> {
    if is_numeric(&args.source) {
        let res = smd_core::ops::search::search_repos(core, dir, &args.source).await?;
        let repo = res.repos.get(args.result).cloned().ok_or_else(|| {
            format!(
                "App {} not found in any configured depot source (result #{})",
                args.source, args.result
            )
        })?;
        let m = smd_core::ops::search::get_repo_manifests(
            core,
            dir,
            &args.source,
            &repo.repo,
            repo.sha.as_deref(),
        )
        .await?;
        let info = smd_core::services::steam_store_api::get_game_info(
            &core.http_client,
            &core.steam_cache,
            &args.source,
        )
        .await
        .ok()
        .flatten();
        Ok(Plan {
            app_id: args.source.clone(),
            depots: m
                .manifests
                .into_iter()
                .map(|x| (x.depot_id, Some(x.manifest_id), x.depot_key, x.size_bytes))
                .collect(),
            key_vdf: Some(m.depot_keys),
            source_type: Some(repo.source_type),
            repo: Some(repo.repo),
            game_name: info.as_ref().and_then(|i| i.name.clone()),
            header_image: info.and_then(|i| i.header_image),
        })
    } else {
        let path = crate::ui::file_browser::expand_tilde(&args.source);
        let parsed = smd_core::ops::files::parse_source_file(&path.to_string_lossy()).await?;
        let app_id = parsed
            .main_app_id
            .map(|i| i.to_string())
            .ok_or_else(|| "The file does not name a main App ID".to_string())?;
        Ok(Plan {
            app_id,
            depots: parsed
                .depots
                .into_iter()
                .map(|d| {
                    (
                        d.depot_id.to_string(),
                        d.manifest_id,
                        d.depot_key,
                        d.size_bytes,
                    )
                })
                .collect(),
            key_vdf: None,
            source_type: None,
            repo: None,
            game_name: None,
            header_image: None,
        })
    }
}

fn fmt_bytes(b: u64) -> String {
    crate::ui::widgets::fmt_bytes(b)
}

static STOP: AtomicBool = AtomicBool::new(false);
static STOP_NOTIFY: Notify = Notify::const_new();

pub fn listen_for_shutdown() {
    tokio::spawn(async {
        shutdown_signal().await;
        STOP.store(true, Ordering::SeqCst);
        STOP_NOTIFY.notify_waiters();
    });
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut term) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

pub async fn stopped() {
    let notified = STOP_NOTIFY.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();
    if STOP.load(Ordering::SeqCst) {
        return;
    }
    notified.await;
}

pub async fn download(dir: PathBuf, args: DownloadArgs) -> i32 {
    let _ = tokio::fs::create_dir_all(&dir).await;
    let settings = settings_service::seed_defaults_if_needed(&dir).await;
    let core = AppState::new();
    let planned = tokio::select! {
        r = plan_from_source(&core, &dir, &args) => r,
        _ = stopped() => return 130,
    };
    let mut plan = match planned {
        Ok(p) => p,
        Err(e) => {
            errln!("error: {}", e);
            return 1;
        }
    };
    if plan.game_name.is_none() {
        let info = tokio::select! {
            r = smd_core::services::steam_store_api::get_game_info(
                &core.http_client,
                &core.steam_cache,
                &plan.app_id,
            ) => r,
            _ = stopped() => return 130,
        };
        if let Ok(Some(info)) = info {
            plan.game_name = info.name;
            plan.header_image = info.header_image;
        }
    }
    let wanted: Vec<String> = args
        .depots
        .iter()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
        .collect();
    let pins: HashMap<String, String> = args
        .manifests
        .iter()
        .filter_map(|m| {
            m.split_once('=')
                .map(|(d, id)| (d.trim().to_string(), id.trim().to_string()))
        })
        .collect();
    let chosen: Vec<_> = plan
        .depots
        .iter()
        .filter(|(id, ..)| wanted.is_empty() || wanted.contains(id))
        .cloned()
        .collect();

    let title = match &plan.game_name {
        Some(name) => format!("{} (App {})", name, plan.app_id),
        None => format!("App {}", plan.app_id),
    };
    println!(
        "{} — {} of {} depot(s)",
        title,
        chosen.len(),
        plan.depots.len()
    );
    for (id, manifest, key, size) in &chosen {
        println!(
            "  {:>10}  manifest {}{}{}",
            id,
            pins.get(id)
                .or(manifest.as_ref())
                .map(String::as_str)
                .unwrap_or("N/A"),
            size.map(|s| format!("  {}", fmt_bytes(s)))
                .unwrap_or_default(),
            if key.is_none() {
                "  (no key in file)"
            } else {
                ""
            }
        );
    }
    if args.list {
        return 0;
    }
    if chosen.is_empty() {
        errln!("error: none of the requested depots exist");
        return 1;
    }

    let config = DownloadConfig {
        app_id: plan.app_id.clone(),
        game_name: plan.game_name.clone(),
        depots: chosen
            .iter()
            .map(|(id, manifest, key, _)| DepotConfig {
                depot_id: id.clone(),
                manifest_id: manifest.clone().unwrap_or_else(|| "N/A".into()),
                custom_manifest_id: pins.get(id).cloned(),
                depot_key: key.clone(),
                uploaded_manifest_path: None,
                display_name: None,
            })
            .collect(),
        mode: Some(
            if plan.repo.is_some() {
                "search"
            } else {
                "upload"
            }
            .into(),
        ),
        key_vdf_keys: plan.key_vdf.clone(),
        download_location: Some(
            args.out
                .clone()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(settings.download_location.clone()),
        ),
        manifest_hub_api_key: args.mh_key.clone().filter(|k| !k.is_empty()),
        header_image: plan.header_image.clone(),
        source_type: plan.source_type.clone(),
    };
    let depot_ids: Vec<String> = config.depots.iter().map(|d| d.depot_id.clone()).collect();

    let (tx, mut rx) = unbounded_channel();
    let sink: smd_core::services::events::Sink = Arc::new(ChannelSink(tx));
    let core = Arc::new(core);
    let started_at = chrono::Utc::now().to_rfc3339();
    let started =
        match smd_core::ops::download::start_download(sink.clone(), &core, &dir, config).await {
            Ok(v) => v,
            Err(e) => {
                errln!("error: {}", e);
                return 1;
            }
        };
    let job = started["jobId"].as_str().unwrap_or_default().to_string();
    let work_dir = started["downloadDir"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    println!("→ {}", work_dir);

    let tty = std::io::stdout().is_terminal() && !args.json;
    let mut printer = Printer::new(tty);
    let mut stop = Box::pin(stopped());
    let mut interrupted = false;
    let code = loop {
        tokio::select! {
            ev = rx.recv() => {
                let Some(ev) = ev else { break 1 };
                if args.json {
                    println!("{}", ev);
                    let _ = std::io::stdout().flush();
                } else {
                    printer.event(&ev);
                }
                match ev["type"].as_str() {
                    Some("complete") => {
                        let outcome = ev["diag"]["outcome"].as_str().unwrap_or("complete");
                        let results = ev["results"].as_array().cloned().unwrap_or_default();
                        record_history(&dir, &plan, &work_dir, &started_at, &results, &depot_ids).await;
                        break match outcome {
                            "complete" => 0,
                            "partial" => 2,
                            _ => 1,
                        };
                    }
                    Some("error") if ev.get("depotId").is_none() => break 1,
                    Some("cancelled") => break if interrupted { 130 } else { 1 },
                    _ => {}
                }
            }
            _ = &mut stop, if !interrupted => {
                interrupted = true;
                printer.clear();
                errln!("\ninterrupted — cancelling…");
                let _ = smd_core::ops::download::cancel_download(&sink, &core, &dir, job.clone()).await;
            }
        }
    };
    printer.clear();
    if interrupted {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    code
}

async fn record_history(
    dir: &std::path::Path,
    plan: &Plan,
    work_dir: &str,
    started_at: &str,
    results: &[Value],
    ids: &[String],
) {
    let total = if results.is_empty() {
        ids.len()
    } else {
        results.len()
    };
    let ok = if results.is_empty() {
        total
    } else {
        results
            .iter()
            .filter(|r| r["success"].as_bool() == Some(true))
            .count()
    };
    if ok == 0 {
        return;
    }
    let entry = HistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        app_id: plan.app_id.clone(),
        game_name: plan.game_name.clone(),
        header_image: plan.header_image.clone(),
        depot_count: total,
        depots_downloaded: ok,
        status: if ok == total { "complete" } else { "partial" }.into(),
        download_dir: work_dir.to_string(),
        started_at: started_at.to_string(),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        source_repo: plan.repo.clone(),
        depot_ids: if results.is_empty() {
            ids.to_vec()
        } else {
            results
                .iter()
                .filter_map(|r| r["depotId"].as_str().map(String::from))
                .collect()
        },
        resume_payload: None,
    };
    let _ = smd_core::services::history::add_entry(dir, entry).await;
}

struct Printer {
    tty: bool,
    live: bool,
    last_line: Instant,
    samples: std::collections::VecDeque<(Instant, u64)>,
}

impl Printer {
    fn new(tty: bool) -> Self {
        Printer {
            tty,
            live: false,
            last_line: Instant::now() - Duration::from_secs(60),
            samples: Default::default(),
        }
    }

    fn clear(&mut self) {
        if self.live {
            print!("\r\x1b[2K");
            self.live = false;
        }
    }

    fn line(&mut self, s: &str) {
        self.clear();
        println!("{}", s);
    }

    fn event(&mut self, ev: &Value) {
        let depot = ev["depotId"].as_str().unwrap_or("");
        match ev["type"].as_str().unwrap_or("") {
            "status" => match ev["step"].as_str().unwrap_or("") {
                "disk_space" => self.line(&format!(
                    "  free disk space: {} GB on {}",
                    ev["freeGB"],
                    ev["drive"].as_str().unwrap_or("")
                )),
                "running_downloader" => {
                    self.samples.clear();
                    if let (Some(c), Some(t)) = (ev["current"].as_u64(), ev["total"].as_u64()) {
                        self.line(&format!("[{}/{}] depot {}", c, t, depot));
                    }
                }
                "keys_generated" => self.line(&format!(
                    "  generated keys for {} depot(s)",
                    ev["depotCount"]
                )),
                "branch_found" => self.line(&format!(
                    "  {}",
                    ev["lastUpdated"].as_str().unwrap_or("source found")
                )),
                _ => {}
            },
            "manifest_source" => self.line(&format!(
                "  [{}] depot {}: {}",
                ev["source"].as_str().unwrap_or(""),
                depot,
                ev["message"].as_str().unwrap_or("")
            )),
            "output" => {
                if let (Some(done), Some(total)) =
                    (ev["completedBytes"].as_u64(), ev["totalBytes"].as_u64())
                {
                    let net = ev["networkBytes"].as_u64().unwrap_or(done);
                    let now = Instant::now();
                    self.samples.push_back((now, net));
                    while self.samples.len() > 2
                        && self
                            .samples
                            .front()
                            .is_some_and(|s| now.duration_since(s.0) > Duration::from_secs(3))
                    {
                        self.samples.pop_front();
                    }
                    let speed = match (self.samples.front(), self.samples.back()) {
                        (Some(a), Some(b)) if b.0 > a.0 => {
                            let bps = b.1.saturating_sub(a.1) as f64
                                / b.0.duration_since(a.0).as_secs_f64();
                            format!("  {}/s", fmt_bytes(bps as u64))
                        }
                        _ => String::new(),
                    };
                    let pct = ev["percent"].as_f64().unwrap_or(0.0);
                    let text = format!(
                        "  depot {}  {:5.1}%  {} / {}{}",
                        depot,
                        pct,
                        fmt_bytes(done),
                        fmt_bytes(total),
                        speed
                    );
                    if self.tty {
                        print!("\r\x1b[2K{}", text);
                        let _ = std::io::stdout().flush();
                        self.live = true;
                    } else if self.last_line.elapsed() > Duration::from_secs(5) {
                        self.last_line = Instant::now();
                        println!("{}", text);
                    }
                } else if let Some(out) = ev["output"].as_str() {
                    for l in out.lines() {
                        self.line(&format!("  {}", l));
                    }
                }
            }
            "depot_complete" => self.line(&format!("  ✓ depot {} done", depot)),
            "error" => self.line(&format!("  ✗ {}", ev["message"].as_str().unwrap_or(""))),
            "complete" | "cancelled" => self.line(ev["message"].as_str().unwrap_or("")),
            _ => {}
        }
    }
}

pub async fn search(dir: PathBuf, args: SearchArgs) -> i32 {
    let _ = tokio::fs::create_dir_all(&dir).await;
    settings_service::seed_defaults_if_needed(&dir).await;
    let core = AppState::new();
    if !is_numeric(&args.query) {
        return match smd_core::ops::search::search_steam_games(&core.http_client, &args.query).await
        {
            Ok(hits) => {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&hits).unwrap_or_default()
                    );
                } else if hits.is_empty() {
                    println!("no games found");
                } else {
                    for h in hits {
                        println!("{:>10}  {}", h.app_id, h.name);
                    }
                }
                0
            }
            Err(e) => {
                errln!("error: {}", e);
                1
            }
        };
    }
    let res = match smd_core::ops::search::search_repos(&core, &dir, &args.query).await {
        Ok(r) => r,
        Err(e) => {
            errln!("error: {}", e);
            return 1;
        }
    };
    if args.json && args.manifests.is_none() {
        println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        return 0;
    }
    if !args.json {
        if res.repos.is_empty() {
            println!(
                "App {} was not found in any configured depot source.",
                args.query
            );
            return 1;
        }
        for (i, r) in res.repos.iter().enumerate() {
            println!(
                "#{}  {}  [{}]  {}",
                i,
                r.repo,
                r.source_type,
                r.date.clone().unwrap_or_default()
            );
        }
    }
    if let Some(idx) = args.manifests {
        let Some(repo) = res.repos.get(idx) else {
            errln!("error: no result #{}", idx);
            return 1;
        };
        match smd_core::ops::search::get_repo_manifests(
            &core,
            &dir,
            &args.query,
            &repo.repo,
            repo.sha.as_deref(),
        )
        .await
        {
            Ok(m) => {
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&m).unwrap_or_default());
                } else {
                    for d in m.manifests {
                        println!(
                            "  {:>10}  manifest {}{}{}",
                            d.depot_id,
                            d.manifest_id,
                            d.size_bytes
                                .map(|s| format!("  {}", fmt_bytes(s)))
                                .unwrap_or_default(),
                            if d.depot_key.is_some() || m.depot_keys.contains_key(&d.depot_id) {
                                ""
                            } else {
                                "  (no key)"
                            }
                        );
                    }
                }
            }
            Err(e) => {
                errln!("error: {}", e);
                return 1;
            }
        }
    }
    0
}

pub async fn history(dir: PathBuf, args: HistoryArgs) -> i32 {
    let h = smd_core::services::history::load_history(&dir).await;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&h.entries).unwrap_or_default()
        );
        return 0;
    }
    if h.entries.is_empty() {
        println!("no downloads yet");
    }
    for e in h.entries {
        println!(
            "{:<20} {:>10}  {:<20} {}/{}  {}",
            e.completed_at
                .as_deref()
                .unwrap_or(&e.started_at)
                .chars()
                .take(19)
                .collect::<String>()
                .replace('T', " "),
            e.app_id,
            e.status,
            e.depots_downloaded,
            e.depot_count,
            e.game_name.unwrap_or_default()
        );
        println!("{:>32} {}", "", e.download_dir);
    }
    0
}
