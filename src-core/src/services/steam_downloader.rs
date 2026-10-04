use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use sha1::{Digest, Sha1};
use tokio::fs;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::{Mutex, Semaphore};

use crate::services::manifest_code_provider::fetch_manifest_request_code_external;
use crate::services::steam_cdn;
use crate::services::steam_chunks::decode_chunk;
use crate::services::steam_manifest::{decode_manifest, DecodedManifest};
use crate::services::steam_session::{
    discover_cdn_servers, fetch_cdn_auth_token, fetch_manifest_request_code, CdnServer,
    SteamSession,
};

const CHUNK_RETRY_COUNT: usize = 8;

pub const DEFAULT_CHUNK_CONCURRENCY: u32 = 8;
pub const MAX_CHUNK_CONCURRENCY: u32 = 64;

fn effective_chunk_concurrency(setting: u32) -> usize {
    setting.clamp(1, MAX_CHUNK_CONCURRENCY) as usize
}

type FileLocks = Arc<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>>;

async fn file_lock_for(locks: &FileLocks, path: &Path) -> Arc<Mutex<()>> {
    let mut guard = locks.lock().await;
    if let Some(existing) = guard.get(path) {
        return existing.clone();
    }
    let lock = Arc::new(Mutex::new(()));
    guard.insert(path.to_path_buf(), lock.clone());
    lock
}

#[derive(Clone, Debug)]
pub struct NativeDownloadProgress {
    pub depot_id: u32,
    pub total_chunks: u64,
    pub completed_chunks: u64,
    pub total_bytes: u64,
    pub completed_bytes: u64,
    pub network_bytes: u64,
    pub skipped_chunks: u64,
    pub skipped_bytes: u64,
    pub percent: f64,
}

pub struct NativeDownloadOutcome {
    pub files_written: usize,
    pub bytes_written: u64,
    pub manifest_files: Vec<String>,
}

pub const RESUME_MARGIN: u64 = 64;
const CHECKPOINT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Clone, Debug, Default)]
pub struct ResumeOptions {
    pub checkpoint: Option<PathBuf>,
    pub manifest_id: String,
    pub trust_checkpoint: bool,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Checkpoint {
    manifest_id: String,
    total_chunks: u64,
    watermark: u64,
}

fn load_checkpoint(path: &Path, manifest_id: &str, total_chunks: u64) -> Option<u64> {
    let bytes = std::fs::read(path).ok()?;
    let cp: Checkpoint = serde_json::from_slice(&bytes).ok()?;
    (cp.manifest_id == manifest_id && cp.total_chunks == total_chunks)
        .then_some(cp.watermark.min(total_chunks))
}

fn trusted_end(watermark: u64) -> u64 {
    watermark.saturating_sub(RESUME_MARGIN)
}

struct CheckpointTracker {
    path: Option<PathBuf>,
    manifest_id: String,
    done: Vec<bool>,
    watermark: u64,
    saved_watermark: u64,
    last_save: Instant,
}

impl CheckpointTracker {
    fn new(path: Option<PathBuf>, manifest_id: String, total_chunks: u64) -> Self {
        Self {
            path,
            manifest_id,
            done: vec![false; total_chunks as usize],
            watermark: 0,
            saved_watermark: 0,
            last_save: Instant::now(),
        }
    }

    fn mark(&mut self, index: u64) {
        if let Some(slot) = self.done.get_mut(index as usize) {
            *slot = true;
        }
        while self.done.get(self.watermark as usize).copied().unwrap_or(false) {
            self.watermark += 1;
        }
        if self.watermark > self.saved_watermark
            && self.last_save.elapsed() >= CHECKPOINT_INTERVAL
        {
            self.save();
        }
    }

    fn save(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let cp = Checkpoint {
            manifest_id: self.manifest_id.clone(),
            total_chunks: self.done.len() as u64,
            watermark: self.watermark,
        };
        let Ok(json) = serde_json::to_vec(&cp) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() && std::fs::rename(&tmp, path).is_ok() {
            self.saved_watermark = self.watermark;
        }
        self.last_save = Instant::now();
    }
}

pub async fn download_depot_native(
    http: Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
    depot_key: [u8; 32],
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
    chunk_concurrency: u32,
    resume: ResumeOptions,
) -> Result<NativeDownloadOutcome, String> {
    let servers = discover_cdn_servers(session.clone(), 0, 20).await?;
    if servers.is_empty() {
        return Err("no CDN servers returned by Steam".to_string());
    }
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let request_code = resolve_manifest_request_code(
        &http,
        session.clone(),
        app_id,
        depot_id,
        manifest_id,
    )
    .await?;

    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".to_string());
    }

    let manifest = fetch_manifest_with_fallback(
        &http,
        &servers,
        session.clone(),
        app_id,
        depot_id,
        manifest_id,
        request_code,
        &depot_key,
    )
    .await?;

    download_chunks_from_manifest(
        http,
        session,
        servers,
        app_id,
        depot_id,
        depot_key,
        manifest,
        out_dir,
        cancel,
        pause,
        progress,
        chunk_concurrency,
        resume,
    )
    .await
}

pub async fn download_depot_from_local_manifest(
    http: Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    depot_key: [u8; 32],
    manifest_bytes: Vec<u8>,
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
    chunk_concurrency: u32,
    resume: ResumeOptions,
) -> Result<NativeDownloadOutcome, String> {
    let servers = discover_cdn_servers(session.clone(), 0, 20).await?;
    if servers.is_empty() {
        return Err("no CDN servers returned by Steam".to_string());
    }
    let manifest = decode_manifest(&manifest_bytes, &depot_key)?;
    download_chunks_from_manifest(
        http,
        session,
        servers,
        app_id,
        depot_id,
        depot_key,
        manifest,
        out_dir,
        cancel,
        pause,
        progress,
        chunk_concurrency,
        resume,
    )
    .await
}

async fn download_chunks_from_manifest(
    http: Client,
    session: Arc<SteamSession>,
    servers: Vec<CdnServer>,
    app_id: u32,
    depot_id: u32,
    depot_key: [u8; 32],
    manifest: DecodedManifest,
    out_dir: PathBuf,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    progress: impl Fn(NativeDownloadProgress) + Send + Sync + 'static,
    chunk_concurrency: u32,
    resume: ResumeOptions,
) -> Result<NativeDownloadOutcome, String> {
    crate::dlog!(
        "native",
        "depot {} download_chunks_with_manifest: pre-creating file structure",
        depot_id
    );
    let existing_files: std::collections::HashSet<PathBuf> = if resume.trust_checkpoint {
        manifest
            .payload
            .mappings
            .iter()
            .filter(|m| m.flags.unwrap_or(0) & 0x40 == 0)
            .filter_map(|m| m.filename.as_ref())
            .map(|rel| out_dir.join(sanitise_relative_path(rel)))
            .filter(|p| p.is_file())
            .collect()
    } else {
        Default::default()
    };
    pre_create_files(&manifest, &out_dir).await?;
    let manifest_files = crate::services::install_state::manifest_files(&manifest);

    let total_chunks: u64 = manifest
        .payload
        .mappings
        .iter()
        .map(|m| m.chunks.len() as u64)
        .sum();
    let total_bytes: u64 = manifest
        .payload
        .mappings
        .iter()
        .flat_map(|m| m.chunks.iter())
        .map(|c| c.cb_original.unwrap_or(0) as u64)
        .sum();
    crate::dlog!(
        "native",
        "depot {} manifest decoded: {} chunks, {:.2} GiB total uncompressed",
        depot_id,
        total_chunks,
        total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    );

    let progress = Arc::new(progress);
    let completed_chunks = Arc::new(Mutex::new(0u64));
    let completed_bytes = Arc::new(Mutex::new(0u64));
    let network_bytes = Arc::new(Mutex::new(0u64));
    let skipped_chunks = Arc::new(Mutex::new(0u64));
    let skipped_bytes = Arc::new(Mutex::new(0u64));
    let semaphore = Arc::new(Semaphore::new(effective_chunk_concurrency(
        chunk_concurrency,
    )));
    let file_locks: FileLocks = Arc::new(Mutex::new(HashMap::new()));
    let token_cache: Arc<Mutex<HashMap<String, Option<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let last_emit_ms = Arc::new(AtomicU64::new(0));
    let start_instant = Instant::now();

    let indexed: Vec<(PathBuf, u64)> = manifest
        .payload
        .mappings
        .iter()
        .filter(|m| m.flags.unwrap_or(0) & 0x40 == 0)
        .filter_map(|m| m.filename.as_ref().map(|rel| (m, rel)))
        .flat_map(|(m, rel)| {
            let file_path = out_dir.join(sanitise_relative_path(rel));
            m.chunks
                .iter()
                .filter(|c| c.sha.is_some())
                .map(move |c| (file_path.clone(), c.cb_original.unwrap_or(0) as u64))
        })
        .collect();
    let indexed_total = indexed.len() as u64;
    let watermark = if resume.trust_checkpoint {
        resume
            .checkpoint
            .as_deref()
            .and_then(|p| load_checkpoint(p, &resume.manifest_id, indexed_total))
    } else {
        None
    };
    let trusted_until = watermark.map(trusted_end).unwrap_or(0);
    let verify_range = watermark.map(|w| (trusted_until, w.saturating_add(RESUME_MARGIN)));
    let tracker = Arc::new(std::sync::Mutex::new(CheckpointTracker::new(
        resume.checkpoint.clone(),
        resume.manifest_id.clone(),
        indexed_total,
    )));
    let trusted: Arc<Vec<bool>> = Arc::new(
        indexed
            .iter()
            .enumerate()
            .map(|(i, (path, _))| (i as u64) < trusted_until && existing_files.contains(path))
            .collect(),
    );
    let (trusted_count, trusted_bytes) = indexed
        .iter()
        .zip(trusted.iter())
        .filter(|(_, ok)| **ok)
        .fold((0u64, 0u64), |(n, b), ((_, len), _)| (n + 1, b + len));
    if trusted_count > 0 {
        {
            let mut t = tracker.lock().unwrap_or_else(|e| e.into_inner());
            for (i, ok) in trusted.iter().enumerate() {
                if *ok {
                    t.mark(i as u64);
                }
            }
        }
        *completed_chunks.lock().await = trusted_count;
        *completed_bytes.lock().await = trusted_bytes;
        *skipped_chunks.lock().await = trusted_count;
        *skipped_bytes.lock().await = trusted_bytes;
        crate::dlog!(
            "native",
            "depot {} resuming from checkpoint: {} of {} chunks trusted",
            depot_id,
            trusted_count,
            indexed_total
        );
        progress(NativeDownloadProgress {
            depot_id,
            total_chunks,
            completed_chunks: trusted_count,
            total_bytes,
            completed_bytes: trusted_bytes,
            network_bytes: 0,
            skipped_chunks: trusted_count,
            skipped_bytes: trusted_bytes,
            percent: if total_bytes > 0 {
                trusted_bytes as f64 * 100.0 / total_bytes as f64
            } else {
                0.0
            },
        });
    }

    let verified_chunks = run_verify_phase(
        &manifest,
        &out_dir,
        depot_id,
        total_chunks,
        total_bytes,
        cancel.clone(),
        progress.clone(),
        completed_chunks.clone(),
        completed_bytes.clone(),
        skipped_chunks.clone(),
        skipped_bytes.clone(),
        last_emit_ms.clone(),
        start_instant,
        verify_range,
    )
    .await;
    crate::dlog!(
        "native",
        "depot {} verify phase done: {} chunks pre-validated",
        depot_id,
        verified_chunks.len()
    );

    let mut tasks = FuturesUnordered::new();
    let mut next_index: u64 = 0;
    for mapping in &manifest.payload.mappings {
        let Some(relative_path) = mapping.filename.clone() else {
            continue;
        };
        if mapping.flags.unwrap_or(0) & 0x40 != 0 {
            continue;
        }
        let file_path = out_dir.join(sanitise_relative_path(&relative_path));

        for chunk in &mapping.chunks {
            let Some(sha) = chunk.sha.as_ref() else {
                continue;
            };
            let index = next_index;
            next_index += 1;
            let tracker = tracker.clone();
            if trusted.get(index as usize).copied().unwrap_or(false) {
                let file_path = file_path.clone();
                let bytes = chunk.cb_original.unwrap_or(0) as u64;
                tasks.push(tokio::spawn(async move {
                    Ok(ChunkOutcome {
                        file_path,
                        bytes_written: bytes,
                        network_bytes: 0,
                        from_cache: true,
                        pre_accounted: true,
                    })
                }));
                continue;
            }
            let chunk_meta = ChunkJob {
                sha: sha.clone(),
                offset: chunk.offset.unwrap_or(0),
                cb_original: chunk.cb_original.unwrap_or(0),
                cb_compressed: chunk.cb_compressed.unwrap_or(0),
                file_path: file_path.clone(),
            };
            let http = http.clone();
            let session = session.clone();
            let servers = servers.clone();
            let depot_key = depot_key;
            let progress = progress.clone();
            let completed_chunks = completed_chunks.clone();
            let completed_bytes = completed_bytes.clone();
            let network_bytes = network_bytes.clone();
            let skipped_chunks = skipped_chunks.clone();
            let skipped_bytes = skipped_bytes.clone();
            let permit = semaphore.clone();
            let file_locks = file_locks.clone();
            let token_cache = token_cache.clone();
            let cancel = cancel.clone();
            let pause = pause.clone();
            let last_emit_ms = last_emit_ms.clone();
            let verified_chunks = verified_chunks.clone();

            if cancel.load(Ordering::SeqCst) {
                break;
            }

            tasks.push(tokio::spawn(async move {
                if cancel.load(Ordering::SeqCst) {
                    return Err("cancelled".to_string());
                }
                if verified_chunks.contains(&chunk_key(
                    &chunk_meta.file_path,
                    chunk_meta.offset,
                    &chunk_meta.sha,
                )) {
                    tracker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .mark(index);
                    return Ok(ChunkOutcome {
                        file_path: chunk_meta.file_path,
                        bytes_written: chunk_meta.cb_original as u64,
                        network_bytes: 0,
                        from_cache: true,
                        pre_accounted: true,
                    });
                }
                let _permit = permit.acquire_owned().await;
                if cancel.load(Ordering::SeqCst) {
                    return Err("cancelled".to_string());
                }
                let expected_bytes = if chunk_meta.cb_compressed > 0 {
                    chunk_meta.cb_compressed
                } else {
                    chunk_meta.cb_original
                } as u64;
                crate::services::speed_limit::throttle(expected_bytes, &cancel).await?;
                while pause.load(Ordering::SeqCst) {
                    if cancel.load(Ordering::SeqCst) {
                        return Err("cancelled".to_string());
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                let res = process_chunk(
                    http,
                    session,
                    &servers,
                    app_id,
                    depot_id,
                    &depot_key,
                    chunk_meta,
                    token_cache,
                    file_locks,
                )
                .await;
                if res.is_ok() {
                    tracker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .mark(index);
                }
                if let Ok(ref outcome) = res {
                    let (cc_val, cb_val, nb_val) = if outcome.pre_accounted {
                        let cc_val = *completed_chunks.lock().await;
                        let cb_val = *completed_bytes.lock().await;
                        let nb_val = *network_bytes.lock().await;
                        (cc_val, cb_val, nb_val)
                    } else {
                        let mut cc = completed_chunks.lock().await;
                        *cc += 1;
                        let cc_val = *cc;
                        drop(cc);
                        let mut cb = completed_bytes.lock().await;
                        *cb += outcome.bytes_written;
                        let cb_val = *cb;
                        drop(cb);
                        let mut nb = network_bytes.lock().await;
                        *nb += outcome.network_bytes;
                        let nb_val = *nb;
                        drop(nb);
                        if outcome.from_cache {
                            let mut sc = skipped_chunks.lock().await;
                            *sc += 1;
                            let mut sb = skipped_bytes.lock().await;
                            *sb += outcome.bytes_written;
                        }
                        (cc_val, cb_val, nb_val)
                    };

                    let elapsed_ms = start_instant.elapsed().as_millis() as u64;
                    let last = last_emit_ms.load(Ordering::Relaxed);
                    let is_final = cc_val == total_chunks;
                    let should_emit = is_final
                        || elapsed_ms.saturating_sub(last) >= 150
                            && last_emit_ms
                                .compare_exchange(
                                    last,
                                    elapsed_ms,
                                    Ordering::Relaxed,
                                    Ordering::Relaxed,
                                )
                                .is_ok();

                    if should_emit {
                        let skipped_chunks_val = *skipped_chunks.lock().await;
                        let skipped_bytes_val = *skipped_bytes.lock().await;
                        let percent = if total_bytes > 0 {
                            (cb_val as f64) * 100.0 / (total_bytes as f64)
                        } else {
                            0.0
                        };
                        progress(NativeDownloadProgress {
                            depot_id,
                            total_chunks,
                            completed_chunks: cc_val,
                            total_bytes,
                            completed_bytes: cb_val,
                            network_bytes: nb_val,
                            skipped_chunks: skipped_chunks_val,
                            skipped_bytes: skipped_bytes_val,
                            percent,
                        });
                    }
                }
                res
            }));
        }
    }

    crate::dlog!(
        "native",
        "depot {} chunk task queue built; awaiting completion",
        depot_id
    );

    let mut files_written_unique = std::collections::HashSet::new();
    let mut bytes_written = 0u64;
    while let Some(joined) = tasks.next().await {
        let chunk_outcome = match joined
            .map_err(|e| format!("chunk task join failed: {}", e))
            .and_then(|r| r)
        {
            Ok(outcome) => outcome,
            Err(e) => {
                tracker.lock().unwrap_or_else(|e| e.into_inner()).save();
                return Err(e);
            }
        };
        bytes_written += chunk_outcome.bytes_written;
        files_written_unique.insert(chunk_outcome.file_path);
    }
    if let Some(path) = &resume.checkpoint {
        let _ = std::fs::remove_file(path);
    }

    crate::dlog!(
        "native",
        "depot {} chunks complete: {} files touched, {} bytes written",
        depot_id,
        files_written_unique.len(),
        bytes_written
    );

    Ok(NativeDownloadOutcome {
        files_written: files_written_unique.len(),
        bytes_written,
        manifest_files,
    })
}

struct ChunkJob {
    sha: Vec<u8>,
    offset: u64,
    cb_original: u32,
    cb_compressed: u32,
    file_path: PathBuf,
}

struct ChunkOutcome {
    file_path: PathBuf,
    bytes_written: u64,
    network_bytes: u64,
    from_cache: bool,
    pre_accounted: bool,
}

async fn process_chunk(
    http: Client,
    session: Arc<SteamSession>,
    servers: &[CdnServer],
    app_id: u32,
    depot_id: u32,
    depot_key: &[u8; 32],
    job: ChunkJob,
    token_cache: Arc<Mutex<HashMap<String, Option<String>>>>,
    file_locks: FileLocks,
) -> Result<ChunkOutcome, String> {
    let sha_hex = hex::encode(&job.sha);

    let mut last_err: Option<String> = None;
    for attempt in 0..CHUNK_RETRY_COUNT {
        if attempt > 0 {
            let delay_ms = chunk_retry_backoff_ms(attempt);
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        let server = pick_server(servers, attempt);
        let token = get_or_fetch_token(
            &token_cache,
            session.clone(),
            &server.host,
            app_id,
            depot_id,
        )
        .await;
        let encrypted = match steam_cdn::fetch_encrypted_chunk(
            &http,
            &server.host,
            depot_id,
            &sha_hex,
            token.as_deref(),
            !server.https_support.is_empty() && server.https_support != "none",
        )
        .await
        {
            Ok(b) if b.len() as u32 == job.cb_compressed || job.cb_compressed == 0 => b,
            Ok(b) => {
                last_err = Some(format!(
                    "chunk size mismatch: got {} bytes, expected {}",
                    b.len(),
                    job.cb_compressed
                ));
                continue;
            }
            Err(e) => {
                if is_hard_chunk_error(&e) {
                    return Err(e);
                }
                last_err = Some(e);
                continue;
            }
        };
        let network_bytes = encrypted.len() as u64;

        let decoded = match decode_chunk(&encrypted, depot_key, job.cb_original) {
            Ok(d) => d,
            Err(e) => {
                last_err = Some(format!("chunk decode failed: {}", e));
                continue;
            }
        };

        let actual_sha = {
            let mut hasher = Sha1::new();
            hasher.update(&decoded);
            hasher.finalize().to_vec()
        };
        if actual_sha != job.sha {
            last_err = Some(format!(
                "chunk SHA mismatch: got {} expected {}",
                hex::encode(&actual_sha),
                sha_hex
            ));
            continue;
        }

        if let Some(parent) = job.file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create dir failed: {}", e))?;
        }
        let write_lock = file_lock_for(&file_locks, &job.file_path).await;
        let _held = write_lock.lock().await;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&job.file_path)
            .await
            .map_err(|e| format!("open file failed: {}", e))?;
        let _ = file
            .seek(std::io::SeekFrom::Start(job.offset))
            .await
            .map_err(|e| format!("seek failed: {}", e))?;
        file.write_all(&decoded)
            .await
            .map_err(|e| format!("write failed: {}", e))?;

        return Ok(ChunkOutcome {
            file_path: job.file_path,
            bytes_written: decoded.len() as u64,
            network_bytes,
            from_cache: false,
            pre_accounted: false,
        });
    }
    Err(last_err.unwrap_or_else(|| "chunk download exhausted retries".to_string()))
}

const VERIFY_FILE_CONCURRENCY: usize = 8;

pub type ChunkKey = (PathBuf, u64, Vec<u8>);

pub fn chunk_key(path: &Path, offset: u64, sha: &[u8]) -> ChunkKey {
    (path.to_path_buf(), offset, sha.to_vec())
}

async fn run_verify_phase(
    manifest: &DecodedManifest,
    out_dir: &Path,
    depot_id: u32,
    total_chunks: u64,
    total_bytes: u64,
    cancel: Arc<AtomicBool>,
    progress: Arc<dyn Fn(NativeDownloadProgress) + Send + Sync>,
    completed_chunks: Arc<Mutex<u64>>,
    completed_bytes: Arc<Mutex<u64>>,
    skipped_chunks: Arc<Mutex<u64>>,
    skipped_bytes: Arc<Mutex<u64>>,
    last_emit_ms: Arc<AtomicU64>,
    start_instant: Instant,
    range: Option<(u64, u64)>,
) -> Arc<std::collections::HashSet<ChunkKey>> {
    let mut by_file: HashMap<PathBuf, Vec<(u64, u32, Vec<u8>)>> = HashMap::new();
    let mut index: u64 = 0;
    for mapping in &manifest.payload.mappings {
        let Some(rel) = mapping.filename.as_ref() else {
            continue;
        };
        if mapping.flags.unwrap_or(0) & 0x40 != 0 {
            continue;
        }
        let file_path = out_dir.join(sanitise_relative_path(rel));
        for chunk in &mapping.chunks {
            let Some(sha) = chunk.sha.as_ref() else {
                continue;
            };
            let current = index;
            index += 1;
            if let Some((start, end)) = range {
                if current < start || current >= end {
                    continue;
                }
            }
            by_file.entry(file_path.clone()).or_default().push((
                chunk.offset.unwrap_or(0),
                chunk.cb_original.unwrap_or(0),
                sha.clone(),
            ));
        }
    }

    let verified: Arc<Mutex<std::collections::HashSet<ChunkKey>>> =
        Arc::new(Mutex::new(std::collections::HashSet::new()));
    let sem = Arc::new(Semaphore::new(VERIFY_FILE_CONCURRENCY));
    let mut tasks: FuturesUnordered<tokio::task::JoinHandle<()>> = FuturesUnordered::new();

    for (path, chunks) in by_file {
        if !path.exists() {
            continue;
        }
        let cancel = cancel.clone();
        let progress = progress.clone();
        let completed_chunks = completed_chunks.clone();
        let completed_bytes = completed_bytes.clone();
        let skipped_chunks = skipped_chunks.clone();
        let skipped_bytes = skipped_bytes.clone();
        let last_emit_ms = last_emit_ms.clone();
        let verified = verified.clone();
        let sem_clone = sem.clone();

        if cancel.load(Ordering::SeqCst) {
            break;
        }

        tasks.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire_owned().await;
            if cancel.load(Ordering::SeqCst) {
                return;
            }
            verify_one_file(
                path,
                chunks,
                depot_id,
                total_chunks,
                total_bytes,
                cancel,
                progress,
                completed_chunks,
                completed_bytes,
                skipped_chunks,
                skipped_bytes,
                last_emit_ms,
                start_instant,
                verified,
            )
            .await;
        }));
    }

    while tasks.next().await.is_some() {}

    let inner: std::collections::HashSet<ChunkKey> = match Arc::try_unwrap(verified) {
        Ok(mutex) => mutex.into_inner(),
        Err(arc) => arc.lock().await.clone(),
    };
    Arc::new(inner)
}

async fn verify_one_file(
    path: PathBuf,
    chunks: Vec<(u64, u32, Vec<u8>)>,
    depot_id: u32,
    total_chunks: u64,
    total_bytes: u64,
    cancel: Arc<AtomicBool>,
    progress: Arc<dyn Fn(NativeDownloadProgress) + Send + Sync>,
    completed_chunks: Arc<Mutex<u64>>,
    completed_bytes: Arc<Mutex<u64>>,
    skipped_chunks: Arc<Mutex<u64>>,
    skipped_bytes: Arc<Mutex<u64>>,
    last_emit_ms: Arc<AtomicU64>,
    start_instant: Instant,
    verified: Arc<Mutex<std::collections::HashSet<ChunkKey>>>,
) {
    use tokio::io::AsyncReadExt;
    let Ok(mut file) = tokio::fs::File::open(&path).await else {
        return;
    };
    let Ok(meta) = file.metadata().await else {
        return;
    };
    let file_len = meta.len();
    let mut sorted = chunks;
    sorted.sort_by_key(|(off, _, _)| *off);

    for (offset, len, expected_sha) in sorted {
        if cancel.load(Ordering::SeqCst) {
            return;
        }
        if offset + len as u64 > file_len {
            continue;
        }
        if file.seek(std::io::SeekFrom::Start(offset)).await.is_err() {
            continue;
        }
        let mut buf = vec![0u8; len as usize];
        if file.read_exact(&mut buf).await.is_err() {
            continue;
        }
        let mut hasher = Sha1::new();
        hasher.update(&buf);
        if hasher.finalize().to_vec() != expected_sha {
            continue;
        }

        {
            let mut set = verified.lock().await;
            set.insert(chunk_key(&path, offset, &expected_sha));
        }

        let mut cc = completed_chunks.lock().await;
        *cc += 1;
        let cc_val = *cc;
        drop(cc);
        let mut cb = completed_bytes.lock().await;
        *cb += len as u64;
        let cb_val = *cb;
        drop(cb);
        let mut sc = skipped_chunks.lock().await;
        *sc += 1;
        drop(sc);
        let mut sb = skipped_bytes.lock().await;
        *sb += len as u64;
        drop(sb);

        let elapsed_ms = start_instant.elapsed().as_millis() as u64;
        let last = last_emit_ms.load(Ordering::Relaxed);
        if elapsed_ms.saturating_sub(last) >= 150
            && last_emit_ms
                .compare_exchange(last, elapsed_ms, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let skipped_chunks_val = *skipped_chunks.lock().await;
            let skipped_bytes_val = *skipped_bytes.lock().await;
            let percent = if total_bytes > 0 {
                (cb_val as f64) * 100.0 / (total_bytes as f64)
            } else {
                0.0
            };
            progress(NativeDownloadProgress {
                depot_id,
                total_chunks,
                completed_chunks: cc_val,
                total_bytes,
                completed_bytes: cb_val,
                network_bytes: 0,
                skipped_chunks: skipped_chunks_val,
                skipped_bytes: skipped_bytes_val,
                percent,
            });
        }
    }
}

async fn get_or_fetch_token(
    cache: &Mutex<HashMap<String, Option<String>>>,
    session: Arc<SteamSession>,
    host: &str,
    app_id: u32,
    depot_id: u32,
) -> Option<String> {
    let key = format!("{}:{}:{}", host, app_id, depot_id);
    {
        let guard = cache.lock().await;
        if let Some(entry) = guard.get(&key) {
            return entry.clone();
        }
    }
    let token = fetch_cdn_auth_token(session, host, app_id, depot_id)
        .await
        .unwrap_or(None);
    let mut guard = cache.lock().await;
    guard.entry(key).or_insert_with(|| token.clone());
    token
}

async fn resolve_manifest_request_code(
    http: &Client,
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
) -> Result<u64, String> {
    match fetch_manifest_request_code(session, app_id, depot_id, manifest_id).await {
        Ok(code) if code != 0 => return Ok(code),
        Ok(_) => {
            eprintln!(
                "[manifest-code] Steam returned request_code=0 for depot {}, falling back to external providers",
                depot_id
            );
        }
        Err(e) => {
            eprintln!(
                "[manifest-code] Steam GetManifestRequestCode failed for depot {} ({}), falling back to external providers",
                depot_id, e
            );
        }
    }
    match fetch_manifest_request_code_external(http, manifest_id).await {
        Ok(resolved) => {
            eprintln!(
                "[manifest-code] resolved depot {} via {}",
                depot_id,
                resolved.source.label()
            );
            Ok(resolved.request_code)
        }
        Err(e) => Err(e),
    }
}

async fn fetch_manifest_with_fallback(
    http: &Client,
    servers: &[CdnServer],
    session: Arc<SteamSession>,
    app_id: u32,
    depot_id: u32,
    manifest_id: u64,
    request_code: u64,
    depot_key: &[u8; 32],
) -> Result<DecodedManifest, String> {
    let mut last_err: Option<String> = None;
    for (idx, server) in servers.iter().enumerate().take(5) {
        let token = fetch_cdn_auth_token(session.clone(), &server.host, app_id, depot_id)
            .await
            .unwrap_or(None);
        let use_https = !server.https_support.is_empty() && server.https_support != "none";
        match steam_cdn::fetch_decoded_manifest(
            http,
            &server.host,
            depot_id,
            manifest_id,
            request_code,
            token.as_deref(),
            depot_key,
            use_https,
        )
        .await
        {
            Ok(m) => return Ok(m),
            Err(e) => {
                last_err = Some(format!("server #{} ({}) failed: {}", idx, server.host, e));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "no servers tried".to_string()))
}

fn pick_server<'a>(servers: &'a [CdnServer], attempt: usize) -> &'a CdnServer {
    &servers[attempt % servers.len()]
}

fn chunk_retry_backoff_ms(attempt: usize) -> u64 {
    let base = 250u64 << attempt.min(7);
    base.min(15_000)
}

fn is_hard_chunk_error(err: &str) -> bool {
    err.contains(" 400 ")
        || err.contains(" 401 ")
        || err.contains(" 403 ")
        || err.contains(" 404 ")
        || err.contains(" 410 ")
}

fn sanitise_relative_path(p: &str) -> String {
    p.replace('\\', "/")
}

async fn pre_create_files(manifest: &DecodedManifest, out_dir: &Path) -> Result<(), String> {
    for mapping in &manifest.payload.mappings {
        let Some(rel) = mapping.filename.as_ref() else {
            continue;
        };
        let target = out_dir.join(sanitise_relative_path(rel));
        if mapping.flags.unwrap_or(0) & 0x40 != 0 {
            fs::create_dir_all(&target)
                .await
                .map_err(|e| format!("create dir failed: {}", e))?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("create parent dir failed: {}", e))?;
        }
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&target)
            .await
            .map_err(|e| format!("pre-create file failed: {}", e))?;
        let expected = mapping.size.unwrap_or(0);
        if expected > 0 {
            file.set_len(expected)
                .await
                .map_err(|e| format!("set_len failed: {}", e))?;
        }
    }
    Ok(())
}


#[cfg(test)]
mod verify_cache_tests {
    use super::*;

    #[test]
    fn same_content_at_a_second_offset_is_not_treated_as_already_written() {
        let mut verified = std::collections::HashSet::new();
        let file = PathBuf::from("/game/data.bin");
        let sha = vec![0xAA; 20];

        verified.insert(chunk_key(&file, 0, &sha));

        assert!(verified.contains(&chunk_key(&file, 0, &sha)));
        assert!(!verified.contains(&chunk_key(&file, 4096, &sha)));
    }

    #[test]
    fn same_content_in_a_second_file_is_not_treated_as_already_written() {
        let mut verified = std::collections::HashSet::new();
        let sha = vec![0xBB; 20];
        verified.insert(chunk_key(Path::new("/game/a.bin"), 0, &sha));

        assert!(!verified.contains(&chunk_key(Path::new("/game/b.bin"), 0, &sha)));
    }

    #[test]
    fn a_different_sha_at_a_known_slot_is_not_satisfied() {
        let mut verified = std::collections::HashSet::new();
        let file = PathBuf::from("/game/data.bin");
        verified.insert(chunk_key(&file, 512, &[0x01; 20]));

        assert!(!verified.contains(&chunk_key(&file, 512, &[0x02; 20])));
    }
}

#[cfg(test)]
mod chunk_concurrency_tests {
    use super::*;

    #[test]
    fn normal_values_pass_through_unchanged() {
        assert_eq!(effective_chunk_concurrency(1), 1);
        assert_eq!(effective_chunk_concurrency(8), 8);
        assert_eq!(effective_chunk_concurrency(32), 32);
        assert_eq!(effective_chunk_concurrency(64), 64);
    }

    #[test]
    fn zero_and_overflow_are_clamped_into_range() {
        assert_eq!(effective_chunk_concurrency(0), 1);
        assert_eq!(effective_chunk_concurrency(1000), 64);
        assert_eq!(effective_chunk_concurrency(u32::MAX), 64);
    }

    #[test]
    fn the_default_matches_the_documented_fallback() {
        assert_eq!(DEFAULT_CHUNK_CONCURRENCY, 8);
        assert_eq!(effective_chunk_concurrency(DEFAULT_CHUNK_CONCURRENCY), 8);
    }

    #[tokio::test]
    async fn the_same_file_resolves_to_the_same_write_lock() {
        let locks: FileLocks = Arc::new(Mutex::new(HashMap::new()));
        let path = Path::new("/game/data.bin");
        let first = file_lock_for(&locks, path).await;
        let second = file_lock_for(&locks, path).await;
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[tokio::test]
    async fn different_files_get_independent_write_locks() {
        let locks: FileLocks = Arc::new(Mutex::new(HashMap::new()));
        let first = file_lock_for(&locks, Path::new("/game/a.bin")).await;
        let second = file_lock_for(&locks, Path::new("/game/b.bin")).await;
        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[tokio::test]
    async fn a_released_write_lock_can_be_acquired_again() {
        let locks: FileLocks = Arc::new(Mutex::new(HashMap::new()));
        let lock = file_lock_for(&locks, Path::new("/game/data.bin")).await;
        {
            let _held = lock.lock().await;
        }
        assert!(lock.try_lock().is_ok());
    }
}

#[cfg(test)]
mod checkpoint_tests {
    use super::*;

    #[test]
    fn the_watermark_only_moves_over_a_gapless_prefix() {
        let mut t = CheckpointTracker::new(None, "1".into(), 6);
        t.mark(1);
        t.mark(2);
        assert_eq!(t.watermark, 0);
        t.mark(0);
        assert_eq!(t.watermark, 3);
        t.mark(5);
        assert_eq!(t.watermark, 3);
        t.mark(3);
        t.mark(4);
        assert_eq!(t.watermark, 6);
    }

    #[test]
    fn a_saved_checkpoint_is_only_used_for_the_same_manifest() {
        let dir = std::env::temp_dir().join(format!("smd-checkpoint-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".smd-depot-1.resume.json");
        let mut t = CheckpointTracker::new(Some(path.clone()), "777".into(), 200);
        for i in 0..150 {
            t.mark(i);
        }
        t.save();
        assert_eq!(load_checkpoint(&path, "777", 200), Some(150));
        assert_eq!(load_checkpoint(&path, "778", 200), None);
        assert_eq!(load_checkpoint(&path, "777", 201), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_last_chunks_before_the_watermark_are_checked_again() {
        assert_eq!(trusted_end(1000), 1000 - RESUME_MARGIN);
        assert_eq!(trusted_end(10), 0);
    }
}
