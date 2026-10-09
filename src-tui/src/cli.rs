use std::collections::HashMap;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Args;

use crate::i18n::{t, tf};
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
    #[arg(
        help = t("tui.cli.argSource")
    )]
    pub source: String,
    #[arg(
        long,
        value_delimiter = ',',
        help = t("tui.cli.argDepots")
    )]
    pub depots: Vec<String>,
    #[arg(
        long,
        short,
        env = "SMD_OUTPUT_DIR",
        help = t("tui.cli.argOut")
    )]
    pub out: Option<PathBuf>,
    #[arg(
        long,
        env = "SMD_MANIFESTHUB_KEY",
        hide_env_values = true,
        help = t("tui.cli.argMhKey")
    )]
    pub mh_key: Option<String>,
    #[arg(
        long,
        default_value_t = 0,
        help = t("tui.cli.argResult")
    )]
    pub result: usize,
    #[arg(
        long = "manifest",
        value_name = "DEPOT=MANIFEST",
        help = t("tui.cli.argManifest")
    )]
    pub manifests: Vec<String>,
    #[arg(
        long,
        value_name = "GAME_DIR",
        help = t("tui.cli.argUpdate")
    )]
    pub update: Option<PathBuf>,
    #[arg(long, requires = "update", help = t("tui.cli.argRepair"))]
    pub repair: bool,
    #[arg(
        long,
        value_name = "RATE",
        env = "SMD_SPEED_LIMIT",
        help = t("tui.cli.argSpeedLimit")
    )]
    pub speed_limit: Option<String>,
    #[arg(
        long,
        help = t("tui.cli.argShutdown")
    )]
    pub shutdown: bool,
    #[arg(long, help = t("tui.cli.argList"))]
    pub list: bool,
    #[arg(long, help = t("tui.cli.argIgnoreSpace"))]
    pub ignore_space: bool,
    #[arg(long, help = t("tui.cli.argAllDepots"))]
    pub all_depots: bool,
    #[arg(long, help = t("tui.cli.argDlc"))]
    pub dlc: bool,
    #[arg(long, conflicts_with = "all_depots", help = t("tui.cli.argLikeSteam"))]
    pub like_steam: bool,
    #[arg(
        long,
        value_name = "OS",
        value_parser = ["windows", "linux", "macos"],
        help = t("tui.cli.argPlatform")
    )]
    pub platform: Option<String>,
    #[arg(long, value_name = "LANGUAGE", help = t("tui.cli.argLanguage"))]
    pub language: Option<String>,
    #[arg(
        long,
        help = t("tui.cli.argJson")
    )]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    #[arg(help = t("tui.cli.argQuery"))]
    pub query: String,
    #[arg(long, help = t("tui.cli.argManifests"))]
    pub manifests: Option<usize>,
    #[arg(long, help = t("tui.cli.argSearchJson"))]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(long, help = t("tui.cli.argHistoryJson"))]
    pub json: bool,
}

macro_rules! errln {
    ($($arg:tt)*) => {
        crate::term::write_original_stderr(&format!("{}\n", format!($($arg)*)))
    };
}

static TELEMETRY: std::sync::OnceLock<smd_core::services::telemetry::Telemetry> =
    std::sync::OnceLock::new();

pub async fn telemetry_start(dir: &std::path::Path, command: &str, props: Value) {
    use smd_core::services::telemetry::{install_crash_hook, Event, Telemetry};
    let _ = tokio::fs::create_dir_all(dir).await;
    install_crash_hook(dir.to_path_buf());
    let channel = option_env!("SMD_BUILD_CHANNEL").unwrap_or("dev-local").to_string();
    let tel = Telemetry::new(dir.to_path_buf(), crate::app::VERSION.to_string(), channel, "cli");
    if tel.is_enabled().await {
        tokio::spawn(tel.clone().run_background_flush());
        tel.emit(Event::new("app_start").with_props(serde_json::json!({ "locale": crate::i18n::language() })))
            .await;
        let mut props = props;
        if let Some(map) = props.as_object_mut() {
            map.insert("command".into(), serde_json::json!(command));
        }
        tel.emit(Event::new("cli_command").with_props(props)).await;
    }
    let _ = TELEMETRY.set(tel);
}

pub async fn cli_emit(kind: &str, props: Value) {
    if let Some(tel) = TELEMETRY.get() {
        tel.emit(smd_core::services::telemetry::Event::new(kind).with_props(props))
            .await;
    }
}

pub async fn telemetry_finish() {
    if let Some(tel) = TELEMETRY.get() {
        let _ = tokio::time::timeout(Duration::from_secs(5), tel.end_session("exit")).await;
    }
}

fn merged(base: &serde_json::Map<String, Value>, extra: Value) -> Value {
    let mut out = base.clone();
    if let Value::Object(map) = extra {
        for (k, v) in map {
            out.insert(k, v);
        }
    }
    Value::Object(out)
}

#[derive(clap::Args, Debug)]
pub struct TelemetryArgs {
    #[arg(value_parser = ["on", "off", "status"], default_value = "status")]
    pub action: String,
}

pub async fn telemetry(dir: PathBuf, args: TelemetryArgs) -> i32 {
    let _ = tokio::fs::create_dir_all(&dir).await;
    match args.action.as_str() {
        "on" | "off" => {
            let accept = args.action == "on";
            if let Err(e) = smd_core::ops::consent::set_telemetry_consent(&dir, accept).await {
                errln!("{}", tf("tui.cli.error", &[("message", &e)]));
                return 1;
            }
            println!("{}", t(if accept { "tui.cli.telemetryOn" } else { "tui.cli.telemetryOff" }));
            0
        }
        _ => {
            let settings = settings_service::load_settings(&dir).await;
            let saved = settings.telemetry_consent == settings_service::TelemetryConsent::Accepted;
            let env = smd_core::services::telemetry::env_consent();
            let on = env.unwrap_or(saved);
            println!("{}", t(if on { "tui.cli.telemetryStatusOn" } else { "tui.cli.telemetryStatusOff" }));
            if let Some(v) = env {
                println!("{}", tf("tui.cli.telemetryEnv", &[("value", &if v { "on" } else { "off" })]));
            }
            if on && !settings.installation_id.is_empty() {
                println!(
                    "{}",
                    tf(
                        "tui.cli.telemetryId",
                        &[("id", &smd_core::services::telemetry::diagnostic_id(&settings.installation_id))]
                    )
                );
            }
            0
        }
    }
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
    all_app_ids: Vec<String>,
}

async fn plan_from_source(
    core: &AppState,
    dir: &std::path::Path,
    args: &DownloadArgs,
) -> Result<Plan, String> {
    if is_numeric(&args.source) {
        let res = smd_core::ops::search::search_repos(core, dir, &args.source).await?;
        let repo = res.repos.get(args.result).cloned().ok_or_else(|| {
            tf(
                "tui.cli.appNotFoundResult",
                &[("app", &args.source), ("n", &args.result)],
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
            all_app_ids: Vec::new(),
        })
    } else {
        let path = crate::ui::file_browser::expand_tilde(&args.source);
        let parsed = smd_core::ops::files::parse_source_file(&path.to_string_lossy()).await?;
        let app_id = parsed
            .main_app_id
            .map(|i| i.to_string())
            .ok_or_else(|| t("tui.cli.noMainAppId"))?;
        let all_app_ids = parsed.all_app_ids.iter().map(|i| i.to_string()).collect();
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
            all_app_ids,
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

fn auto_choice_line(choice: &smd_core::services::depot_select::Selection, total: usize) -> String {
    let lang_key = format!("steamLanguages.{}", choice.language);
    let lang = match t(&lang_key) {
        name if name == lang_key => choice.language.clone(),
        name => name,
    };
    let platform = match choice.platform.as_str() {
        "windows" => "Windows",
        "linux" => "Linux",
        "macos" => "macOS",
        other => other,
    };
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for s in &choice.skipped {
        match counts.iter_mut().find(|(k, _)| *k == s.reason.key()) {
            Some((_, n)) => *n += 1,
            None => counts.push((s.reason.key(), 1)),
        }
    }
    let mut line = format!(
        "{} — {} · {}",
        tf(
            "select.autoSelectTitle",
            &[("count", &choice.selected.len()), ("total", &total)]
        ),
        platform,
        lang
    );
    if !counts.is_empty() {
        let list = counts
            .iter()
            .map(|(k, n)| format!("{}× {}", n, t(&format!("select.skipReason.{}", k))))
            .collect::<Vec<_>>()
            .join(", ");
        line.push_str(&format!(" — {}", tf("select.autoSelectSkipped", &[("list", &list)])));
    }
    line
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
            errln!("{}", tf("tui.cli.error", &[("message", &e)]));
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
    let mut pins: HashMap<String, String> = args
        .manifests
        .iter()
        .filter_map(|m| {
            m.split_once('=')
                .map(|(d, id)| (d.trim().to_string(), id.trim().to_string()))
        })
        .collect();
    let mut wanted = wanted;
    if args.repair {
        let dir = args.update.clone().unwrap_or_default();
        let installed = smd_core::services::install_state::installed(&dir);
        if wanted.is_empty() {
            wanted = installed.iter().map(|d| d.depot_id.clone()).collect();
        }
        for d in installed {
            pins.entry(d.depot_id).or_insert(d.manifest_id);
        }
    }
    let like_steam = args.like_steam || (settings.auto_select_depots && !args.all_depots);
    let selection = if !args.depots.is_empty() {
        "manual"
    } else if like_steam && args.update.is_none() {
        "like_steam"
    } else {
        "default"
    };
    if wanted.is_empty() && args.update.is_none() && like_steam {
        let meta = match plan.app_id.parse::<u32>() {
            Ok(n) => tokio::select! {
                r = smd_core::services::steam_pics::fetch_depots_with_names(core.steam_session.clone(), n) => r,
                _ = stopped() => return 130,
            },
            Err(_) => Err("invalid app id".to_string()),
        };
        match meta {
            Ok(meta) if !meta.is_empty() => {
                let mut prefs_settings = settings.clone();
                if let Some(p) = &args.platform {
                    prefs_settings.target_platform = p.clone();
                }
                if let Some(l) = &args.language {
                    prefs_settings.game_language = l.clone();
                }
                if args.dlc {
                    prefs_settings.include_dlc = true;
                }
                let prefs = smd_core::services::depot_select::prefs_from_settings(
                    &prefs_settings,
                    crate::i18n::language(),
                );
                let candidates: Vec<String> = plan.depots.iter().map(|(id, ..)| id.clone()).collect();
                let choice = smd_core::services::depot_select::recommend_keyed(
                    &meta,
                    &candidates,
                    |id| {
                        plan.depots.iter().any(|(d, _, key, _)| d == id && key.is_some())
                            || plan.key_vdf.as_ref().is_some_and(|k| k.contains_key(id))
                    },
                    &prefs,
                );
                if choice.known {
                    if !args.json {
                        println!("{}", auto_choice_line(&choice, candidates.len()));
                    }
                    wanted = choice.selected.clone();
                }
            }
            Ok(_) => {}
            Err(e) => {
                if !args.json {
                    errln!("{}", tf("tui.cli.autoSelectFailed", &[("message", &e)]));
                }
            }
        }
    }
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
    if args.json {
        let depots: Vec<Value> = chosen
            .iter()
            .map(|(id, manifest, key, size)| {
                serde_json::json!({
                    "depotId": id,
                    "manifestId": pins.get(id).or(manifest.as_ref()),
                    "sizeBytes": size,
                    "hasKey": key.is_some(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "type": "plan",
                "appId": plan.app_id,
                "gameName": plan.game_name,
                "totalDepots": plan.depots.len(),
                "depots": depots,
            })
        );
    } else {
        println!(
            "{}",
            tf(
                "tui.cli.plan",
                &[
                    ("title", &title),
                    ("count", &chosen.len()),
                    ("total", &plan.depots.len()),
                ],
            )
        );
    }
    for (id, manifest, key, size) in chosen.iter().filter(|_| !args.json) {
        println!(
            "  {:>10}  {} {}{}{}",
            id,
            t("tui.cli.manifest"),
            pins.get(id)
                .or(manifest.as_ref())
                .map(String::as_str)
                .unwrap_or("N/A"),
            size.map(|s| format!("  {}", fmt_bytes(s)))
                .unwrap_or_default(),
            if key.is_none() {
                format!("  {}", t("tui.cli.noKeyInFile"))
            } else {
                String::new()
            }
        );
    }
    if args.list {
        return 0;
    }
    if chosen.is_empty() {
        errln!(
            "{}",
            tf("tui.cli.error", &[("message", &t("tui.cli.noDepots"))])
        );
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
        update_dir: args
            .update
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        speed_limit: args.speed_limit.clone(),
        resume_mode: None,
        repair: args.repair,
        like_steam: Some(like_steam),
        all_app_ids: Some(plan.all_app_ids.clone()),
        include_dlc: args.dlc.then_some(true),
    };
    if let Some(limit) = config.speed_limit.as_deref() {
        if let Err(e) = smd_core::services::speed_limit::parse_speed_limit(limit) {
            errln!("{}", tf("tui.cli.error", &[("message", &e)]));
            return 1;
        }
    }
    if !args.ignore_space && config.update_dir.is_none() {
        let needed: u64 = chosen.iter().filter_map(|(_, _, _, size)| *size).sum();
        let base = smd_core::ops::download::base_download_dir(config.download_location.as_deref());
        let check = smd_core::services::disk_space::check(&base, needed);
        if let Some(free) = check.free.filter(|_| needed > 0 && !check.enough) {
            errln!(
                "{}",
                tf(
                    "tui.cli.error",
                    &[(
                        "message",
                        &tf(
                            "tui.cli.noSpace",
                            &[
                                ("needed", &fmt_bytes(needed)),
                                ("free", &fmt_bytes(free)),
                                ("path", &check.path),
                            ],
                        )
                    )]
                )
            );
            return 1;
        }
    }
    let depot_ids: Vec<String> = config.depots.iter().map(|d| d.depot_id.clone()).collect();
    let job_tag: String = uuid::Uuid::new_v4().simple().to_string().chars().take(16).collect();
    let mut ctx = serde_json::Map::new();
    for (k, v) in [
        ("job", serde_json::json!(job_tag)),
        ("engine", serde_json::json!(if settings.use_native_downloader { "native" } else { "ddm" })),
        ("source_count", serde_json::json!(settings.depot_sources.len())),
        ("had_mh_key", serde_json::json!(config.manifest_hub_api_key.is_some())),
        ("mode", serde_json::json!(if config.repair { "repair" } else if config.update_dir.is_some() { "update" } else { "new" })),
        ("selection", serde_json::json!(selection)),
        ("source", serde_json::json!(plan.source_type.clone().unwrap_or_else(|| if is_numeric(&args.source) { "search".into() } else { "upload".into() }))),
        ("queue", serde_json::json!(false)),
        ("dlc", serde_json::json!(config.include_dlc)),
        ("custom_manifest", serde_json::json!(!args.manifests.is_empty())),
    ] {
        ctx.insert(k.to_string(), v);
    }
    cli_emit("download_started", merged(&ctx, serde_json::json!({ "depot_count": depot_ids.len(), "json": args.json, "shutdown": args.shutdown }))).await;

    let (tx, mut rx) = unbounded_channel();
    let sink: smd_core::services::events::Sink = Arc::new(ChannelSink(tx));
    let core = Arc::new(core);
    let started_at = chrono::Utc::now().to_rfc3339();
    let started =
        match smd_core::ops::download::start_download(sink.clone(), &core, &dir, config).await {
            Ok(v) => v,
            Err(e) => {
                cli_emit("download_completed", merged(&ctx, serde_json::json!({
                    "success": false, "outcome": "failed", "fail_stage": "unknown", "fail_class": "unknown",
                    "err_key": crate::i18n::error_key(&e).unwrap_or_else(|| "unmatched".into()),
                }))).await;
                errln!("{}", tf("tui.cli.error", &[("message", &e)]));
                return 1;
            }
        };
    let job = started["jobId"].as_str().unwrap_or_default().to_string();
    let work_dir = started["downloadDir"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if args.json {
        println!(
            "{}",
            serde_json::json!({ "type": "started", "jobId": job, "downloadDir": work_dir })
        );
    } else {
        println!("→ {}", work_dir);
    }

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
                        let mut props = merged(&ctx, ev["diag"].clone());
                        props["success"] = serde_json::json!(outcome == "complete");
                        cli_emit("download_completed", props).await;
                        let results = ev["results"].as_array().cloned().unwrap_or_default();
                        record_history(&dir, &plan, &work_dir, &started_at, &results, &depot_ids).await;
                        break match outcome {
                            "complete" => 0,
                            "partial" => 2,
                            _ => 1,
                        };
                    }
                    Some("error") if ev.get("depotId").is_none() => {
                        let mut props = merged(&ctx, ev["diag"].clone());
                        props["success"] = serde_json::json!(false);
                        props["err_key"] = serde_json::json!(ev["key"].as_str().map(String::from)
                            .or_else(|| crate::i18n::error_key(ev["message"].as_str().unwrap_or("")))
                            .unwrap_or_else(|| "unmatched".into()));
                        cli_emit("download_completed", props).await;
                        break 1;
                    }
                    Some("cancelled") => {
                        if interrupted {
                            cli_emit("download_abandoned", merged(&ctx, serde_json::json!({ "outcome": "abandoned", "via": "signal" }))).await;
                        } else {
                            cli_emit("download_completed", merged(&ctx, serde_json::json!({ "success": false, "outcome": "cancelled" }))).await;
                        }
                        break if interrupted { 130 } else { 1 };
                    }
                    _ => {}
                }
            }
            _ = &mut stop, if !interrupted => {
                interrupted = true;
                printer.clear();
                errln!("\n{}", t("tui.cli.interrupted"));
                let _ = smd_core::ops::download::cancel_download(&sink, &core, &dir, job.clone()).await;
            }
        }
    };
    printer.clear();
    if interrupted {
        tokio::time::sleep(Duration::from_millis(300)).await;
        return code;
    }
    if args.shutdown {
        shutdown_after_download().await;
    }
    code
}

async fn shutdown_after_download() {
    errln!("{}", t("tui.cli.shutdownIn"));
    cli_emit("shutdown_after", serde_json::json!({ "action": "countdown" })).await;
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(60)) => {}
        _ = stopped() => {
            cli_emit("shutdown_after", serde_json::json!({ "action": "aborted" })).await;
            errln!("{}", t("tui.cli.shutdownAborted"));
            return;
        }
    }
    cli_emit("shutdown_after", serde_json::json!({ "action": "powered_off", "followup": false })).await;
    telemetry_finish().await;
    let result = tokio::task::spawn_blocking(smd_core::ops::system::power_off)
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r);
    if let Err(e) = result {
        errln!(
            "{}",
            tf(
                "tui.cli.error",
                &[("message", &tf("tui.cli.shutdownFailed", &[("message", &e)]))]
            )
        );
    }
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
        size_bytes: None,
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
                    "  {}",
                    tf(
                        "tui.cli.diskSpace",
                        &[
                            ("gb", &ev["freeGB"]),
                            ("drive", &ev["drive"].as_str().unwrap_or("")),
                        ],
                    )
                )),
                "running_downloader" => {
                    self.samples.clear();
                    if let (Some(c), Some(t)) = (ev["current"].as_u64(), ev["total"].as_u64()) {
                        self.line(&tf(
                            "tui.cli.depotHeader",
                            &[("current", &c), ("total", &t), ("depot", &depot)],
                        ));
                    }
                }
                "keys_generated" => self.line(&format!(
                    "  {}",
                    tf("tui.progress.keysGenerated", &[("count", &ev["depotCount"])])
                )),
                "branch_found" => {
                    let text = if ev.get("key").is_some() {
                        crate::i18n::event_text(ev)
                    } else {
                        ev["lastUpdated"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| t("tui.cli.sourceFound"))
                    };
                    self.line(&format!("  {}", text));
                }
                "manifest_hub_rate_limited" | "retrying_depot" | "depot_up_to_date" | "removed_stale_files" => {
                    self.line(&format!("  {}", crate::i18n::event_text(ev)))
                }
                _ => {}
            },
            "manifest_source" => {
                let source = ev["source"].as_str().unwrap_or("");
                let label_key = format!("events.src.label.{}", source);
                let label = match t(&label_key) {
                    l if l == label_key => source.to_string(),
                    l => l,
                };
                self.line(&format!(
                    "  {}",
                    tf(
                        "events.line",
                        &[
                            ("label", &label),
                            ("depot", &depot),
                            ("text", &crate::i18n::event_text(ev)),
                        ],
                    )
                ));
                if source == "manifesthub_unavailable" {
                    self.line(&format!("  {}", t("tui.cli.mhHint")));
                }
            }
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
                        "  {}  {:5.1}%  {} / {}{}",
                        tf("tui.cli.depotProgress", &[("depot", &depot)]),
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
            "depot_complete" => self.line(&format!(
                "  {}",
                tf("tui.cli.depotDone", &[("depot", &depot)])
            )),
            "error" => self.line(&format!("  ✗ {}", crate::i18n::event_text(ev))),
            "cancelled" => {
                let text = match ev["step"].as_str() {
                    Some("cancelled_kept") => t("progress.cancelledKept"),
                    Some("cancelled_cleanup") => t("progress.cancelledCleanup"),
                    _ => crate::i18n::event_text(ev),
                };
                self.line(&text)
            }
            "complete" => self.line(&crate::i18n::event_text(ev)),
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
                cli_emit("search_performed", serde_json::json!({ "found": !hits.is_empty(), "by": "name", "repo_count": 0 })).await;
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&hits).unwrap_or_default()
                    );
                } else if hits.is_empty() {
                    println!("{}", t("tui.cli.noGames"));
                } else {
                    for h in hits {
                        println!("{:>10}  {}", h.app_id, h.name);
                    }
                }
                0
            }
            Err(e) => {
                errln!("{}", tf("tui.cli.error", &[("message", &e)]));
                1
            }
        };
    }
    let res = match smd_core::ops::search::search_repos(&core, &dir, &args.query).await {
        Ok(r) => {
            cli_emit("search_performed", serde_json::json!({ "found": !r.repos.is_empty(), "by": "app_id", "repo_count": r.repos.len() })).await;
            r
        }
        Err(e) => {
            cli_emit("search_performed", serde_json::json!({ "found": false, "by": "app_id", "repo_count": 0, "probe_class": "error" })).await;
            errln!("{}", tf("tui.cli.error", &[("message", &e)]));
            return 1;
        }
    };
    if args.json && args.manifests.is_none() {
        println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        return 0;
    }
    if !args.json {
        if res.repos.is_empty() {
            println!("{}", tf("tui.cli.appNotFound", &[("app", &args.query)]));
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
            errln!(
                "{}",
                tf(
                    "tui.cli.error",
                    &[("message", &tf("tui.cli.noResult", &[("n", &idx)]))]
                )
            );
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
                            "  {:>10}  {} {}{}{}",
                            d.depot_id,
                            t("tui.cli.manifest"),
                            d.manifest_id,
                            d.size_bytes
                                .map(|s| format!("  {}", fmt_bytes(s)))
                                .unwrap_or_default(),
                            if d.depot_key.is_some() || m.depot_keys.contains_key(&d.depot_id) {
                                String::new()
                            } else {
                                format!("  {}", t("tui.cli.noKey"))
                            }
                        );
                    }
                }
            }
            Err(e) => {
                errln!("{}", tf("tui.cli.error", &[("message", &e)]));
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
        println!("{}", t("tui.cli.noDownloads"));
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
            status_label(&e),
            e.depots_downloaded,
            e.depot_count,
            e.game_name.unwrap_or_default()
        );
        println!("{:>32} {}", "", e.download_dir);
    }
    0
}

fn status_label(e: &HistoryEntry) -> String {
    match e.status.as_str() {
        "complete" => t("history.statusComplete"),
        "partial" => t("history.statusPartial"),
        "cancelled_resumable" if e.resume_payload.is_some() => t("history.statusResumable"),
        "cancelled" | "cancelled_resumable" => t("history.statusCancelled"),
        _ => t("history.statusFailed"),
    }
}
