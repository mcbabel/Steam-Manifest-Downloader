async function initTelemetryConsent() {
  try {
    const status = await invoke('get_telemetry_status');
    if (status.consent === 'pending') {
      const modal = els.telemetryModal;
      await new Promise((resolve) => {
        const observer = new MutationObserver(() => {
          if (modal.classList.contains('hidden')) {
            observer.disconnect();
            resolve();
          }
        });
        observer.observe(modal, { attributes: true, attributeFilter: ['class'] });
        modal.classList.remove('hidden');
      });
    } else if (status.consent === 'accepted') {
      invoke('emit_telemetry_event', { kind: 'app_start', props: { locale: window.i18n.getCurrentLocale() } }).catch(() => {});
    }
  } catch (e) {
    console.error('Failed to load telemetry status:', e);
  }
}

async function acceptTelemetry() {
  try {
    await invoke('set_telemetry_consent', { accept: true });
    invoke('emit_telemetry_event', { kind: 'app_start', props: { locale: window.i18n.getCurrentLocale() } }).catch(() => {});
  } catch (e) {
    console.error('Failed to accept telemetry:', e);
  }
  els.telemetryModal.classList.add('hidden');
}

async function declineTelemetry() {
  try {
    await invoke('set_telemetry_consent', { accept: false });
  } catch (e) {
    console.error('Failed to decline telemetry:', e);
  }
  els.telemetryModal.classList.add('hidden');
}

function emitEvent(kind, props) {
  invoke('emit_telemetry_event', { kind, props: props ?? null }).catch(() => {});
}

const reportedErrors = new Set();

function reportError(area, raw, key) {
  const k = key || window.i18n.errorKey(raw) || 'unmatched';
  const id = `${area}:${k}`;
  if (reportedErrors.has(id) || reportedErrors.size >= 40) return;
  reportedErrors.add(id);
  emitEvent('error_shown', { area, key: k });
}

let jsErrorCount = 0;

function reportJsError(message, source, line) {
  if (jsErrorCount >= 5) return;
  jsErrorCount += 1;
  const file = String(source || '').split(/[\\/]/).pop() || 'inline';
  emitEvent('crash', {
    source: 'js',
    location: line ? `${file}:${line}` : file,
    message: window.i18n.errorKey(String(message || '')) || String(message || '').slice(0, 300),
    thread: 'main',
  });
}

window.addEventListener('error', (e) => {
  reportJsError(e.message || (e.error && e.error.message), e.filename, e.lineno);
});

window.addEventListener('unhandledrejection', (e) => {
  const reason = e.reason;
  const message = reason && reason.message ? reason.message : String(reason);
  const frame = reason && reason.stack ? /([^\s()]+\.js):(\d+)/.exec(reason.stack) : null;
  reportJsError(`unhandled: ${message}`, frame ? frame[1] : '', frame ? frame[2] : 0);
});

function syncTelemetryFocus(forced) {
  const focused = typeof forced === 'boolean'
    ? forced
    : document.visibilityState === 'visible' && document.hasFocus();
  if (focused === state.telemetryFocused) return;
  state.telemetryFocused = focused;
  invoke('set_telemetry_focus', { focused }).catch(() => {});
}

window.addEventListener('focus', () => syncTelemetryFocus(true));
window.addEventListener('blur', () => syncTelemetryFocus(false));
document.addEventListener('visibilitychange', () => syncTelemetryFocus());
window.addEventListener('load', () => syncTelemetryFocus());

function telemetryNonce() {
  const b = new Uint8Array(8);
  (crypto.getRandomValues ? crypto : window.crypto).getRandomValues(b);
  return Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
}

function classifyStartFailure(err) {
  const t = String(err || '');
  if (t.includes('Invalid App ID') || t.includes('Invalid depot ID')) {
    return { fail_stage: 'source_probe', fail_class: 'decode' };
  }
  if (t.includes('Cannot create download directory')) {
    return { fail_stage: 'disk', fail_class: 'io' };
  }
  return { fail_stage: 'unknown', fail_class: 'unknown' };
}

function jobContext() {
  return Object.assign({
    job: state.dlNonce || null,
    source_count: state.dlSourceCount ?? null,
    had_mh_key: !!state.dlHadMhKey,
  }, state.dlContext || {});
}

function clearJobTelemetryState() {
  state.dlNonce = null;
  state.dlStartedAt = null;
  state.dlStage = null;
  state.dlDepotCount = null;
  state.dlSourceCount = null;
  state.dlHadMhKey = false;
  state.dlContext = null;
}

function durationBucket(ms) {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 5) return '<5s';
  if (s < 30) return '5-30s';
  if (s < 120) return '30s-2m';
  if (s < 600) return '2-10m';
  if (s < 3600) return '10-60m';
  return '>60m';
}

function countBucket(n) {
  const v = Number(n) || 0;
  if (v <= 0) return '0';
  if (v === 1) return '1';
  if (v <= 3) return '2-3';
  if (v <= 10) return '4-10';
  return '>10';
}

const EMU_FAIL_CLASSES = new Set([
  'emu_binary_missing',
  'interfaces_failed',
  'backup_failed',
  'copy_failed',
  'settings_write_failed',
  'no_parent_dir',
  'folder_missing',
  'no_backup',
  'restore_failed',
]);

function emuEntryPoint() {
  if (state.emuStandalone) return 'standalone';
  if (state.emuEditMode) return 'history';
  return 'download';
}

function emuPlatformMix(targets) {
  const list = targets || [];
  const win = list.some(t => t.platform === 'windows');
  const linux = list.some(t => t.platform === 'linux');
  if (win && linux) return 'mixed';
  if (linux) return 'linux';
  if (win) return 'windows';
  return 'none';
}

function emuFailClass(results) {
  const first = (results || []).find(r => !r.success);
  if (!first) return null;
  return EMU_FAIL_CLASSES.has(first.failClass) ? first.failClass : 'unknown';
}

function emuErrorHint(err) {
  const t = String(err || '');
  if (/os error 225|virus|unwanted software|unerwünschte software/i.test(t)) {
    return window.i18n.t('emulator.hintAntivirusFile');
  }
  if (/os error 5\b|os error 13\b|access is denied|zugriff verweigert|permission denied/i.test(t)) {
    return window.i18n.t('emulator.hintPermission');
  }
  if (/GitHub fetch failed|GitHub returned HTTP|download returned HTTP|error sending request|steamless download|not present in release/i.test(t)) {
    return window.i18n.t('emulator.hintDownload');
  }
  return '';
}

function withEmuHint(text, ...sources) {
  const hint = sources.map(emuErrorHint).find(Boolean);
  return hint ? `${text}\n\n${hint}` : text;
}

function classifyEmuCommandError(err) {
  const t = String(err || '');
  if (t.includes('AV_BLOCKED')) return 'av_blocked';
  if (t.includes('GitHub')) return 'release_fetch_failed';
  if (t.includes('download returned HTTP')) return 'emu_download_failed';
  return 'unknown';
}

function emuOutcome(success, total) {
  if (total <= 0) return 'failed';
  if (success === total) return 'complete';
  return success > 0 ? 'partial' : 'failed';
}

function abandonProps() {
  return Object.assign(jobContext(), {
    outcome: 'abandoned',
    depots_total: state.dlDepotCount ?? 0,
    last_stage: state.dlStage || 'unknown',
    duration_bucket: durationBucket(Date.now() - (state.dlStartedAt || Date.now())),
    engine: state.currentEngine || 'native',
  });
}
