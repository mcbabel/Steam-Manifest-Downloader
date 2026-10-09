use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub struct Options {
    pub data_dir: PathBuf,
    pub out: PathBuf,
    pub days: i64,
    pub all_channels: bool,
}

impl Options {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let mut opts = Options {
            data_dir: std::env::var("TELEMETRY_DATA_DIR")
                .unwrap_or_else(|_| "./data".into())
                .into(),
            out: "report.html".into(),
            days: 90,
            all_channels: false,
        };
        let mut it = args.iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--data" => opts.data_dir = it.next().context("--data needs a folder")?.into(),
                "--out" => opts.out = it.next().context("--out needs a file")?.into(),
                "--days" => {
                    opts.days = it
                        .next()
                        .context("--days needs a number")?
                        .parse()
                        .context("--days is not a number")?
                }
                "--all-channels" => opts.all_channels = true,
                other => anyhow::bail!("unknown option {}", other),
            }
        }
        Ok(opts)
    }
}

#[derive(Debug, Clone)]
pub struct Ev {
    pub day: NaiveDate,
    pub install: String,
    pub session: String,
    pub version: String,
    pub os: String,
    pub frontend: String,
    pub package: String,
    pub locale: String,
    pub kind: String,
    pub props: Value,
}

pub fn run(args: &[String]) -> Result<()> {
    let opts = Options::from_args(args)?;
    let events = load(&opts.data_dir, opts.days, opts.all_channels)?;
    let html = render(&events, Utc::now().date_naive(), opts.days);
    std::fs::write(&opts.out, html)
        .with_context(|| format!("failed to write {}", opts.out.display()))?;
    eprintln!("{} events -> {}", events.len(), opts.out.display());
    Ok(())
}

pub fn load(dir: &Path, days: i64, all_channels: bool) -> Result<Vec<Ev>> {
    let since = Utc::now().date_naive() - Duration::days(days);
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read {}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("events-") && n.ends_with(".jsonl"))
        })
        .collect();
    files.sort();
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)?;
        for line in text.lines() {
            if let Ok(record) = serde_json::from_str::<Value>(line) {
                out.extend(parse_record(&record, since, all_channels));
            }
        }
    }
    Ok(out)
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

pub fn parse_record(record: &Value, since: NaiveDate, all_channels: bool) -> Vec<Ev> {
    let payload = &record["payload"];
    let channel = text(payload, "channel");
    if !all_channels && channel != "stable" {
        return Vec::new();
    }
    let received = record["received_at"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let Some(received) = received else {
        return Vec::new();
    };
    let base = Ev {
        day: received.date_naive(),
        install: text(payload, "installation_id"),
        session: text(payload, "session_id"),
        version: text(payload, "app_version"),
        os: text(payload, "os"),
        frontend: non_empty(text(payload, "frontend"), "gui"),
        package: non_empty(text(payload, "package"), "unknown"),
        locale: non_empty(text(payload, "locale"), "unknown"),
        kind: String::new(),
        props: Value::Null,
    };
    let Some(events) = payload["events"].as_array() else {
        return Vec::new();
    };
    events
        .iter()
        .filter_map(|e| {
            let ts = e["ts"].as_i64().unwrap_or(received.timestamp());
            let day = DateTime::<Utc>::from_timestamp(ts.min(received.timestamp()), 0)
                .map(|d| d.date_naive())
                .unwrap_or(base.day);
            if day < since {
                return None;
            }
            Some(Ev {
                day,
                kind: text(e, "kind"),
                props: e.get("props").cloned().unwrap_or(Value::Null),
                ..base.clone()
            })
        })
        .collect()
}

fn non_empty(s: String, fallback: &str) -> String {
    if s.is_empty() {
        fallback.to_string()
    } else {
        s
    }
}

fn prop(e: &Ev, key: &str) -> String {
    match e.props.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Null) | None => "-".to_string(),
        Some(other) => other.to_string(),
    }
}

fn outcome(e: &Ev) -> String {
    match e.props.get("outcome").and_then(|o| o.as_str()) {
        Some(o) => o.to_string(),
        None => match e.props.get("success").and_then(|v| v.as_bool()) {
            Some(true) => "complete".to_string(),
            _ => "failed".to_string(),
        },
    }
}

fn version_key(v: &str) -> Vec<u64> {
    v.split(|c: char| !c.is_ascii_digit())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

#[derive(Default)]
pub struct Activity {
    pub dau: f64,
    pub wau: f64,
    pub months: Vec<(String, usize)>,
    pub installs: usize,
    pub sessions: usize,
}

pub fn activity(events: &[Ev], today: NaiveDate) -> Activity {
    let mut by_day: HashMap<NaiveDate, HashSet<&str>> = HashMap::new();
    let mut by_month: BTreeMap<String, HashSet<&str>> = BTreeMap::new();
    for e in events {
        by_day.entry(e.day).or_default().insert(&e.install);
        by_month
            .entry(format!("{}-{:02}", e.day.year(), e.day.month()))
            .or_default()
            .insert(&e.install);
    }
    let window: Vec<NaiveDate> = (1..=30).map(|i| today - Duration::days(i)).collect();
    let dau = window
        .iter()
        .map(|d| by_day.get(d).map_or(0, |s| s.len()))
        .sum::<usize>() as f64
        / window.len() as f64;
    let wau = window
        .iter()
        .map(|d| {
            let mut set: HashSet<&str> = HashSet::new();
            for i in 0..7 {
                if let Some(s) = by_day.get(&(*d - Duration::days(i))) {
                    set.extend(s.iter().copied());
                }
            }
            set.len()
        })
        .sum::<usize>() as f64
        / window.len() as f64;
    Activity {
        dau,
        wau,
        months: by_month.into_iter().map(|(m, s)| (m, s.len())).collect(),
        installs: events.iter().map(|e| e.install.as_str()).collect::<HashSet<_>>().len(),
        sessions: events.iter().map(|e| e.session.as_str()).collect::<HashSet<_>>().len(),
    }
}

#[derive(Default, Debug, Clone)]
pub struct VersionStats {
    pub installs: usize,
    pub started: usize,
    pub complete: usize,
    pub partial: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub lost: usize,
    pub interrupted: usize,
    pub crashes: usize,
}

impl VersionStats {
    pub fn success_rate(&self) -> Option<f64> {
        let finished = self.complete + self.partial + self.failed;
        (finished > 0).then(|| self.complete as f64 / finished as f64)
    }
}

pub fn versions(events: &[Ev]) -> Vec<(String, VersionStats)> {
    let mut map: HashMap<String, VersionStats> = HashMap::new();
    let mut installs: HashMap<String, HashSet<&str>> = HashMap::new();
    let mut started_jobs: HashMap<(String, String), String> = HashMap::new();
    let mut ended_jobs: HashSet<(String, String)> = HashSet::new();
    for e in events {
        installs.entry(e.version.clone()).or_default().insert(&e.install);
        let stats = map.entry(e.version.clone()).or_default();
        let job = e.props.get("job").and_then(|j| j.as_str()).map(String::from);
        match e.kind.as_str() {
            "download_started" => {
                stats.started += 1;
                if let Some(job) = job {
                    started_jobs.insert((e.install.clone(), job), e.version.clone());
                }
            }
            "download_completed" => {
                match outcome(e).as_str() {
                    "complete" => stats.complete += 1,
                    "partial" => stats.partial += 1,
                    "cancelled" => stats.cancelled += 1,
                    _ => stats.failed += 1,
                }
                if let Some(job) = job {
                    ended_jobs.insert((e.install.clone(), job));
                }
            }
            "download_abandoned" => {
                if let Some(job) = job {
                    ended_jobs.insert((e.install.clone(), job));
                }
            }
            "download_interrupted" => stats.interrupted += 1,
            "crash" => stats.crashes += 1,
            _ => {}
        }
    }
    for (key, version) in &started_jobs {
        if !ended_jobs.contains(key) {
            map.entry(version.clone()).or_default().lost += 1;
        }
    }
    for (version, set) in installs {
        map.entry(version).or_default().installs = set.len();
    }
    let mut list: Vec<(String, VersionStats)> = map.into_iter().collect();
    list.sort_by(|a, b| version_key(&b.0).cmp(&version_key(&a.0)));
    list
}

pub struct Group {
    pub key: Vec<String>,
    pub count: usize,
    pub installs: usize,
    pub versions: Vec<String>,
}

pub fn group(events: &[Ev], kind: &str, filter: impl Fn(&Ev) -> bool, keys: &[&str]) -> Vec<Group> {
    let mut map: HashMap<Vec<String>, (usize, HashSet<&str>, HashSet<&str>)> = HashMap::new();
    for e in events.iter().filter(|e| e.kind == kind && filter(e)) {
        let key: Vec<String> = keys.iter().map(|k| prop(e, k)).collect();
        let entry = map.entry(key).or_default();
        entry.0 += 1;
        entry.1.insert(&e.install);
        entry.2.insert(&e.version);
    }
    let mut list: Vec<Group> = map
        .into_iter()
        .map(|(key, (count, installs, versions))| {
            let mut versions: Vec<String> = versions.into_iter().map(String::from).collect();
            versions.sort_by(|a, b| version_key(b).cmp(&version_key(a)));
            Group {
                key,
                count,
                installs: installs.len(),
                versions,
            }
        })
        .collect();
    list.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    list
}

fn latest_per_install(events: &[Ev]) -> HashMap<&str, &Ev> {
    let mut map: HashMap<&str, &Ev> = HashMap::new();
    for e in events {
        match map.get(e.install.as_str()) {
            Some(prev) if prev.day > e.day => {}
            _ => {
                map.insert(&e.install, e);
            }
        }
    }
    map
}

fn share(events: &[Ev], field: impl Fn(&Ev) -> String) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in latest_per_install(events).values() {
        *counts.entry(field(e)).or_default() += 1;
    }
    let mut list: Vec<(String, usize)> = counts.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

fn settings_share(events: &[Ev]) -> Vec<(String, usize, usize)> {
    let starts: Vec<Ev> = events
        .iter()
        .filter(|e| e.kind == "app_start" && e.props.get("settings").is_some())
        .cloned()
        .collect();
    let latest = latest_per_install(&starts);
    let total = latest.len();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in latest.values() {
        if let Some(Value::Object(map)) = e.props.get("settings") {
            for (k, v) in map {
                let label = match v {
                    Value::Bool(true) => k.clone(),
                    Value::Bool(false) => continue,
                    Value::String(s) => format!("{} = {}", k, s),
                    other => format!("{} = {}", k, other),
                };
                *counts.entry(label).or_default() += 1;
            }
        }
    }
    counts.into_iter().map(|(k, n)| (k, n, total)).collect()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn pct(part: usize, total: usize) -> String {
    if total == 0 {
        "-".into()
    } else {
        format!("{:.0}%", part as f64 * 100.0 / total as f64)
    }
}

fn bar(part: usize, max: usize) -> String {
    let width = if max == 0 { 0 } else { part * 100 / max };
    format!("<span class=\"bar\"><span style=\"width:{}%\"></span></span>", width)
}

fn table(out: &mut String, head: &[&str], rows: &[Vec<String>]) {
    out.push_str("<table><thead><tr>");
    for h in head {
        let _ = write!(out, "<th>{}</th>", esc(h));
    }
    out.push_str("</tr></thead><tbody>");
    if rows.is_empty() {
        let _ = write!(out, "<tr><td colspan=\"{}\" class=\"muted\">no data</td></tr>", head.len());
    }
    for row in rows {
        out.push_str("<tr>");
        for cell in row {
            let _ = write!(out, "<td>{}</td>", cell);
        }
        out.push_str("</tr>");
    }
    out.push_str("</tbody></table>");
}

fn group_rows(groups: &[Group], limit: usize) -> Vec<Vec<String>> {
    let max = groups.first().map_or(0, |g| g.count);
    groups
        .iter()
        .take(limit)
        .map(|g| {
            let mut row: Vec<String> = g.key.iter().map(|k| format!("<code>{}</code>", esc(k))).collect();
            row.push(format!("{} {}", g.count, bar(g.count, max)));
            row.push(g.installs.to_string());
            row.push(esc(&g.versions.join(", ")));
            row
        })
        .collect()
}

pub fn render(events: &[Ev], today: NaiveDate, days: i64) -> String {
    let act = activity(events, today);
    let vers = versions(events);
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>SMD telemetry report</title><style>{}</style></head><body><main>",
        CSS
    );
    let _ = write!(
        out,
        "<h1>SMD telemetry report</h1><p class=\"muted\">Last {} days up to {} · {} events · {} installs · {} sessions</p>",
        days,
        today,
        events.len(),
        act.installs,
        act.sessions
    );

    out.push_str("<section class=\"cards\">");
    for (label, value) in [
        ("Daily active (30 day avg)", format!("{:.1}", act.dau)),
        ("Weekly active (30 day avg)", format!("{:.1}", act.wau)),
        (
            "Monthly active (latest)",
            act.months.last().map_or("-".into(), |m| m.1.to_string()),
        ),
    ] {
        let _ = write!(out, "<div class=\"card\"><b>{}</b><span>{}</span></div>", esc(&value), esc(label));
    }
    out.push_str("</section>");

    out.push_str("<h2>Monthly active installs</h2>");
    let max = act.months.iter().map(|m| m.1).max().unwrap_or(0);
    let rows: Vec<Vec<String>> = act
        .months
        .iter()
        .map(|(m, n)| vec![esc(m), format!("{} {}", n, bar(*n, max))])
        .collect();
    table(&mut out, &["Month", "Installs"], &rows);

    out.push_str("<h2>Versions</h2><p class=\"muted\">Success counts complete downloads against complete, partial and failed ones. Lost means a download started and the session never reported an end; interrupted means the next start found the marker of a download that did not finish.</p>");
    let rows: Vec<Vec<String>> = vers
        .iter()
        .map(|(v, s)| {
            vec![
                esc(v),
                s.installs.to_string(),
                s.started.to_string(),
                s.complete.to_string(),
                s.partial.to_string(),
                s.failed.to_string(),
                s.cancelled.to_string(),
                s.success_rate().map_or("-".into(), |r| format!("{:.0}%", r * 100.0)),
                s.lost.to_string(),
                s.interrupted.to_string(),
                s.crashes.to_string(),
            ]
        })
        .collect();
    table(
        &mut out,
        &["Version", "Installs", "Started", "Complete", "Partial", "Failed", "Cancelled", "Success", "Lost", "Interrupted", "Crashes"],
        &rows,
    );

    out.push_str("<h2>Download failures</h2>");
    let failures = group(
        events,
        "download_completed",
        |e| matches!(outcome(e).as_str(), "failed" | "partial"),
        &["fail_stage", "fail_class"],
    );
    table(&mut out, &["Stage", "Class", "Count", "Installs", "Versions"], &group_rows(&failures, 25));

    out.push_str("<h3>Error keys of failed downloads</h3>");
    let keys = group(
        events,
        "download_completed",
        |e| e.props.get("err_key").is_some(),
        &["err_key", "fail_stage"],
    );
    table(&mut out, &["Error key", "Stage", "Count", "Installs", "Versions"], &group_rows(&keys, 25));

    out.push_str("<h2>Crashes</h2>");
    let crashes = group(events, "crash", |_| true, &["source", "location", "message"]);
    table(&mut out, &["Source", "Location", "Message", "Count", "Installs", "Versions"], &group_rows(&crashes, 40));

    out.push_str("<h2>Errors shown</h2>");
    let shown = group(events, "error_shown", |_| true, &["area", "key"]);
    table(&mut out, &["Area", "Key", "Count", "Installs", "Versions"], &group_rows(&shown, 40));

    out.push_str("<h2>Interrupted downloads</h2>");
    let interrupted = group(events, "download_interrupted", |_| true, &["step", "engine", "mode"]);
    table(&mut out, &["Step", "Engine", "Mode", "Count", "Installs", "Versions"], &group_rows(&interrupted, 20));

    out.push_str("<h2>Emulator patching</h2>");
    let patches = group(events, "patch_applied", |_| true, &["entry", "outcome", "fail_class"]);
    table(&mut out, &["Entry", "Outcome", "Fail class", "Count", "Installs", "Versions"], &group_rows(&patches, 25));

    out.push_str("<h2>How downloads are started</h2>");
    let starts = group(events, "download_started", |_| true, &["mode", "selection", "source", "engine"]);
    table(&mut out, &["Mode", "Selection", "Source", "Engine", "Count", "Installs", "Versions"], &group_rows(&starts, 25));

    out.push_str("<h2>Other features</h2>");
    let mut feature_rows: Vec<Vec<String>> = Vec::new();
    for (kind, keys) in [
        ("library_added", &["from", "ok"][..]),
        ("game_launched", &["method", "partial"][..]),
        ("proxy_tested", &["proxy", "ok", "key"][..]),
        ("shortcut_created", &["desktop", "start_menu"][..]),
        ("update_checked", &["available", "failed"][..]),
        ("update_installed", &[][..]),
        ("diagnostics_copied", &[][..]),
        ("bug_report_opened", &["source", "diag", "log"][..]),
    ] {
        for g in group(events, kind, |_| true, keys).iter().take(8) {
            feature_rows.push(vec![
                format!("<code>{}</code>", esc(kind)),
                esc(&keys.iter().zip(&g.key).map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(" ")),
                g.count.to_string(),
                g.installs.to_string(),
            ]);
        }
    }
    table(&mut out, &["Event", "Details", "Count", "Installs"], &feature_rows);

    out.push_str("<h2>Settings in use</h2><p class=\"muted\">From the newest app_start of each install that sends schema 3.</p>");
    let rows: Vec<Vec<String>> = settings_share(events)
        .into_iter()
        .map(|(k, n, total)| vec![format!("<code>{}</code>", esc(&k)), n.to_string(), pct(n, total)])
        .collect();
    table(&mut out, &["Setting", "Installs", "Share"], &rows);

    out.push_str("<h2>Languages</h2><p class=\"muted\">The language the app is shown in, from installs that send schema 3, and the game language picked for Like Steam downloads.</p><div class=\"grid\">");
    let with_locale: Vec<Ev> = events.iter().filter(|e| e.locale != "unknown").cloned().collect();
    let starts: Vec<Ev> = events
        .iter()
        .filter(|e| e.kind == "app_start" && e.props["settings"]["game_language"].is_string())
        .cloned()
        .collect();
    for (title, list) in [
        ("App language", share(&with_locale, |e| e.locale.clone())),
        ("Game language", share(&starts, |e| e.props["settings"]["game_language"].as_str().unwrap_or("-").to_string())),
    ] {
        render_share(&mut out, title, &list);
    }
    out.push_str("</div>");

    out.push_str("<h2>Platforms</h2><div class=\"grid\">");
    for (title, list) in [
        ("Version", share(events, |e| e.version.clone())),
        ("OS", share(events, |e| e.os.clone())),
        ("Frontend", share(events, |e| e.frontend.clone())),
        ("Package", share(events, |e| e.package.clone())),
    ] {
        render_share(&mut out, title, &list);
    }
    out.push_str("</div></main></body></html>");
    out
}

fn render_share(out: &mut String, title: &str, list: &[(String, usize)]) {
    let total: usize = list.iter().map(|x| x.1).sum();
    let max = list.first().map_or(0, |x| x.1);
    let rows: Vec<Vec<String>> = list
        .iter()
        .take(12)
        .map(|(k, n)| vec![esc(k), format!("{} {}", n, bar(*n, max)), pct(*n, total)])
        .collect();
    out.push_str("<div>");
    let _ = write!(out, "<h3>{}</h3>", esc(title));
    table(out, &[title, "Installs", "Share"], &rows);
    out.push_str("</div>");
}

const CSS: &str = ":root{--bg:#f6f7f9;--fg:#1d2330;--muted:#667085;--card:#fff;--line:#e4e7ec;--accent:#3b82f6}@media (prefers-color-scheme:dark){:root{--bg:#0f1117;--fg:#e6e8ee;--muted:#98a2b3;--card:#171a22;--line:#262b36;--accent:#60a5fa}}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.5 system-ui,sans-serif}main{max-width:1100px;margin:0 auto;padding:24px 16px}h1{margin:0 0 4px}h2{margin:32px 0 8px}h3{margin:16px 0 6px;font-size:15px}.muted{color:var(--muted)}.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px;margin-top:16px}.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:14px}.card b{display:block;font-size:26px}.card span{color:var(--muted)}table{width:100%;border-collapse:collapse;background:var(--card);border:1px solid var(--line);border-radius:10px;overflow:hidden;display:block;overflow-x:auto}th,td{padding:6px 10px;border-bottom:1px solid var(--line);text-align:left;vertical-align:top;white-space:nowrap}th{color:var(--muted);font-weight:600}code{font-size:12px}.bar{display:inline-block;width:80px;height:6px;background:var(--line);border-radius:3px;margin-left:6px;vertical-align:middle}.bar span{display:block;height:100%;background:var(--accent);border-radius:3px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px}";

#[cfg(test)]
mod tests {
    use super::*;

    fn record(day: &str, install: &str, version: &str, events: Value) -> Value {
        serde_json::json!({
            "received_at": format!("{}T12:00:00Z", day),
            "payload": {
                "channel": "stable",
                "installation_id": install,
                "session_id": format!("{}-s", install),
                "app_version": version,
                "os": "windows",
                "events": events,
            }
        })
    }

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn ts(s: &str) -> i64 {
        day(s).and_hms_opt(11, 0, 0).unwrap().and_utc().timestamp()
    }

    #[test]
    fn counts_outcomes_and_lost_downloads_per_version() {
        let since = day("2026-01-01");
        let mut events = parse_record(
            &record(
                "2026-03-02",
                "a",
                "1.5.0",
                serde_json::json!([
                    { "kind": "download_started", "ts": ts("2026-03-02"), "props": { "job": "1" } },
                    { "kind": "download_completed", "ts": ts("2026-03-02"), "props": { "job": "1", "outcome": "complete" } },
                    { "kind": "download_started", "ts": ts("2026-03-02"), "props": { "job": "2" } },
                ]),
            ),
            since,
            false,
        );
        events.extend(parse_record(
            &record(
                "2026-03-03",
                "b",
                "1.5.0",
                serde_json::json!([
                    { "kind": "download_completed", "ts": ts("2026-03-03"), "props": { "outcome": "failed", "fail_stage": "depot_key" } },
                    { "kind": "download_completed", "ts": ts("2026-03-03"), "props": { "success": true } }
                ]),
            ),
            since,
            false,
        ));
        let list = versions(&events);
        let (v, s) = &list[0];
        assert_eq!(v, "1.5.0");
        assert_eq!((s.started, s.complete, s.failed, s.lost, s.installs), (2, 2, 1, 1, 2));
        assert_eq!(s.success_rate(), Some(2.0 / 3.0));
    }

    #[test]
    fn dev_builds_are_left_out_unless_asked() {
        let mut r = record("2026-03-02", "a", "1.5.0", serde_json::json!([{ "kind": "app_start", "ts": ts("2026-03-02") }]));
        r["payload"]["channel"] = serde_json::json!("dev-local");
        assert!(parse_record(&r, day("2026-01-01"), false).is_empty());
        assert_eq!(parse_record(&r, day("2026-01-01"), true).len(), 1);
    }

    #[test]
    fn daily_active_is_an_average_over_thirty_days() {
        let events: Vec<Ev> = (1..=30)
            .flat_map(|i| {
                let d = day("2026-04-01") - Duration::days(i);
                parse_record(
                    &record(&d.to_string(), if i % 2 == 0 { "a" } else { "b" }, "1.5.0", serde_json::json!([{ "kind": "app_start", "ts": d.and_hms_opt(1, 0, 0).unwrap().and_utc().timestamp() }])),
                    day("2026-01-01"),
                    false,
                )
            })
            .collect();
        let act = activity(&events, day("2026-04-01"));
        assert!((act.dau - 1.0).abs() < 1e-9);
        assert!((act.wau - 2.0).abs() < 0.3);
    }

    #[test]
    fn languages_count_the_newest_locale_per_install() {
        let mut a = record("2026-03-02", "a", "1.5.1", serde_json::json!([{ "kind": "app_start", "ts": ts("2026-03-02"), "props": { "settings": { "game_language": "german" } } }]));
        a["payload"]["locale"] = serde_json::json!("de");
        let mut b = record("2026-03-03", "b", "1.5.1", serde_json::json!([{ "kind": "app_start", "ts": ts("2026-03-03") }]));
        b["payload"]["locale"] = serde_json::json!("en");
        let old = record("2026-03-01", "c", "1.4.3", serde_json::json!([{ "kind": "app_start", "ts": ts("2026-03-01") }]));
        let mut events = parse_record(&a, day("2026-01-01"), false);
        events.extend(parse_record(&b, day("2026-01-01"), false));
        events.extend(parse_record(&old, day("2026-01-01"), false));
        let html = render(&events, day("2026-03-10"), 30);
        let section = &html[html.find("<h2>Languages").unwrap()..html.find("<h2>Platforms").unwrap()];
        assert!(section.contains("<td>de</td>"));
        assert!(section.contains("<td>en</td>"));
        assert!(section.contains("<td>german</td>"));
        assert!(!section.contains("<td>unknown</td>"));
    }

    #[test]
    fn render_escapes_reported_text() {
        let events = parse_record(
            &record(
                "2026-03-02",
                "a",
                "1.5.0",
                serde_json::json!([{ "kind": "crash", "ts": ts("2026-03-02"), "props": { "source": "js", "location": "app.js:1", "message": "<script>" } }]),
            ),
            day("2026-01-01"),
            false,
        );
        let html = render(&events, day("2026-03-10"), 30);
        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
