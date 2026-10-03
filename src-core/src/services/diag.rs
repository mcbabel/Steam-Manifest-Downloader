use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    SourceProbe,
    Login,
    Pics,
    ManifestCode,
    ManifestFetch,
    ManifestDecode,
    DepotKey,
    CdnToken,
    Chunk,
    Disk,
    Ddm,
    Unknown,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::SourceProbe => "source_probe",
            Stage::Login => "login",
            Stage::Pics => "pics",
            Stage::ManifestCode => "manifest_code",
            Stage::ManifestFetch => "manifest_fetch",
            Stage::ManifestDecode => "manifest_decode",
            Stage::DepotKey => "depot_key",
            Stage::CdnToken => "cdn_token",
            Stage::Chunk => "chunk",
            Stage::Disk => "disk",
            Stage::Ddm => "ddm",
            Stage::Unknown => "unknown",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    SteamDirect,
    DepotSource,
    Hubcap,
    Ryuu,
    ManifestHub,
    Uploaded,
    Cached,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::SteamDirect => "steam_direct",
            SourceKind::DepotSource => "depot_source",
            SourceKind::Hubcap => "hubcap",
            SourceKind::Ryuu => "ryuu",
            SourceKind::ManifestHub => "manifesthub",
            SourceKind::Uploaded => "uploaded",
            SourceKind::Cached => "cached",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "steam_direct" => Some(SourceKind::SteamDirect),
            "depot_source" => Some(SourceKind::DepotSource),
            "hubcap" => Some(SourceKind::Hubcap),
            "ryuu" => Some(SourceKind::Ryuu),
            "manifesthub" => Some(SourceKind::ManifestHub),
            "uploaded" => Some(SourceKind::Uploaded),
            "cached" => Some(SourceKind::Cached),
            _ => None,
        }
    }
}

pub const NOT_FOUND: &str = "not_found";
pub const UNAUTHORIZED: &str = "unauthorized";
pub const RATE_LIMITED: &str = "rate_limited";
pub const CLIENT_ERROR: &str = "http_4xx";
pub const SERVER_ERROR: &str = "http_5xx";
pub const REDIRECT_LOOP: &str = "redirect";
pub const DNS: &str = "dns";
pub const CONNECT: &str = "connect";
pub const TLS: &str = "tls";
pub const TIMEOUT: &str = "timeout";
pub const BODY: &str = "body";
pub const DECODE: &str = "decode";
pub const IO: &str = "io";
pub const CANCELLED: &str = "cancelled";
pub const NO_SOURCES: &str = "no_sources_configured";
pub const NO_KEY: &str = "no_api_key";
pub const UNKNOWN: &str = "unknown";

pub fn classify_status(status: u16) -> &'static str {
    match status {
        401 | 403 => UNAUTHORIZED,
        404 | 410 => NOT_FOUND,
        429 => RATE_LIMITED,
        300..=399 => REDIRECT_LOOP,
        400..=499 => CLIENT_ERROR,
        500..=599 => SERVER_ERROR,
        _ => UNKNOWN,
    }
}

pub fn classify_transport(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        return TIMEOUT;
    }
    if err.is_connect() {
        return CONNECT;
    }
    if err.is_decode() {
        return DECODE;
    }
    if err.is_body() {
        return BODY;
    }
    if err.is_redirect() {
        return REDIRECT_LOOP;
    }
    if err.is_status() {
        return err.status().map(|s| classify_status(s.as_u16())).unwrap_or(UNKNOWN);
    }
    UNKNOWN
}

fn mentions_status(text: &str, code: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(rel) = text[from..].find(code) {
        let start = from + rel;
        let end = start + code.len();
        let left_ok = start == 0 || !bytes[start - 1].is_ascii_digit();
        let right_ok = end == bytes.len() || !bytes[end].is_ascii_digit();
        if left_ok && right_ok {
            return true;
        }
        from = end;
    }
    false
}

fn rate_limited(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("too many requests")
        || lower.contains("error code: 1015")
        || mentions_status(text, "429")
}

pub fn classify_pipeline_error(text: &str) -> (Stage, &'static str) {
    if rate_limited(text) {
        let stage = if text.contains("chunk") {
            Stage::Chunk
        } else if text.contains("ManifestHub") || text.contains("manifest") {
            Stage::ManifestFetch
        } else {
            Stage::Unknown
        };
        return (stage, RATE_LIMITED);
    }

    const TABLE: &[(&str, Stage, &str)] = &[
        ("no manifest sources configured", Stage::SourceProbe, NO_SOURCES),
        ("No ManifestHub API key", Stage::ManifestFetch, NO_KEY),
        ("chunk decode failed", Stage::Chunk, DECODE),
        ("chunk size mismatch", Stage::Chunk, DECODE),
        ("chunk SHA mismatch", Stage::Chunk, DECODE),
        ("chunk CRC mismatch", Stage::Chunk, DECODE),
        ("chunk download exhausted retries", Stage::Chunk, TIMEOUT),
        ("chunk CDN returned HTTP error", Stage::Chunk, SERVER_ERROR),
        ("chunk CDN request failed", Stage::Chunk, CONNECT),
        ("chunk CDN body read failed", Stage::Chunk, BODY),
        ("chunk task join failed", Stage::Chunk, UNKNOWN),
        ("unrecognised chunk container magic", Stage::Chunk, DECODE),
        ("Unsupported VZ version", Stage::Chunk, DECODE),
        ("VZ blob too short", Stage::Chunk, DECODE),
        ("VSZTD", Stage::Chunk, DECODE),
        ("zstd decode failed", Stage::Chunk, DECODE),
        ("LZMA decompress failed", Stage::Chunk, DECODE),
        ("Anonymous Steam login failed", Stage::Login, UNAUTHORIZED),
        ("Steam server discovery failed", Stage::Login, CONNECT),
        ("Steam session bootstrap failed", Stage::Login, CONNECT),
        ("Steam PICS batch query timed out", Stage::Pics, TIMEOUT),
        ("Steam PICS query timed out", Stage::Pics, TIMEOUT),
        ("PICS request failed", Stage::Pics, CONNECT),
        ("PICS returned no info", Stage::Pics, NOT_FOUND),
        ("PICS info has no", Stage::Pics, NOT_FOUND),
        ("DepotDownloader exited with non-zero code", Stage::Ddm, UNKNOWN),
        ("Failed to start DepotDownloaderMod", Stage::Ddm, IO),
        ("Failed to wait for DepotDownloaderMod", Stage::Ddm, IO),
        ("GetManifestRequestCode failed", Stage::ManifestCode, NOT_FOUND),
        ("no manifest request code", Stage::ManifestCode, NOT_FOUND),
        ("external manifest-code providers failed", Stage::ManifestCode, NOT_FOUND),
        ("Steam CDN server discovery timed out", Stage::CdnToken, TIMEOUT),
        ("GetCDNAuthToken failed", Stage::CdnToken, UNAUTHORIZED),
        ("GetServersForSteamPipe failed", Stage::CdnToken, SERVER_ERROR),
        ("no CDN servers returned", Stage::CdnToken, NOT_FOUND),
        ("ManifestHub API request failed", Stage::ManifestFetch, CONNECT),
        ("ManifestHub API error for depot", Stage::ManifestFetch, SERVER_ERROR),
        ("Failed to read ManifestHub response body", Stage::ManifestFetch, BODY),
        ("ManifestHub API:", Stage::ManifestFetch, CLIENT_ERROR),
        ("manifest CDN returned HTTP error", Stage::ManifestFetch, SERVER_ERROR),
        ("manifest CDN request failed", Stage::ManifestFetch, CONNECT),
        ("manifest CDN body read failed", Stage::ManifestFetch, BODY),
        ("Manifest not found:", Stage::ManifestFetch, NOT_FOUND),
        ("Cached hubcap manifest not found", Stage::ManifestFetch, NOT_FOUND),
        ("All manifest downloads failed", Stage::ManifestFetch, NOT_FOUND),
        ("manifest zip", Stage::ManifestDecode, DECODE),
        ("protobuf parse failed", Stage::ManifestDecode, DECODE),
        ("magic mismatch: got 0x", Stage::ManifestDecode, DECODE),
        ("symmetric ciphertext too short", Stage::ManifestDecode, DECODE),
        ("AES-256", Stage::ManifestDecode, DECODE),
        ("filename base64 decode failed", Stage::ManifestDecode, DECODE),
        ("filename not valid UTF-8", Stage::ManifestDecode, DECODE),
        ("end-of-manifest read failed", Stage::ManifestDecode, DECODE),
        ("extends past manifest body", Stage::ManifestDecode, DECODE),
        ("header read failed", Stage::ManifestDecode, DECODE),
        ("length read failed", Stage::ManifestDecode, DECODE),
        ("length overflow", Stage::ManifestDecode, DECODE),
        ("depot key", Stage::DepotKey, NOT_FOUND),
        ("has no key", Stage::DepotKey, NOT_FOUND),
        ("key hex invalid", Stage::DepotKey, DECODE),
        ("key length !=", Stage::DepotKey, DECODE),
        ("read cached manifest failed", Stage::Disk, IO),
        ("manifest from disk failed", Stage::Disk, IO),
        ("create parent dir failed", Stage::Disk, IO),
        ("create dir failed", Stage::Disk, IO),
        ("pre-create file failed", Stage::Disk, IO),
        ("open file failed", Stage::Disk, IO),
        ("set_len failed", Stage::Disk, IO),
        ("seek failed", Stage::Disk, IO),
        ("write failed", Stage::Disk, IO),
    ];

    for (needle, stage, class) in TABLE {
        if text.contains(needle) {
            return (*stage, class);
        }
    }

    let lower = text.to_ascii_lowercase();
    if lower.contains("timed out") || lower.contains("timeout") {
        return (Stage::Unknown, TIMEOUT);
    }
    if lower.contains("cancel") {
        return (Stage::Unknown, CANCELLED);
    }
    (Stage::Unknown, UNKNOWN)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Complete,
    Partial,
    Failed,
    Cancelled,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Complete => "complete",
            Outcome::Partial => "partial",
            Outcome::Failed => "failed",
            Outcome::Cancelled => "cancelled",
        }
    }

    pub fn from_counts(ok: usize, total: usize) -> Self {
        if total == 0 {
            Outcome::Failed
        } else if ok == total {
            Outcome::Complete
        } else if ok > 0 {
            Outcome::Partial
        } else {
            Outcome::Failed
        }
    }
}

pub fn duration_bucket(secs: u64) -> &'static str {
    match secs {
        0..=4 => "<5s",
        5..=29 => "5-30s",
        30..=119 => "30s-2m",
        120..=599 => "2-10m",
        600..=3599 => "10-60m",
        _ => ">60m",
    }
}

pub fn count_bucket(n: usize) -> &'static str {
    match n {
        0 => "0",
        1 => "1",
        2 => "2",
        3..=4 => "3-4",
        5..=8 => "5-8",
        9..=16 => "9-16",
        _ => "17+",
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DepotDiag {
    pub ok: bool,
    pub stage: Option<Stage>,
    pub class: Option<&'static str>,
    pub source: Option<SourceKind>,
}

impl DepotDiag {
    pub fn ok(source: SourceKind) -> Self {
        Self { ok: true, stage: None, class: None, source: Some(source) }
    }

    pub fn failed(stage: Stage, class: &'static str) -> Self {
        Self { ok: false, stage: Some(stage), class: Some(class), source: None }
    }

    pub fn failed_at(stage: Stage, class: &'static str, source: SourceKind) -> Self {
        Self { ok: false, stage: Some(stage), class: Some(class), source: Some(source) }
    }
}

fn served_by(r: &serde_json::Value) -> Option<SourceKind> {
    r["sourcesTried"]
        .as_array()?
        .iter()
        .rev()
        .find_map(|v| v.as_str().and_then(SourceKind::from_label))
}

pub fn from_download_results(results: &[serde_json::Value], source: SourceKind) -> Vec<DepotDiag> {
    results
        .iter()
        .map(|r| {
            if r["success"].as_bool().unwrap_or(false) {
                DepotDiag::ok(served_by(r).unwrap_or(source))
            } else {
                let (stage, class) = classify_pipeline_error(r["error"].as_str().unwrap_or(""));
                DepotDiag::failed_at(stage, class, source)
            }
        })
        .collect()
}

pub fn tried_from_results(results: &[serde_json::Value]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for r in results {
        if let Some(list) = r["sourcesTried"].as_array() {
            for v in list {
                if let Some(kind) = v.as_str().and_then(SourceKind::from_label) {
                    out.push(kind.as_str());
                }
            }
        }
    }
    out
}

pub fn tried_from_diags(depots: &[DepotDiag]) -> Vec<&'static str> {
    depots
        .iter()
        .filter_map(|d| d.source.map(|s| s.as_str()))
        .collect()
}

pub fn summarize(
    depots: &[DepotDiag],
    tried: &[&'static str],
    elapsed_secs: u64,
    engine: &'static str,
) -> serde_json::Value {
    let total = depots.len();
    let ok = depots.iter().filter(|d| d.ok).count();
    let outcome = Outcome::from_counts(ok, total);

    let mut stages: Vec<&'static str> = Vec::new();
    let mut classes: Vec<&'static str> = Vec::new();
    let mut sources_ok: Vec<&'static str> = Vec::new();
    for d in depots {
        if let Some(s) = d.stage {
            stages.push(s.as_str());
        }
        if let Some(c) = d.class {
            classes.push(c);
        }
        if d.ok {
            if let Some(s) = d.source {
                sources_ok.push(s.as_str());
            }
        }
    }

    serde_json::json!({
        "outcome": outcome.as_str(),
        "depots_total": total,
        "depots_ok": ok,
        "depot_bucket": count_bucket(total),
        "duration_bucket": duration_bucket(elapsed_secs),
        "engine": engine,
        "fail_stage": mode(&stages),
        "fail_class": mode(&classes),
        "source_ok": mode(&sources_ok),
        "fail_stages": tally(&stages),
        "sources_ok": tally(&sources_ok),
        "sources_tried": tally(tried),
    })
}

fn mode(items: &[&'static str]) -> Option<&'static str> {
    let mut best: Option<(&'static str, usize)> = None;
    for it in items {
        let n = items.iter().filter(|o| *o == it).count();
        if best.map_or(true, |(_, bn)| n > bn) {
            best = Some((it, n));
        }
    }
    best.map(|(s, _)| s)
}

fn tally(items: &[&'static str]) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for it in items {
        let e = out.entry(it.to_string()).or_insert(serde_json::json!(0));
        if let Some(n) = e.as_u64() {
            *e = serde_json::json!(n + 1);
        }
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_and_overloaded_are_distinguishable() {
        assert_eq!(classify_status(404), NOT_FOUND);
        assert_eq!(classify_status(503), SERVER_ERROR);
        assert_eq!(classify_status(429), RATE_LIMITED);
        assert_ne!(classify_status(404), classify_status(503));
    }

    #[test]
    fn partial_is_not_a_success() {
        assert_eq!(Outcome::from_counts(4, 4), Outcome::Complete);
        assert_eq!(Outcome::from_counts(1, 4), Outcome::Partial);
        assert_eq!(Outcome::from_counts(0, 4), Outcome::Failed);
        assert_eq!(Outcome::from_counts(0, 0), Outcome::Failed);
    }

    #[test]
    fn summary_reports_the_dominant_reason_and_the_spread() {
        let depots = vec![
            DepotDiag::ok(SourceKind::SteamDirect),
            DepotDiag::failed(Stage::ManifestCode, NOT_FOUND),
            DepotDiag::failed(Stage::ManifestCode, NOT_FOUND),
            DepotDiag::failed(Stage::Chunk, TIMEOUT),
        ];
        let v = summarize(&depots, &["steam_direct", "steam_direct", "depot_source"], 7, "native");
        assert_eq!(v["outcome"], "partial");
        assert_eq!(v["depots_ok"], 1);
        assert_eq!(v["depots_total"], 4);
        assert_eq!(v["fail_stage"], "manifest_code");
        assert_eq!(v["duration_bucket"], "5-30s");
        assert_eq!(v["fail_stages"]["manifest_code"], 2);
        assert_eq!(v["fail_stages"]["chunk"], 1);
        assert_eq!(v["source_ok"], "steam_direct");
        assert_eq!(v["sources_tried"]["steam_direct"], 2);
        assert_eq!(v["sources_tried"]["depot_source"], 1);
    }

    #[test]
    fn tried_chain_survives_the_round_trip_but_unknown_labels_do_not() {
        let results = vec![
            serde_json::json!({
                "depotId": "731", "success": false, "error": "GetManifestRequestCode failed: 404",
                "sourcesTried": ["steam_direct", "depot_source", "manifesthub"],
            }),
            serde_json::json!({
                "depotId": "732", "success": true,
                "sourcesTried": ["steam_direct", "not_a_real_source", "../../etc/passwd"],
            }),
        ];
        let tried = tried_from_results(&results);
        assert_eq!(tried.iter().filter(|t| **t == "steam_direct").count(), 2);
        assert!(tried.contains(&"manifesthub"));
        assert!(!tried.iter().any(|t| t.contains("passwd")));
        assert_eq!(tried.len(), 4);
    }

    #[test]
    fn tried_counts_one_attempt_per_depot_not_one_per_kind() {
        let depots = vec![
            DepotDiag::failed_at(Stage::ManifestFetch, NOT_FOUND, SourceKind::DepotSource),
            DepotDiag::failed_at(Stage::ManifestFetch, NOT_FOUND, SourceKind::DepotSource),
            DepotDiag::ok(SourceKind::SteamDirect),
        ];
        let tried = tried_from_diags(&depots);
        let v = summarize(&depots, &tried, 3, "ddm");
        assert_eq!(v["sources_tried"]["depot_source"], 2);
        assert_eq!(v["sources_tried"]["steam_direct"], 1);
    }

    #[test]
    fn a_cached_manifest_is_not_reported_as_a_steam_download() {
        let results = vec![serde_json::json!({
            "depotId": "731", "success": true, "sourcesTried": ["cached"],
        })];
        let diags = from_download_results(&results, SourceKind::SteamDirect);
        let v = summarize(&diags, &tried_from_results(&results), 3, "native");
        assert_eq!(v["source_ok"], "cached");
    }

    #[test]
    fn a_fallback_success_is_credited_to_the_source_that_worked() {
        let results = vec![serde_json::json!({
            "depotId": "731", "success": true,
            "sourcesTried": ["steam_direct", "depot_source"],
        })];
        let diags = from_download_results(&results, SourceKind::SteamDirect);
        let v = summarize(&diags, &tried_from_results(&results), 3, "native");
        assert_eq!(v["source_ok"], "depot_source");
        assert_eq!(v["sources_tried"]["steam_direct"], 1);
        assert_eq!(v["sources_tried"]["depot_source"], 1);
    }

    #[test]
    fn without_a_chain_the_engine_default_still_applies() {
        let results = vec![serde_json::json!({ "depotId": "731", "success": true })];
        let diags = from_download_results(&results, SourceKind::SteamDirect);
        let v = summarize(&diags, &[], 3, "native");
        assert_eq!(v["source_ok"], "steam_direct");
    }

    #[test]
    fn results_without_a_tried_list_are_not_an_error() {
        let results = vec![serde_json::json!({ "depotId": "731", "success": true })];
        assert!(tried_from_results(&results).is_empty());
    }

    #[test]
    fn every_stage_label_is_reachable_from_a_real_error_string() {
        let cases = [
            ("Anonymous Steam login failed: bad creds", Stage::Login),
            ("Steam PICS query timed out after 20s", Stage::Pics),
            ("GetManifestRequestCode failed: 404", Stage::ManifestCode),
            ("manifest CDN request failed: reset", Stage::ManifestFetch),
            ("protobuf parse failed: eof", Stage::ManifestDecode),
            ("no depot key for depot 731", Stage::DepotKey),
            ("GetCDNAuthToken failed: denied", Stage::CdnToken),
            ("chunk CDN returned HTTP error: 503", Stage::Chunk),
            ("open file failed: EACCES", Stage::Disk),
            ("DepotDownloader exited with non-zero code for depot 731", Stage::Ddm),
            ("no manifest sources configured", Stage::SourceProbe),
        ];
        for (text, want) in cases {
            assert_eq!(classify_pipeline_error(text).0, want, "for {:?}", text);
        }
    }

    #[test]
    fn chunk_failures_are_not_mistaken_for_manifest_decode() {
        for text in [
            "chunk decode failed: AES-256-CBC decrypt failed: bad padding",
            "chunk decode failed: LZMA decompress failed: eof",
            "chunk decode failed: unrecognised chunk container magic: 5A 5A",
            "chunk size mismatch after decompress: got 10, expected 20",
        ] {
            assert_eq!(
                classify_pipeline_error(text).0,
                Stage::Chunk,
                "for {:?}",
                text
            );
        }
    }

    #[test]
    fn manifest_side_crypto_still_reports_manifest_decode() {
        assert_eq!(
            classify_pipeline_error("AES-256-ECB IV decrypt failed: bad key").0,
            Stage::ManifestDecode
        );
        assert_eq!(
            classify_pipeline_error("magic mismatch: got 0x1234").0,
            Stage::ManifestDecode
        );
    }

    #[test]
    fn missing_fallback_outranks_the_upstream_error_it_wraps() {
        let (stage, class) =
            classify_pipeline_error("No ManifestHub API key (manifest CDN request failed: reset)");
        assert_eq!(class, NO_KEY);
        assert_eq!(stage, Stage::ManifestFetch);

        let (_, class2) = classify_pipeline_error(
            "no manifest sources configured and No ManifestHub API key (chunk CDN request failed)",
        );
        assert_eq!(class2, NO_SOURCES);
    }

    #[test]
    fn manifesthub_rate_limiting_is_visible_on_every_engine() {
        for text in [
            "ManifestHub API error for depot 731: 429",
            "request failed: error code: 1015",
            "Too Many Requests",
        ] {
            assert_eq!(
                classify_pipeline_error(text).1,
                RATE_LIMITED,
                "for {:?}",
                text
            );
        }
    }

    #[test]
    fn a_byte_count_containing_429_is_not_a_rate_limit() {
        for text in [
            "chunk size mismatch after decompress: got 11429, expected 20",
            "write failed: only 4291 of 8192 bytes",
        ] {
            assert_ne!(
                classify_pipeline_error(text).1,
                RATE_LIMITED,
                "for {:?}",
                text
            );
        }
    }

    #[test]
    fn depot_key_problems_are_their_own_stage() {
        for text in [
            "depot 731 has no key",
            "depot 731 key hex invalid",
            "depot 731 key length != 32 bytes",
        ] {
            assert_eq!(classify_pipeline_error(text).0, Stage::DepotKey, "for {:?}", text);
        }
    }

    #[test]
    fn unmapped_errors_report_unknown_rather_than_guessing() {
        let (stage, class) = classify_pipeline_error("something nobody anticipated");
        assert_eq!(stage, Stage::Unknown);
        assert_eq!(class, UNKNOWN);
    }

    #[test]
    fn empty_job_has_no_reason_rather_than_a_wrong_one() {
        let v = summarize(&[], &[], 0, "native");
        assert_eq!(v["outcome"], "failed");
        assert!(v["fail_stage"].is_null());
        assert!(v["source_ok"].is_null());
    }
}
