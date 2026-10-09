async function copyBuildInfo() {
  const info = state.buildInfo;
  if (!info) return;
  const text = [
    `Channel: ${info.channel}`,
    `Version: ${info.version}`,
    `Commit: ${info.gitSha}`,
    `Build date: ${info.buildDate}`,
    `Profile: ${info.profile}`,
    `Platform: ${info.targetOs}/${info.targetArch}`,
    `Package: ${info.package || 'unknown'}`,
    `Engine: ${info.engine || 'unknown'}`,
    `Language: ${window.i18n.getCurrentLocale()}`,
    `Diagnostic ID: ${info.diagnosticId || window.i18n.t('settings.diagnosticIdOff')}`,
  ].join('\n');
  if (info.diagnosticId) emitEvent('diagnostics_copied');

  try {
    await navigator.clipboard.writeText(text);
    const btn = els.btnCopyBuildInfo;
    const original = btn.textContent;
    btn.textContent = window.i18n.t('common.copied');
    setTimeout(() => { btn.textContent = original; }, 1500);
  } catch (e) {
    console.error('Clipboard write failed:', e);
  }
}

const ISSUE_URL = 'https://github.com/MCbabel/Steam-Manifest-Downloader/issues/new';
const ISSUE_URL_LIMIT = 7000;

async function diagnosticLines() {
  let info = state.buildInfo;
  if (!info) {
    try { info = await invoke('get_build_info'); state.buildInfo = info; } catch (_) { info = {}; }
  }
  return [
    `App version: ${info.version || 'unknown'} (${info.channel || 'unknown'}, ${info.gitSha || 'unknown'})`,
    `Platform: ${info.targetOs || '?'}/${info.targetArch || '?'}, ${info.package || 'unknown'}`,
    `Engine: ${info.engine || 'unknown'}`,
    `Language: ${window.i18n.getCurrentLocale()}`,
    `Diagnostic ID: ${info.diagnosticId || 'statistics off'}`,
  ];
}

function recentLogLines(max) {
  const lines = Array.from(els.terminalOutput ? els.terminalOutput.children : []).map(n => n.textContent);
  return lines.slice(-max);
}

async function openBugReport(prefill) {
  const modal = els.bugReportModal;
  if (!modal) return;
  const fields = { title: '#bug-title', what: '#bug-what', steps: '#bug-steps', expected: '#bug-expected' };
  for (const [key, sel] of Object.entries(fields)) {
    const el = modal.querySelector(sel);
    el.value = (prefill && prefill[key]) || '';
    el.classList.remove('is-invalid');
  }
  const hasLog = recentLogLines(1).length > 0;
  const logToggle = modal.querySelector('#bug-include-log');
  logToggle.checked = !!(prefill && prefill.log && hasLog);
  logToggle.disabled = !hasLog;
  logToggle.closest('.settings-row').classList.toggle('is-disabled', !hasLog);
  modal.querySelector('#bug-include-diag').checked = true;
  modal.querySelector('#bug-report-error').classList.add('hidden');
  modal.querySelector('#bug-diag-preview').textContent = (await diagnosticLines()).join(' · ');
  state.bugReportSource = (prefill && prefill.source) || 'manual';
  modal.classList.remove('hidden');
  modal.querySelector('#bug-title').focus();
}

function closeBugReport() {
  if (els.bugReportModal) els.bugReportModal.classList.add('hidden');
}

function issueFields(title, parts) {
  const info = state.buildInfo || {};
  const os = { windows: 'Windows', linux: 'Linux', macos: 'macOS' }[info.targetOs] || '';
  const fields = {
    template: 'bug_report.yml',
    title: `[Bug] ${title}`,
    what: parts.what.trim(),
    steps: parts.steps.trim(),
    expected: parts.expected.trim(),
    version: info.version || '',
    os,
    environment: parts.diag ? parts.diag.join('\n') : '',
    logs: parts.log && parts.log.length ? parts.log.join('\n') : '',
    extra: 'Sent from the app\'s bug report form.',
  };
  return Object.fromEntries(Object.entries(fields).filter(([, v]) => v));
}

function issueUrl(title, parts) {
  return `${ISSUE_URL}?${new URLSearchParams(issueFields(title, parts)).toString()}`;
}

async function submitBugReport() {
  const modal = els.bugReportModal;
  const get = (sel) => modal.querySelector(sel).value;
  const titleEl = modal.querySelector('#bug-title');
  const whatEl = modal.querySelector('#bug-what');
  const errorEl = modal.querySelector('#bug-report-error');
  titleEl.classList.toggle('is-invalid', !titleEl.value.trim());
  whatEl.classList.toggle('is-invalid', !whatEl.value.trim());
  if (!titleEl.value.trim() || !whatEl.value.trim()) {
    errorEl.textContent = window.i18n.t('bugReport.required');
    errorEl.classList.remove('hidden');
    (titleEl.value.trim() ? whatEl : titleEl).focus();
    return;
  }
  const withDiag = modal.querySelector('#bug-include-diag').checked;
  const withLog = modal.querySelector('#bug-include-log').checked;
  const parts = {
    what: get('#bug-what'),
    steps: get('#bug-steps'),
    expected: get('#bug-expected'),
    diag: withDiag ? await diagnosticLines() : null,
    log: withLog ? recentLogLines(60) : null,
  };
  const title = titleEl.value.trim();
  let url = issueUrl(title, parts);
  while (url.length > ISSUE_URL_LIMIT && parts.log && parts.log.length > 5) {
    parts.log = parts.log.slice(Math.ceil(parts.log.length / 3));
    url = issueUrl(title, parts);
  }
  for (const key of ['what', 'steps', 'expected']) {
    while (url.length > ISSUE_URL_LIMIT && parts[key].length > 200) {
      parts[key] = parts[key].slice(0, Math.floor(parts[key].length * 0.75)) + ' …';
      url = issueUrl(title, parts);
    }
  }
  try {
    await window.__TAURI__.shell.open(url);
    emitEvent('bug_report_opened', { source: state.bugReportSource || 'manual', diag: withDiag, log: withLog });
    closeBugReport();
    showToast(window.i18n.t('bugReport.opened'), 'success', 6000);
  } catch (e) {
    errorEl.textContent = window.i18n.localizeError(String(e));
    errorEl.classList.remove('hidden');
  }
}

function initBugReport() {
  if (!els.bugReportModal) return;
  if (els.btnReportBug) els.btnReportBug.addEventListener('click', () => openBugReport());
  const fromSettings = document.getElementById('btn-settings-report-bug');
  if (fromSettings) fromSettings.addEventListener('click', () => openBugReport({ source: 'settings' }));
  if (els.btnReportFailure) {
    els.btnReportFailure.addEventListener('click', () => openBugReport({
      source: 'download_failed',
      what: state.lastFailureText ? `${window.i18n.t('bugReport.failedPrefix')}\n\n${state.lastFailureText}` : '',
      log: true,
    }));
  }
  els.bugReportModal.querySelector('#btn-bug-cancel').addEventListener('click', closeBugReport);
  els.bugReportModal.querySelector('#btn-bug-open').addEventListener('click', submitBugReport);
  els.bugReportModal.querySelector('.modal__backdrop').addEventListener('click', closeBugReport);
  els.bugReportModal.querySelectorAll('#bug-title, #bug-what').forEach(el => {
    el.addEventListener('input', () => {
      if (el.value.trim()) el.classList.remove('is-invalid');
      const missing = els.bugReportModal.querySelectorAll('#bug-title.is-invalid, #bug-what.is-invalid').length;
      if (!missing) els.bugReportModal.querySelector('#bug-report-error').classList.add('hidden');
    });
  });
}
