function initProgressUI(depots) {
  els.progressBarFill.style.width = '0%';
  els.progressStatus.textContent = window.i18n.t('progress.initializing');
  els.terminalOutput.innerHTML = '';
  els.completionMessage.classList.add('hidden');
  state.downloadFailed = false;
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  els.btnCancel.classList.remove('hidden');
  els.btnCancel.disabled = false;
  els.btnCancel.innerHTML = `${ICONS.x} <span data-i18n="progress.cancel">${escapeHtml(window.i18n.t('progress.cancel'))}</span>`;
  state.paused = false;
  state.lastSkippedShown = 0;
  resetTaskbarProgress();
  setTaskbarProgress(0);
  const isNative = state.currentEngine === 'native';
  if (els.btnPause) {
    els.btnPause.classList.toggle('hidden', !isNative);
    els.btnPause.textContent = window.i18n.t('progress.pause');
    els.btnPause.disabled = false;
  }
  if (els.nativeAlphaBanner) {
    els.nativeAlphaBanner.classList.toggle('hidden', !isNative);
  }
  els.diskSpaceInfo.classList.add('hidden');
  if (els.depotProgressFill) els.depotProgressFill.style.width = '0%';
  if (els.depotProgressText) els.depotProgressText.textContent = '0%';
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;

  els.depotProgressList.innerHTML = '';
  depots.forEach((depot) => {
    const item = document.createElement('div');
    item.className = 'depot-progress-item';
    item.id = `depot-progress-${depot.depotId}`;
    item.innerHTML = `
      <div class="depot-progress-item__icon depot-progress-item__icon--pending">●</div>
      <div class="depot-progress-item__label">Depot ${escapeHtml(String(depot.depotId))}</div>
      <div class="depot-progress-item__status">${escapeHtml(window.i18n.t('progress.depotWaiting'))}</div>
    `;
    els.depotProgressList.appendChild(item);
  });
}

async function connectProgressListener() {
  if (state.unlistenProgress) {
    state.unlistenProgress();
    state.unlistenProgress = null;
  }

  const unlisten = await listen('download-progress', (event) => {
    handleProgressMessage(event.payload);
  });
  state.unlistenProgress = unlisten;
  appendTerminalLine(window.i18n.t('progress.connected'), 'info');
}

function cleanupProgressListener() {
  if (state.unlistenProgress) {
    state.unlistenProgress();
    state.unlistenProgress = null;
  }
}

function handleProgressMessage(msg) {
  if (msg.jobId && !state.jobId && state.dlStartedAt) {
    state.jobId = msg.jobId;
  }
  if (msg.jobId && msg.jobId !== state.jobId) {
    return;
  }
  switch (msg.type) {
    case 'status':
      if (msg.step === 'disk_space') {
        showDiskSpace(msg.freeGB, msg.drive);
      } else {
        handleStatusUpdate(msg);
      }
      break;

    case 'output':
      handleOutput(msg);
      break;

    case 'depot_complete':
      updateDepotStatus(msg.depotId, 'done', window.i18n.t('progress.depotComplete'));
      updateOverallProgress(msg.current, msg.total);
      updateDepotDownloadProgress(100);
      break;

    case 'manifest_source':
      handleManifestSource(msg);
      break;

    case 'complete':
      handleComplete(msg);
      break;

    case 'error':
      handleError(msg);
      break;

    case 'cancelled':
      handleCancelled(msg);
      break;
  }
}

const PIPELINE_STEPS = [
  'checking_branch', 'branch_found', 'downloading_keyvdf', 'downloading_manifests',
  'downloading_manifest', 'downloading_manifest_hub', 'manifest_hub_rate_limited',
  'generating_keys', 'keys_generated', 'starting_downloader', 'running_downloader',
  'paused', 'resumed',
];

function handleStatusUpdate(msg) {
  if (typeof msg.step === 'string' && PIPELINE_STEPS.includes(msg.step)) {
    state.dlStage = msg.step;
  }
  switch (msg.step) {
    case 'checking_branch':
      els.progressStatus.textContent = window.i18n.t('progress.checkingBranch', { appId: msg.appId });
      appendTerminalLine(window.i18n.t('progress.checkingBranch', { appId: msg.appId }), 'info');
      break;

    case 'branch_found':
      appendTerminalLine(`✓ ${msg.key ? eventText(msg) : window.i18n.t('progress.branchFound', { info: msg.lastUpdated || '' })}`, 'success');
      break;

    case 'manifest_hub_rate_limited':
    case 'retrying_depot':
    case 'depot_up_to_date':
    case 'removed_stale_files':
      if (msg.message) appendTerminalLine(eventText(msg), msg.step === 'manifest_hub_rate_limited' || msg.step === 'retrying_depot' ? 'warn' : 'info');
      break;

    case 'downloading_manifests':
      els.progressStatus.textContent = window.i18n.t('progress.manifestsStart', { total: msg.total });
      break;

    case 'downloading_manifest':
      if (msg.current && msg.total) {
        els.progressStatus.textContent = window.i18n.t('progress.manifestOne', { current: msg.current, total: msg.total, depotId: msg.depotId });
        updateOverallProgress(msg.current - 1, msg.total * 2);
      }
      updateDepotStatus(msg.depotId, 'active', window.i18n.t('progress.depotManifest'));
      if (msg.filename) {
        appendTerminalLine(window.i18n.t('progress.downloadingFile', { file: msg.filename }), 'info');
      }
      break;

    case 'downloading_manifest_hub':
      els.progressStatus.textContent = window.i18n.t('progress.customManifest', { depotId: msg.depotId });
      updateDepotStatus(msg.depotId, 'active', window.i18n.t('progress.customManifestStatus', { id: msg.manifestId }));
      appendTerminalLine(window.i18n.t('progress.customManifestLine', { depotId: msg.depotId, manifestId: msg.manifestId }), 'info');
      break;

    case 'generating_keys':
      els.progressStatus.textContent = window.i18n.t('progress.generatingKeys');
      appendTerminalLine(window.i18n.t('progress.generatingKeys'), 'info');
      break;

    case 'keys_generated':
      appendTerminalLine(`✓ ${window.i18n.t('progress.keysGenerated', { count: msg.depotCount })}`, 'success');
      break;

    case 'starting_downloader':
      els.progressStatus.textContent = window.i18n.t('progress.runningDdmStart', { total: msg.total });
      break;

    case 'running_downloader':
      state.speedTracker.samples = [];
      state.speedTracker.byteSamples = [];
      state.speedTracker.lastTime = 0;
      state.speedTracker.lastPercent = 0;
      state.speedTracker.depotStartTime = Date.now();
      state.speedTracker.lastUpdateTime = Date.now();
      clearInterval(state.speedTracker.staleTimer);
      state.speedTracker.staleTimer = null;
      startStaleTimer();
      if (msg.depotId) {
        const depot = state.parsedData?.depots?.find(d => d.depotId === String(msg.depotId));
        state.speedTracker.currentDepotSize = depot?.sizeBytes || 0;
        state.speedTracker.currentDepotId = msg.depotId;
      }
      if (msg.current && msg.total) {
        taskbar.depotIndex = msg.current;
        taskbar.depotTotal = msg.total;
        taskbarDepotProgress(0);
        els.progressStatus.textContent = window.i18n.t('progress.runningDdm', { current: msg.current, total: msg.total, depotId: msg.depotId });
        const baseProgress = state.parsedData ? state.selectedDepots.size : 0;
        updateOverallProgress(baseProgress + msg.current - 1, baseProgress + msg.total);
      }
      updateDepotStatus(msg.depotId, 'active', window.i18n.t('progress.depotDownloading'));
      if (msg.command) {
        appendTerminalLine(`> ${msg.key ? eventText(msg) : msg.command}`, 'info');
      }
      break;
  }
}

function eventText(msg) {
  if (!msg) return '';
  if (msg.key) {
    const params = {};
    for (const [name, value] of Object.entries(msg.params || {})) {
      params[name] = name === 'error' || name === 'reason' ? window.i18n.localizeError(value) : value;
    }
    const text = window.i18n.t(msg.key, params);
    if (text !== msg.key) return text;
  }
  return window.i18n.localizeError(msg.message || '');
}

function handleManifestSource(msg) {
  const classes = {
    manifesthub_fallback: 'warn',
    manifesthub_unavailable: 'stderr',
  };
  const labelKey = `events.src.label.${msg.source}`;
  const label = window.i18n.t(labelKey);
  const text = window.i18n.t('events.line', {
    label: label === labelKey ? (msg.source || '?') : label,
    depot: msg.depotId,
    text: eventText(msg),
  });
  appendTerminalLine(text, classes[msg.source] || 'info');
  if (msg.source === 'manifesthub_unavailable') {
    state.suggestMhKey = true;
  }
}

function handleOutput(msg) {
  const cls = msg.stream === 'stderr' ? 'stderr' : 'stdout';
  const text = msg.output || msg.line;
  if (msg.completedBytes != null && msg.totalBytes != null && msg.totalBytes > 0) {
    updateDepotDownloadProgressBytes(
      msg.percent ?? (msg.completedBytes * 100 / msg.totalBytes),
      msg.completedBytes,
      msg.totalBytes,
      msg.networkBytes
    );
    if (msg.skippedChunks != null && msg.skippedChunks > 0) {
      const lastShown = state.lastSkippedShown || 0;
      const milestone = Math.floor(msg.skippedChunks / 25);
      if (milestone > lastShown) {
        state.lastSkippedShown = milestone;
        const mb = (msg.skippedBytes / (1024 * 1024)).toFixed(1);
        appendTerminalLine(
          `✓ ${window.i18n.t('progress.resumedChunks', { count: msg.skippedChunks, mb })}`,
          'success'
        );
      }
    }
    return;
  }
  if (text) {
    appendTerminalLine(text, cls);
    const percentMatch = text.match(/^\s*(\d{1,3}(?:\.\d{1,2})?)%/);
    if (percentMatch) {
      const percent = parseFloat(percentMatch[1]);
      updateDepotDownloadProgress(percent);
    }
  }
}

const taskbar = { key: '', depotIndex: 0, depotTotal: 0, percent: null };

function setTaskbarProgress(percent) {
  const p = percent == null ? null : Math.max(0, Math.min(100, Math.floor(percent)));
  taskbar.percent = p;
  const paused = p != null && !!state.paused;
  const key = `${p}|${paused}`;
  if (key === taskbar.key) return;
  taskbar.key = key;
  const base = window.i18n.t('appName');
  const title = p == null
    ? base
    : `${p}%${paused ? ` (${window.i18n.t('progress.pausedShort')})` : ''} · ${state.gameName ? `${state.gameName} – ` : ''}${base}`;
  invoke('set_download_progress', { percent: p, paused, title }).catch(() => {});
}

function resetTaskbarProgress() {
  taskbar.depotIndex = 0;
  taskbar.depotTotal = 0;
  setTaskbarProgress(null);
}

function taskbarDepotProgress(depotPercent) {
  const total = taskbar.depotTotal || state.dlDepotCount || 1;
  const done = Math.max(taskbar.depotIndex - 1, 0);
  setTaskbarProgress(((done + Math.min(depotPercent, 100) / 100) / total) * 100);
}

function updateDepotDownloadProgressBytes(percent, completedBytes, totalBytes, networkBytes) {
  taskbarDepotProgress(percent);
  if (els.depotProgressFill) {
    els.depotProgressFill.style.width = `${Math.min(percent, 100)}%`;
  }
  if (els.depotProgressText) {
    const sizePart = (completedBytes != null && totalBytes != null && totalBytes > 0)
      ? ` — ${formatBytes(completedBytes)} / ${formatBytes(totalBytes)}`
      : '';
    els.depotProgressText.textContent = `${percent.toFixed(1)}%${sizePart}`;
  }
  updateSpeedAndEtaBytes(completedBytes, totalBytes, networkBytes);
}

const SPEED_WINDOW_MS = 3000;
const SPEED_MIN_WINDOW_MS = 1500;
const ETA_DISPLAY_INTERVAL_MS = 1500;

function updateSpeedAndEtaBytes(completedBytes, totalBytes, networkBytes) {
  const now = Date.now();
  const tracker = state.speedTracker;
  tracker.lastUpdateTime = now;

  if (!tracker.byteSamples) tracker.byteSamples = [];
  const speedSource = networkBytes != null ? networkBytes : completedBytes;
  tracker.byteSamples.push({
    bytes: speedSource,
    decompressed: completedBytes,
    time: now,
  });
  const cutoff = now - SPEED_WINDOW_MS;
  while (tracker.byteSamples.length > 2 && tracker.byteSamples[0].time < cutoff) {
    tracker.byteSamples.shift();
  }
  if (tracker.byteSamples.length < 2) return;

  const oldest = tracker.byteSamples[0];
  const newest = tracker.byteSamples[tracker.byteSamples.length - 1];
  const timeDelta = newest.time - oldest.time;
  const networkDelta = newest.bytes - oldest.bytes;
  const decompressedDelta = newest.decompressed - oldest.decompressed;
  if (timeDelta < SPEED_MIN_WINDOW_MS || networkDelta <= 0) return;

  const networkBytesPerSecond = networkDelta / (timeDelta / 1000);
  const decompressedBytesPerSecond = decompressedDelta / (timeDelta / 1000);
  const remainingBytes = Math.max(0, totalBytes - completedBytes);
  const etaSeconds =
    decompressedBytesPerSecond > 0 ? remainingBytes / decompressedBytesPerSecond : Infinity;

  const lastDisplay = tracker.lastDisplayMs || 0;
  if (now - lastDisplay < ETA_DISPLAY_INTERVAL_MS) return;
  tracker.lastDisplayMs = now;

  let speedText;
  const mbps = networkBytesPerSecond / (1024 * 1024);
  if (mbps >= 1) {
    speedText = `↓ ${mbps.toFixed(1)} MB/s`;
  } else {
    speedText = `↓ ${(networkBytesPerSecond / 1024).toFixed(0)} KB/s`;
  }

  const infoEl = els.downloadSpeedInfo;
  if (infoEl) {
    infoEl.classList.remove('hidden');
    els.downloadSpeed.textContent = speedText;
    els.downloadEta.textContent = formatEta(etaSeconds);
  }
}

function updateDepotDownloadProgress(percent) {
  taskbarDepotProgress(percent);
  if (els.depotProgressFill) {
    els.depotProgressFill.style.width = `${Math.min(percent, 100)}%`;
  }
  if (els.depotProgressText) {
    els.depotProgressText.textContent = `${percent.toFixed(1)}%`;
  }
  updateSpeedAndEta(percent);
}

function updateSpeedAndEta(percent) {
  const now = Date.now();
  const tracker = state.speedTracker;

  tracker.lastUpdateTime = now;

  if (tracker.lastTime === 0) {
    tracker.lastTime = now;
    tracker.lastPercent = percent;
    return;
  }

  tracker.samples.push({ percent, time: now });

  if (tracker.samples.length > 10) {
    tracker.samples.shift();
  }

  if (tracker.samples.length < 2) return;

  const oldest = tracker.samples[0];
  const newest = tracker.samples[tracker.samples.length - 1];
  const timeDelta = (newest.time - oldest.time) / 1000; // seconds
  const percentDelta = newest.percent - oldest.percent;

  if (timeDelta <= 0 || percentDelta <= 0) return;

  const percentPerSecond = percentDelta / timeDelta;
  const remainingPercent = 100 - percent;
  const etaSeconds = remainingPercent / percentPerSecond;

  const lastDisplay = tracker.lastDisplayMs || 0;
  if (now - lastDisplay < ETA_DISPLAY_INTERVAL_MS) return;
  tracker.lastDisplayMs = now;

  let speedText = '';
  if (tracker.currentDepotSize > 0) {
    const bytesPerSecond = (tracker.currentDepotSize * percentDelta / 100) / timeDelta;
    const mbPerSecond = bytesPerSecond / (1024 * 1024);
    speedText = `↓ ${mbPerSecond.toFixed(1)} MB/s`;
  } else {
    speedText = `↓ ${percentPerSecond.toFixed(2)}%/s`;
  }

  const etaText = formatEta(etaSeconds);

  const infoEl = els.downloadSpeedInfo;
  if (infoEl) {
    infoEl.classList.remove('hidden');
    els.downloadSpeed.textContent = speedText;
    els.downloadEta.textContent = etaText;
  }
}

function formatEta(seconds) {
  if (!isFinite(seconds) || seconds < 0 || seconds > 86400) {
    return 'calculating...';
  }

  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) {
    return `~${h}h ${m}m remaining`;
  } else if (m > 0) {
    return `~${m}m ${s}s remaining`;
  } else {
    return `~${s}s remaining`;
  }
}

function startStaleTimer() {
  if (state.speedTracker.staleTimer) return; // Already running

  state.speedTracker.staleTimer = setInterval(() => {
    const now = Date.now();
    const timeSinceLastUpdate = (now - state.speedTracker.lastUpdateTime) / 1000;

    if (timeSinceLastUpdate > 5) {
      const waitingFor = (now - state.speedTracker.lastUpdateTime) / 1000;
      const infoEl = els.downloadSpeedInfo;
      if (infoEl) {
        infoEl.classList.remove('hidden');
        els.downloadSpeed.textContent = window.i18n.t('progress.largeFile');
        els.downloadEta.textContent = window.i18n.t('progress.waitingFor', { time: formatElapsed(waitingFor) });
      }
    }
  }, 1000);
}

function formatElapsed(seconds) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);

  if (h > 0) return `${h}h ${m}m ${s}s`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function newEntryId() {
  if (window.crypto && typeof window.crypto.randomUUID === 'function') {
    return window.crypto.randomUUID();
  }
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

function buildPendingHistoryEntry(msg) {
  if (!state.downloadDir) return;
  const results = Array.isArray(msg && msg.results) ? msg.results : [];
  const total = results.length || (state.parsedData ? state.selectedDepots.size : 0);
  const successCount = results.length
    ? results.filter(r => r.success).length
    : total;
  const status = total > 0 && successCount === total ? 'complete' : 'partial';
  const appId = state.parsedData && state.parsedData.mainAppId
    ? String(state.parsedData.mainAppId)
    : (state.searchAppId ? String(state.searchAppId) : '');
  const depotIds = results.length
    ? results.map(r => String(r.depotId)).filter(Boolean)
    : Array.from(state.selectedDepots);
  state.pendingHistoryEntry = {
    id: newEntryId(),
    app_id: appId,
    game_name: state.gameName || null,
    header_image: state.headerImage || null,
    depot_count: total,
    depots_downloaded: successCount,
    status,
    download_dir: state.downloadDir,
    started_at: state.downloadStartedAt || new Date().toISOString(),
    completed_at: new Date().toISOString(),
    source_repo: state.searchRepo || null,
    depot_ids: depotIds.map(String),
  };
}

async function commitPendingHistory() {
  const entry = state.pendingHistoryEntry;
  if (!entry) return;
  state.pendingHistoryEntry = null;
  try {
    await invoke('record_history_entry', { entry });
  } catch (e) {
    console.error('record_history_entry failed:', e);
  }
}

function handleComplete(msg) {
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;
  const outcome = (msg.diag && msg.diag.outcome) || 'complete';
  const allOk = outcome === 'complete';

  els.progressBarFill.style.width = '100%';
  els.progressStatus.innerHTML = allOk
    ? `<span class="status-success">${ICONS.checkCircle} ${escapeHtml(window.i18n.t('progress.statusComplete'))}</span>`
    : `<span class="status-warning">${ICONS.alertTriangle} ${escapeHtml(window.i18n.t('progress.statusIncomplete'))}</span>`;
  updateDepotDownloadProgress(100);
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  emitEvent('download_completed', Object.assign({ success: allOk }, jobContext(), msg.diag || {}));
  buildPendingHistoryEntry(msg);
  const summary = eventText(msg);
  showCompletion(allOk, summary);

  if (msg.results) {
    const results = Array.isArray(msg.results) ? msg.results : [];
    results.forEach((r) => {
      updateDepotStatus(r.depotId, r.success ? 'done' : 'error', window.i18n.t(r.success ? 'progress.depotComplete' : 'progress.depotFailed'));
    });
  }

  appendTerminalLine(`\n${summary}`, allOk ? 'success' : 'error');
  if (state.repairRunning) {
    const results = Array.isArray(msg.results) ? msg.results : [];
    const repaired = results.reduce((sum, r) => sum + (Number(r.downloadedBytes) || 0), 0);
    appendTerminalLine(
      repaired > 0
        ? `✓ ${window.i18n.t('progress.repairDone', { size: formatBytes(repaired) })}`
        : `✓ ${window.i18n.t('progress.repairClean')}`,
      'success'
    );
  }

  const gameName = state.gameName || window.i18n.t('common.game');
  if (allOk) {
    showBrowserNotification(window.i18n.t('notifications.completeTitle'), window.i18n.t('notifications.completeBody', { name: gameName }), state.headerImage);
  } else {
    showBrowserNotification(window.i18n.t('notifications.partialTitle'), summary || window.i18n.t('notifications.partialBody', { name: gameName }), state.headerImage);
  }
  playNotificationSound();

  cleanupProgressListener();
  checkEmulatorSupport();
  maybeScheduleShutdown();
  queueAfterDownload(allOk ? 'complete' : 'partial');
}

function handleError(msg) {
  if (msg.depotId) {
    updateDepotStatus(msg.depotId, 'error', window.i18n.t('progress.depotError'));
  }
  const errorText = eventText(msg);
  appendTerminalLine(window.i18n.t('progress.errorLine', { message: errorText }), 'error');

  // If it's a fatal error (no depotId = pipeline-level error), show Start Over and notify
  if (!msg.depotId) {
    clearInterval(state.speedTracker.staleTimer);
    state.speedTracker.staleTimer = null;
    if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
    emitEvent('download_completed', Object.assign({ success: false }, jobContext(), msg.diag || {}, {
      err_key: msg.key || window.i18n.errorKey(msg.message) || 'unmatched',
    }));
    showCompletion(false, errorText);
    showBrowserNotification(window.i18n.t('notifications.failedTitle'), window.i18n.t('notifications.failedBody', { message: errorText }));
    playNotificationSound();
    cleanupProgressListener();
    maybeScheduleShutdown();
    queueAfterDownload('failed');
  }
}

function handleCancelled(msg) {
  clearInterval(state.speedTracker.staleTimer);
  state.speedTracker.staleTimer = null;
  els.progressBarFill.style.width = '0%';
  els.progressStatus.textContent = window.i18n.t('progress.cancelledStatus');
  if (els.downloadSpeedInfo) els.downloadSpeedInfo.classList.add('hidden');
  const localized = msg.step === 'cancelled_kept'
    ? window.i18n.t('progress.cancelledKept')
    : msg.step === 'cancelled_cleanup'
      ? window.i18n.t('progress.cancelledCleanup')
      : (msg.message ? eventText(msg) : window.i18n.t('progress.cancelledCleanup'));
  appendTerminalLine(`\n${localized}`, 'error');
  emitEvent('download_completed', Object.assign({ success: false }, jobContext(), {
    outcome: 'cancelled',
    depots_total: state.dlDepotCount ?? 0,
    last_stage: state.dlStage || 'unknown',
    duration_bucket: durationBucket(Date.now() - (state.dlStartedAt || Date.now())),
    engine: state.currentEngine || 'native',
  }, msg.diag || {}));
  showCompletion(false, localized);
  cleanupProgressListener();
  queueAfterDownload('cancelled');
}

function updateDepotStatus(depotId, status, text) {
  const item = document.getElementById(`depot-progress-${depotId}`);
  if (!item) return;

  const icon = item.querySelector('.depot-progress-item__icon');
  const statusEl = item.querySelector('.depot-progress-item__status');

  icon.className = 'depot-progress-item__icon';

  switch (status) {
    case 'active':
      icon.classList.add('depot-progress-item__icon--active');
      icon.textContent = '◉';
      break;
    case 'done':
      icon.classList.add('depot-progress-item__icon--done');
      icon.textContent = '✓';
      break;
    case 'error':
      icon.classList.add('depot-progress-item__icon--error');
      icon.textContent = '✗';
      break;
    default:
      icon.classList.add('depot-progress-item__icon--pending');
      icon.textContent = '●';
  }

  statusEl.textContent = text;
}

function updateOverallProgress(current, total) {
  if (total <= 0) return;
  const percent = Math.min(Math.round((current / total) * 100), 99);
  els.progressBarFill.style.width = `${percent}%`;
}

function appendTerminalLine(text, type = 'stdout') {
  const line = document.createElement('div');
  line.className = `terminal__line--${type}`;
  line.textContent = text;
  els.terminalOutput.appendChild(line);
  els.terminalOutput.scrollTop = els.terminalOutput.scrollHeight;
}

function showCompletion(success, message) {
  resetTaskbarProgress();
  state.jobId = null;
  els.completionMessage.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  els.completionMessage.classList.add(success ? 'completion-message--success' : 'completion-message--error');
  els.completionMessage.textContent = message;
  els.btnCancel.classList.add('hidden');
  if (els.btnPause) els.btnPause.classList.add('hidden');
  state.downloadFailed = !success;
  state.lastFailureText = success ? null : message;
  if (els.btnReportFailure) els.btnReportFailure.classList.toggle('hidden', success);
  if (els.btnNextStep) {
    els.btnNextStep.classList.remove('hidden');
    if (success) {
      updateNextButtonText();
    } else {
      els.btnNextStep.textContent = window.i18n.t('progress.backToSelection');
    }
  }
}

function showMhKeyRequiredHint() {
  let hint = document.getElementById('mh-apikey-required');
  if (!hint) {
    hint = document.createElement('p');
    hint.id = 'mh-apikey-required';
    hint.className = 'dd-path__hint dd-path__hint--error';
    const wrap = els.mhApiKey ? els.mhApiKey.closest('.settings-section') : null;
    if (wrap) wrap.appendChild(hint);
  }
  hint.textContent = window.i18n.t('select.manifestHubRequired');
  if (els.mhApiKey) {
    els.mhApiKey.classList.add('dd-path__input--error');
    els.mhApiKey.focus();
  }
}

function hideMhKeyRequiredHint() {
  const hint = document.getElementById('mh-apikey-required');
  if (hint) hint.remove();
  if (els.mhApiKey) els.mhApiKey.classList.remove('dd-path__input--error');
}

function showMhKeySuggestionHint() {
  hideMhKeyRequiredHint();
  let hint = document.getElementById('mh-apikey-required');
  if (!hint) {
    hint = document.createElement('p');
    hint.id = 'mh-apikey-required';
    hint.className = 'dd-path__hint dd-path__hint--error';
    const wrap = els.mhApiKey ? els.mhApiKey.closest('.settings-section') : null;
    if (wrap) wrap.appendChild(hint);
  }
  hint.textContent = window.i18n.t('select.manifestHubAfterFailure');
  if (els.mhApiKey) {
    els.mhApiKey.classList.add('dd-path__input--error');
    els.mhApiKey.focus();
  }
}

function activeUpdateDir(appId) {
  if (!state.updateDir || !state.updateAppId) return null;
  return String(appId) === state.updateAppId ? state.updateDir : null;
}

function activeRepair(appId) {
  return activeUpdateDir(appId) && state.repairManifests ? state.repairManifests : null;
}

function renderUpdateNotice(step) {
  const notice = document.getElementById('update-mode-notice');
  if (!notice) return;
  const appId = state.parsedData && state.parsedData.mainAppId;
  if (step === 2 && activeUpdateDir(appId)) {
    notice.innerHTML = window.i18n.t(activeRepair(appId) ? 'select.repairNotice' : 'select.updateNotice', { path: escapeHtml(state.updateDir) });
    notice.classList.remove('hidden');
  } else {
    notice.classList.add('hidden');
  }
}

function resetApp() {
  commitPendingHistory();
  state.updateDir = null;
  state.updateAppId = null;
  state.repairManifests = null;
  if (state.jobId) {
    const orphanJob = state.jobId;
    emitEvent('download_abandoned', abandonProps());
    invoke('cancel_download', { jobId: orphanJob }).catch((e) => {
      console.warn('orphan cancel_download failed:', e);
    });
  }
  clearJobTelemetryState();
  state.parsedData = null;
  state.selectedDepots.clear();
  state.jobId = null;
  state.gameName = null;
  state.headerImage = null;
  state.downloadDir = null;
  state.depotManifests = {};
  state.searchRepos = [];
  state.selectedRepo = null;
  state.searchAppId = null;
  state.searchSha = null;
  state.searchKeyVdfKeys = null;
  state.emulatorAvailable = false;
  state.emulatorScan = [];
  state.emuSelectedFiles = new Set();
  state.emuEditTargets = [];
  state.emuPatchedPaths = new Set();
  state.emuSettingsPrefillPath = null;
  state.emuBusy = false;
  state.emuApplyComplete = false;
  state.emuStandalone = false;
  state.emuSelectionExplicit = false;
  state.bypassInitialState = false;
  state.drmTargets = [];
  if (els.emuDrmSection) els.emuDrmSection.classList.add('hidden');
  if (els.emuDrmStatusWrap) els.emuDrmStatusWrap.classList.add('hidden');
  if (els.emuBypassToggle) els.emuBypassToggle.checked = false;
  setEmuEditMode(false);
  state.steamLibraryDetectedExes = [];
  if (els.steamExePath) els.steamExePath.value = '';
  if (els.steamGameName) els.steamGameName.value = '';
  if (els.steamLaunchOptions) els.steamLaunchOptions.value = '';
  if (els.steamLibraryResult) els.steamLibraryResult.classList.add('hidden');
  if (els.steamDetectedSection) els.steamDetectedSection.classList.add('hidden');
  resetSteamButtons();
  if (els.shortcutSteamLibrary) els.shortcutSteamLibrary.checked = false;
  cleanupProgressListener();
  if (els.shortcutStatus) els.shortcutStatus.classList.add('hidden');
  if (els.shortcutDetectedSection) els.shortcutDetectedSection.classList.add('hidden');
  if (els.shortcutDetectedList) els.shortcutDetectedList.innerHTML = '';
  if (els.shortcutExePath) els.shortcutExePath.value = '';
  state.shortcutsCreated = false;
  if (els.btnCreateShortcuts) { els.btnCreateShortcuts.disabled = false; els.btnCreateShortcuts.textContent = window.i18n.t('shortcut.createShortcuts'); }
  if (els.btnShortcutSkip) els.btnShortcutSkip.classList.remove('hidden');
  if (els.btnNextStep) els.btnNextStep.classList.add('hidden');
  if (els.emuApplyStatus) els.emuApplyStatus.classList.add('hidden');
  if (els.emuFileList) els.emuFileList.innerHTML = '';
  populateEmuSettings(null);
  els.gameInfoBanner.classList.add('hidden');
  els.gameInfoLoading.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchError.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  els.manifestLoading.classList.add('hidden');
  resetUpload();
  resetPatchOnlyTab();
  goToStep(1);
  if (!state.queueRunning) setTimeout(showPendingFollowupIfAny, 300);
}
