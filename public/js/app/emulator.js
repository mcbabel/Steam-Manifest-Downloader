async function checkEmulatorSupport() {
  state.emuSelectionExplicit = false;
  if (!state.downloadDir) {
    state.emulatorAvailable = false;
    return;
  }
  try {
    const scanned = await invoke('emu_scan_game_dir', { gameDir: state.downloadDir });
    state.emulatorScan = Array.isArray(scanned) ? scanned : [];
    state.emuSelectedFiles = new Set();
    state.emulatorAvailable = state.emulatorScan.length > 0;
  } catch (e) {
    console.error('emu_scan_game_dir failed:', e);
    state.emulatorScan = [];
    state.emuSelectedFiles = new Set();
    state.emulatorAvailable = false;
  }
  updateNextButtonText();
}

function updateNextButtonText() {
  if (!els.btnNextStep) return;
  const hasNext = state.shortcutSupported || state.steamLibrarySupported || state.emulatorAvailable;
  els.btnNextStep.textContent = hasNext
    ? window.i18n.t('common.next')
    : window.i18n.t('emulator.goToHome');
}

async function goToEmulatorStep(opts = {}) {
  state.emuEditTargets = [];
  state.emuPatchedPaths = new Set();
  state.emuSettingsPrefillPath = null;
  state.emuBusy = false;
  state.bypassInitialState = false;
  hideEmuAlreadyPatchedNotice();
  applyEmuSettingsHint();
  if (state.emuEditMode) setEmuEditMode(false);
  state.emuApplyComplete = false;
  if (els.btnEmuApply) els.btnEmuApply.textContent = window.i18n.t('emulator.apply');
  applyEmuStartOverVisibility();
  goToStep(5);
  if (!state.emulatorScan || state.emulatorScan.length === 0) {
    await checkEmulatorSupport();
  }
  renderEmuFileList(state.emulatorScan);
  updateEmuActionButtons();
  if (opts.prefillSettings !== false) {
    populateEmuSettings(loadLastEmuSettings() || {});
  }
  applyBypassAvailability();
  scanForDlcMergeAsync();
  scanForDrmAsync();
  await loadEmuReleaseInfo();
}

async function scanForDlcMergeAsync() {
  if (els.emuDlcMergeSection) els.emuDlcMergeSection.classList.add('hidden');
  if (els.emuDlcMergeStatus) els.emuDlcMergeStatus.classList.add('hidden');
  if (state.emuStandalone) return;
  if (!state.downloadDir) return;
  const appId = currentAppIdForEmu();
  try {
    const plan = await invoke('emu_scan_for_dlc_merge', {
      gameDir: state.downloadDir,
      appId: appId || null,
    });
    if (!plan || !plan.toMerge || plan.toMerge.length === 0) return;
    state.dlcMergePlan = plan;
    if (els.emuDlcMergeSection) els.emuDlcMergeSection.classList.remove('hidden');
    if (els.emuDlcMergeHint) {
      els.emuDlcMergeHint.innerHTML = renderMergePlanHint(plan);
    }
    if (els.btnEmuMergeDlcs) els.btnEmuMergeDlcs.disabled = false;
  } catch (e) {
    console.warn('emu_scan_for_dlc_merge failed:', e);
  }
}

function roleLabel(role) {
  const key = `depots.role_${role}`;
  const translated = window.i18n.t(key);
  return translated && translated !== key ? translated : role;
}

function renderMergePlanHint(plan) {
  const mainLabel = plan.mainLabel
    ? `${plan.mainLabel} (${plan.mainDepotId})`
    : plan.mainDepotId;
  const toMergeRows = plan.toMerge
    .map(d => `<li><strong>${escapeHtml(d.depotId)}</strong> ${d.label ? '— ' + escapeHtml(d.label) : ''} <span class="depot-role-pill depot-role-pill--${escapeHtml(d.role)}">${escapeHtml(roleLabel(d.role))}</span></li>`)
    .join('');
  const skippedRows = (plan.skipped || [])
    .map(d => `<li><strong>${escapeHtml(d.depotId)}</strong> ${d.label ? '— ' + escapeHtml(d.label) : ''} <span class="depot-role-pill depot-role-pill--skipped">${escapeHtml(roleLabel(d.role))} (${escapeHtml(window.i18n.t('emulator.dlcMergeSkippedTag'))})</span></li>`)
    .join('');
  const intro = window.i18n.t('emulator.dlcMergeHintDetail', {
    count: plan.toMerge.length,
    main: escapeHtml(mainLabel),
  });
  const skippedBlock = skippedRows
    ? `<p class="dd-path__hint">${window.i18n.t('emulator.dlcMergeSkippedNote')}</p><ul class="emu-dlc-merge-list">${skippedRows}</ul>`
    : '';
  return `${intro}<ul class="emu-dlc-merge-list">${toMergeRows}</ul>${skippedBlock}`;
}

async function performDlcMerge() {
  if (!state.dlcMergePlan) return;
  const plan = state.dlcMergePlan;
  if (els.btnEmuMergeDlcs) {
    els.btnEmuMergeDlcs.disabled = true;
    els.btnEmuMergeDlcs.textContent = window.i18n.t('emulator.dlcMergeBusy');
  }
  if (els.emuDlcMergeStatus) {
    els.emuDlcMergeStatus.classList.remove('hidden');
    els.emuDlcMergeStatus.textContent = window.i18n.t('emulator.dlcMergeBusy');
  }
  try {
    await invoke('emu_merge_dlc_depots', {
      mainDepotDir: plan.mainDepotDir,
      dlcDepotDirs: plan.dlcDepotDirs,
    });
    emitEvent('dlc_merged', { ok: true, depots: countBucket(plan.dlcDepotDirs.length) });
    state.dlcMergePlan = null;
    if (els.emuDlcMergeStatus) {
      els.emuDlcMergeStatus.textContent = window.i18n.t('emulator.dlcMergeDone', {
        count: plan.dlcDepotDirs.length,
      });
    }
    if (els.btnEmuMergeDlcs) {
      els.btnEmuMergeDlcs.textContent = window.i18n.t('emulator.dlcMergeDoneShort');
    }
    setTimeout(() => {
      if (els.emuDlcMergeSection) els.emuDlcMergeSection.classList.add('hidden');
    }, 4000);
  } catch (e) {
    console.error('emu_merge_dlc_depots failed:', e);
    if (els.emuDlcMergeStatus) {
      els.emuDlcMergeStatus.textContent = window.i18n.t('emulator.dlcMergeError', {
        message: window.i18n.localizeError(e),
      });
    }
    if (els.btnEmuMergeDlcs) {
      els.btnEmuMergeDlcs.disabled = false;
      els.btnEmuMergeDlcs.textContent = window.i18n.t('emulator.dlcMergeButton');
    }
  }
}

function applyBypassAvailability() {
  const hasWindowsTarget = (state.emulatorScan || []).some(t => t.platform === 'windows');
  const toggle = els.emuBypassToggle;
  const section = els.emuBypassSection;
  if (!section) return;
  section.classList.toggle('emu-bypass-section--disabled', !hasWindowsTarget);
  if (!toggle) return;
  if (!hasWindowsTarget) {
    toggle.checked = false;
    toggle.disabled = true;
  } else {
    toggle.disabled = false;
  }
  const hintEl = section.querySelector('.emu-bypass-row__hint');
  if (hintEl) {
    hintEl.innerHTML = hasWindowsTarget
      ? window.i18n.t('emulator.bypassHint')
      : window.i18n.t('emulator.bypassLinuxNote');
  }
}

async function scanForDrmAsync() {
  if (!state.downloadDir) return;
  try {
    const entries = await invoke('steamless_scan', { gameDir: state.downloadDir });
    state.drmTargets = Array.isArray(entries) ? entries : [];
    renderDrmSection();
  } catch (e) {
    console.warn('steamless_scan failed:', e);
    state.drmTargets = [];
    if (els.emuDrmSection) els.emuDrmSection.classList.add('hidden');
  }
}

function renderDrmSection() {
  if (!els.emuDrmSection) return;
  if (!state.drmTargets || state.drmTargets.length === 0) {
    els.emuDrmSection.classList.add('hidden');
    return;
  }
  els.emuDrmSection.classList.remove('hidden');
  if (els.emuDrmList) {
    els.emuDrmList.innerHTML = state.drmTargets.map(t => {
      const rel = relativizeEmuPath(t.path);
      const size = formatBytes(t.size_bytes);
      return `<div class="emu-drm-item" data-path="${escapeHtml(t.path)}">
        <span class="emu-drm-item__path" title="${escapeHtml(t.path)}">${escapeHtml(rel)}</span>
        <span class="emu-drm-item__size">${escapeHtml(size || '')}</span>
      </div>`;
    }).join('');
  }
  if (els.emuDrmStatusWrap) els.emuDrmStatusWrap.classList.add('hidden');
  if (els.btnEmuDrmRemove) {
    els.btnEmuDrmRemove.disabled = false;
    els.btnEmuDrmRemove.textContent = window.i18n.t('emulator.drmRemove');
  }
}

function setDrmStatus(kind, text) {
  if (!els.emuDrmStatus) return;
  els.emuDrmStatus.classList.remove('emu-drm-status--busy', 'emu-drm-status--success', 'emu-drm-status--error');
  if (kind === 'busy') els.emuDrmStatus.classList.add('emu-drm-status--busy');
  else if (kind === 'success') els.emuDrmStatus.classList.add('emu-drm-status--success');
  else if (kind === 'error') els.emuDrmStatus.classList.add('emu-drm-status--error');
  els.emuDrmStatus.textContent = text;
  if (els.emuDrmStatusWrap) els.emuDrmStatusWrap.classList.remove('hidden');
}

const DRM_COPY_ICON = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
</svg>`;
const DRM_COPY_ICON_CHECK = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <polyline points="20 6 9 17 4 12"/>
</svg>`;

async function copyDrmLog() {
  if (!els.emuDrmStatus || !els.btnEmuDrmCopy) return;
  const text = els.emuDrmStatus.textContent || '';
  try {
    await navigator.clipboard.writeText(text);
  } catch (e) {
    console.error('clipboard write failed:', e);
    return;
  }
  els.btnEmuDrmCopy.classList.add('emu-drm-copy-btn--copied');
  els.btnEmuDrmCopy.innerHTML = DRM_COPY_ICON_CHECK;
  els.btnEmuDrmCopy.setAttribute('title', window.i18n.t('emulator.drmCopyLogCopied'));
  setTimeout(() => {
    els.btnEmuDrmCopy.classList.remove('emu-drm-copy-btn--copied');
    els.btnEmuDrmCopy.innerHTML = DRM_COPY_ICON;
    els.btnEmuDrmCopy.setAttribute('title', window.i18n.t('emulator.drmCopyLog'));
  }, 1500);
}

async function removeDrm() {
  if (!state.drmTargets || state.drmTargets.length === 0) return;
  const paths = state.drmTargets.map(t => t.path);
  if (els.btnEmuDrmRemove) els.btnEmuDrmRemove.disabled = true;
  setDrmStatus('busy', window.i18n.t('emulator.drmRemoving'));

  try {
    const results = await invoke('steamless_unpack', { targets: paths });
    const success = results.filter(r => r.success).length;
    const failed = results.length - success;
    emitEvent('steamless_used', { outcome: failed === 0 ? 'complete' : success ? 'partial' : 'failed', targets: countBucket(results.length) });
    if (failed === 0) {
      setDrmStatus('success', window.i18n.t('emulator.drmRemoveSuccess', { count: success }));
      results.forEach((r, i) => {
        const item = els.emuDrmList && els.emuDrmList.children[i];
        if (item && r.success) item.classList.add('emu-drm-item--success');
      });
      state.drmTargets = [];
    } else {
      const first = results.find(r => !r.success);
      const errMsg = first && first.error ? first.error : 'unknown error';
      const monoNeeded = /command not found|No such file|cannot run|exec format/i.test(errMsg)
        && /mono/i.test(errMsg);
      const hint = monoNeeded ? '\n\n' + window.i18n.t('emulator.drmMonoHint') : '';
      const summary = window.i18n.t('emulator.drmRemovePartial', { success, failed });
      setDrmStatus('error', withEmuHint(`${summary}\n\n${errMsg}${hint}`, errMsg));
    }
  } catch (e) {
    console.error('steamless_unpack failed:', e);
    const errMsg = String(e);
    const monoNeeded = /command not found|No such file|cannot run|exec format/i.test(errMsg)
      && /mono/i.test(errMsg);
    const hint = monoNeeded ? '\n\n' + window.i18n.t('emulator.drmMonoHint') : '';
    setDrmStatus('error', withEmuHint(window.i18n.t('emulator.drmRemoveError', { message: window.i18n.localizeError(errMsg) }) + hint, errMsg));
  } finally {
    if (els.btnEmuDrmRemove) els.btnEmuDrmRemove.disabled = false;
  }
}

function extractDepotFolderFromPath(p) {
  if (!p) return '';
  const norm = p.replace(/\\/g, '/');
  const marker = '/depots/';
  const idx = norm.indexOf(marker);
  if (idx < 0) return '';
  const rest = norm.slice(idx + marker.length);
  const slash = rest.indexOf('/');
  return slash < 0 ? rest : rest.slice(0, slash);
}

function pickDefaultEmuDepot(files) {
  const hostIsLinux = (navigator.userAgent || '').toLowerCase().includes('linux');
  const groups = new Map();
  for (const f of files) {
    const key = extractDepotFolderFromPath(f.path) || '__root__';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(f);
  }
  let best = null;
  let bestScore = -Infinity;
  for (const [key, group] of groups.entries()) {
    let score = 0;
    const hasHostPlatform = group.some(f => (f.platform === 'linux') === hostIsLinux);
    if (hasHostPlatform) score += 1000;
    if (group.some(f => f.arch === 'x64')) score += 100;
    score += group.length;
    if (score > bestScore) {
      bestScore = score;
      best = key;
    }
  }
  return best;
}

function applyEmuSettingsHint() {
  if (!els.emuSettingsHint) return;
  if ((state.emuEditTargets || []).length > 0) {
    els.emuSettingsHint.textContent = window.i18n.t('emulator.settingsHintPatched');
    return;
  }
  els.emuSettingsHint.textContent = state.emuStandalone
    ? window.i18n.t('emulator.settingsHintStandalone')
    : window.i18n.t('emulator.settingsHint');
}

function updateEmuPatchedNotice() {
  if (!els.emuAlreadyPatched) return;
  const patched = (state.emuEditTargets || []).length;
  const total = (state.emulatorScan || []).length;
  if (state.emuEditMode || patched === 0) {
    hideEmuAlreadyPatchedNotice();
    return;
  }
  els.emuAlreadyPatched.textContent = patched === total
    ? window.i18n.t('emulator.patchedNoticeAll')
    : window.i18n.t('emulator.patchedNoticeSome', { patched, total });
  els.emuAlreadyPatched.classList.remove('hidden');
}

function hideEmuAlreadyPatchedNotice() {
  if (els.emuAlreadyPatched) els.emuAlreadyPatched.classList.add('hidden');
}

function renderEmuFileList(files) {
  if (!els.emuFileList) return;
  if (!files || files.length === 0) {
    els.emuFileList.innerHTML = '';
    if (els.emuFileEmpty) els.emuFileEmpty.classList.remove('hidden');
    state.emuSelectedFiles = new Set();
    updateEmuActionButtons();
    return;
  }
  if (els.emuFileEmpty) els.emuFileEmpty.classList.add('hidden');

  const multipleDepots = new Set(files.map(f => extractDepotFolderFromPath(f.path)).filter(Boolean)).size > 1;
  const defaultDepot = multipleDepots ? pickDefaultEmuDepot(files) : null;

  if (!state.emuSelectionExplicit && (!state.emuSelectedFiles || state.emuSelectedFiles.size === 0)) {
    state.emuSelectedFiles = new Set(
      multipleDepots
        ? files.filter(f => extractDepotFolderFromPath(f.path) === defaultDepot).map(f => f.path)
        : files.map(f => f.path)
    );
  }

  const rows = files.map((f, idx) => {
    const relPath = relativizeEmuPath(f.path);
    const isLinux = f.platform === 'linux';
    const platformLabel = isLinux
      ? window.i18n.t('emulator.platformLinux')
      : window.i18n.t('emulator.platformWindows');
    const platformClass = isLinux ? 'emu-file-tag--linux' : 'emu-file-tag--windows';
    const archLabel = f.arch === 'x64'
      ? window.i18n.t('emulator.archX64')
      : window.i18n.t('emulator.archX32');
    const checked = state.emuSelectedFiles.has(f.path) ? 'checked' : '';
    return `
      <label class="emu-file-item">
        <input type="checkbox" class="emu-file-item__check" data-path="${escapeHtml(f.path)}" data-idx="${idx}" ${checked}>
        <span class="emu-file-item__path" title="${escapeHtml(f.path)}">${escapeHtml(relPath)}</span>
        <span class="emu-file-item__tags">
          <span class="emu-file-tag ${platformClass}">${escapeHtml(platformLabel)}</span>
          <span class="emu-file-tag">${escapeHtml(archLabel)}</span>
          ${state.emuPatchedPaths.has(f.path)
            ? `<span class="emu-file-tag emu-file-tag--patched">${escapeHtml(window.i18n.t('emulator.patchedBadge'))}</span>`
            : ''}
        </span>
      </label>`;
  }).join('');
  els.emuFileList.innerHTML = rows;

  els.emuFileList.querySelectorAll('.emu-file-item__check').forEach(cb => {
    cb.addEventListener('change', (e) => {
      const p = cb.dataset.path;
      state.emuSelectionExplicit = true;
      if (cb.checked) state.emuSelectedFiles.add(p);
      else state.emuSelectedFiles.delete(p);
      updateEmuActionButtons();
    });
  });

  updateEmuActionButtons();
}

function setEmuBusy(on) {
  state.emuBusy = !!on;
  updateEmuActionButtons();
}

function updateEmuActionButtons() {
  const selected = getSelectedEmuTargets();
  const patchedCount = (state.emuEditTargets || []).length;
  const busy = !!state.emuBusy;

  if (els.btnEmuApply) {
    els.btnEmuApply.classList.toggle('hidden', !!state.emuEditMode);
    els.btnEmuApply.disabled = busy || selected.length === 0;
    if (!state.emuApplyComplete) {
      const allSelectedPatched = selected.length > 0
        && selected.every(t => state.emuPatchedPaths.has(t.path));
      els.btnEmuApply.textContent = allSelectedPatched
        ? window.i18n.t('emulator.reapply')
        : window.i18n.t('emulator.apply');
    }
  }

  if (els.btnEmuSaveSettings) {
    els.btnEmuSaveSettings.classList.toggle('hidden', patchedCount === 0);
    els.btnEmuSaveSettings.disabled = busy || !state.emuSettingsPrefillPath;
    const primary = state.emuEditMode || selected.length === 0;
    els.btnEmuSaveSettings.classList.toggle('btn--primary', primary);
    els.btnEmuSaveSettings.classList.toggle('btn--outline', !primary);
  }

  if (els.btnEmuRevert) {
    els.btnEmuRevert.classList.toggle('hidden', patchedCount === 0);
    els.btnEmuRevert.disabled = busy;
  }
  updateEmuOverview();
}

function selectEmuTab(section) {
  const step = document.getElementById('step-emulator');
  if (!step) return;
  step.querySelectorAll('.emu-nav__tab').forEach(tab => {
    const on = tab.dataset.section === section;
    tab.classList.toggle('is-active', on);
    tab.setAttribute('aria-selected', on ? 'true' : 'false');
    tab.tabIndex = on ? 0 : -1;
  });
  step.querySelectorAll('.emu-panel').forEach(panel => {
    panel.classList.toggle('is-active', panel.dataset.section === section);
  });
}

function setEmuBadge(section, text, tone) {
  const badge = document.querySelector(`#step-emulator .emu-nav__badge[data-badge="${section}"]`);
  if (!badge) return;
  badge.textContent = text || '';
  badge.classList.toggle('hidden', !text);
  badge.classList.toggle('emu-nav__badge--on', tone === 'on');
  badge.classList.toggle('emu-nav__badge--warn', tone === 'warn');
}

function updateEmuOverview() {
  const summary = document.getElementById('emu-summary');
  if (!summary) return;
  const t = (key, params) => window.i18n.t(key, params);
  const total = (state.emulatorScan || []).length;
  const selected = getSelectedEmuTargets().length;
  const drmShown = !!(els.emuDrmSection && !els.emuDrmSection.classList.contains('hidden'));
  const mergeShown = !!(els.emuDlcMergeSection && !els.emuDlcMergeSection.classList.contains('hidden'));
  const gameData = document.getElementById('emu-gamedata-toggle');
  const bypass = document.getElementById('emu-bypass-toggle');
  const gameDataOn = !!(gameData && gameData.checked);
  const bypassOn = !!(bypass && bypass.checked);
  const variant = selectedEmuVariant();
  const custom = Object.keys(gatherEmuSettings() || {}).length;

  setEmuBadge('files', drmShown ? '!' : (total ? `${selected}/${total}` : ''), drmShown ? 'warn' : null);
  setEmuBadge('gamedata', t(gameDataOn ? 'emulator.summary.on' : 'emulator.summary.off'), gameDataOn ? 'on' : null);
  setEmuBadge('emulator', custom ? String(custom) : '', null);
  setEmuBadge('extras', mergeShown ? '!' : (bypassOn ? t('emulator.summary.on') : ''), mergeShown ? 'warn' : (bypassOn ? 'on' : null));

  const chips = [];
  const chip = (text, tone) => chips.push(`<span class="emu-summary__chip${tone ? ` emu-summary__chip--${tone}` : ''}">${text}</span>`);
  if (state.gameName) chip(`<strong>${escapeHtml(state.gameName)}</strong>`);
  if (total) chip(escapeHtml(t('emulator.summary.files', { selected, total })));
  chip(escapeHtml(t(variant === 'experimental' ? 'emulator.variantExperimental' : 'emulator.variantRegular')));
  chip(escapeHtml(t('emulator.tabs.gameData')), gameDataOn ? 'on' : 'off');
  if (bypassOn) chip(escapeHtml(t('emulator.bypassLabel')), 'on');
  if (custom) chip(escapeHtml(t('emulator.summary.custom', { count: custom })));
  if (drmShown) chip(escapeHtml(t('emulator.drmTitle')), 'warn');
  if (mergeShown) chip(escapeHtml(t('emulator.dlcMergeLabel')), 'warn');
  summary.innerHTML = chips.join('');
}

const reportedEmuSections = new Set();

function reportEmuSection(section) {
  if (!section || reportedEmuSections.has(section)) return;
  reportedEmuSections.add(section);
  emitEvent('emu_section_viewed', { section });
}

function initEmuNav() {
  const step = document.getElementById('step-emulator');
  if (!step) return;
  const tabs = () => Array.from(step.querySelectorAll('.emu-nav__tab'));
  tabs().forEach(tab => {
    tab.addEventListener('click', () => {
      selectEmuTab(tab.dataset.section);
      reportEmuSection(tab.dataset.section);
    });
    tab.addEventListener('keydown', (e) => {
      const list = tabs();
      const idx = list.indexOf(tab);
      const keys = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 };
      let next = null;
      if (e.key in keys) next = list[(idx + keys[e.key] + list.length) % list.length];
      else if (e.key === 'Home') next = list[0];
      else if (e.key === 'End') next = list[list.length - 1];
      if (!next) return;
      e.preventDefault();
      selectEmuTab(next.dataset.section);
      reportEmuSection(next.dataset.section);
      next.focus();
    });
  });
  step.addEventListener('change', () => updateEmuOverview());
  step.addEventListener('input', (e) => {
    if (e.target && e.target.matches('[data-emu-key]')) updateEmuOverview();
  });
  const observer = new MutationObserver(() => updateEmuOverview());
  [els.emuDrmSection, els.emuDlcMergeSection].forEach(el => {
    if (el) observer.observe(el, { attributes: true, attributeFilter: ['class'] });
  });
}

function refreshEmuPatchedState(scanned) {
  state.emulatorScan = Array.isArray(scanned) ? scanned : [];
  state.emuEditTargets = state.emulatorScan.filter(f => f.is_patched);
  state.emuPatchedPaths = new Set(state.emuEditTargets.map(f => f.path));
}

async function hydrateEmuPatchedContext() {
  if (!state.emuEditTargets || state.emuEditTargets.length === 0) {
    state.emuSettingsPrefillPath = null;
    state.bypassInitialState = false;
    if (els.emuBypassToggle && !els.emuBypassToggle.disabled) els.emuBypassToggle.checked = false;
  } else {
    try {
      const initial = await invoke('emu_read_emu_settings', { targetPath: state.emuEditTargets[0].path });
      populateEmuSettings(initial || {});
      state.emuSettingsPrefillPath = state.emuEditTargets[0].path;
    } catch (e) {
      console.error('emu_read_emu_settings failed:', e);
      state.emuSettingsPrefillPath = null;
      setEmuApplyStatus('error', window.i18n.t('emulator.settingsReadError', { message: window.i18n.localizeError(e) }));
    }
    let installed = false;
    try {
      installed = await invoke('steam_api_bypass_status', {
        targets: state.emuEditTargets.map(t => t.path),
      });
    } catch (e) {
      console.error('steam_api_bypass_status failed:', e);
    }
    state.bypassInitialState = !!installed;
    if (els.emuBypassToggle && !els.emuBypassToggle.disabled) els.emuBypassToggle.checked = !!installed;
  }
  applyEmuSettingsHint();
  updateEmuPatchedNotice();
  renderEmuFileList(state.emulatorScan);
}

async function refreshEmuScanInPlace() {
  if (!state.downloadDir) return;
  try {
    const scanned = await invoke('emu_scan_game_dir', { gameDir: state.downloadDir });
    refreshEmuPatchedState(scanned);
    state.emuSettingsPrefillPath = state.emuEditTargets.length > 0
      ? state.emuEditTargets[0].path
      : null;
  } catch (e) {
    console.error('emu_scan_game_dir after action failed:', e);
    return;
  }
  applyEmuSettingsHint();
  updateEmuPatchedNotice();
  renderEmuFileList(state.emulatorScan);
}

function getSelectedEmuTargets() {
  if (!state.emulatorScan) return [];
  if (!state.emuSelectionExplicit && (!state.emuSelectedFiles || state.emuSelectedFiles.size === 0)) {
    return state.emulatorScan.slice();
  }
  return state.emulatorScan.filter(f => state.emuSelectedFiles.has(f.path));
}

function relativizeEmuPath(absPath) {
  if (!absPath) return '';
  if (state.downloadDir && absPath.startsWith(state.downloadDir)) {
    const rest = absPath.slice(state.downloadDir.length);
    return rest.replace(/^[\\/]+/, '');
  }
  return absPath;
}

async function loadEmuReleaseInfo() {
  if (!els.emuReleaseStatus) return;
  els.emuReleaseStatus.classList.remove('emu-release-status--ready');
  els.emuReleaseStatus.classList.add('emu-release-status--busy');
  els.emuReleaseStatus.textContent = window.i18n.t('emulator.releaseLoading');
  try {
    const info = await invoke('emu_release_info');
    state.emulatorReleaseInfo = info;
    els.emuReleaseStatus.classList.remove('emu-release-status--busy');
    els.emuReleaseStatus.classList.add('emu-release-status--ready');
    els.emuReleaseStatus.textContent = window.i18n.t('emulator.releaseReady', { tag: info.tag });
  } catch (e) {
    console.error('emu_release_info failed:', e);
    els.emuReleaseStatus.classList.remove('emu-release-status--busy', 'emu-release-status--ready');
    els.emuReleaseStatus.textContent = window.i18n.localizeError(e);
  }
}

function currentAppIdForEmu() {
  if (state.parsedData && state.parsedData.mainAppId) return String(state.parsedData.mainAppId);
  if (state.searchAppId) return String(state.searchAppId);
  return '';
}

function selectedEmuVariant() {
  const checked = document.querySelector('input[name="emu-variant"]:checked');
  return (checked && checked.value === 'experimental') ? 'experimental' : 'regular';
}

async function syncEmuBypass(targets) {
  if (!els.emuBypassToggle || els.emuBypassToggle.disabled) return '';
  const want = !!els.emuBypassToggle.checked;
  if (want === state.bypassInitialState) return '';
  let outcome;
  if (want) {
    outcome = await applySteamApiBypass(targets);
  } else {
    try {
      await invoke('steam_api_bypass_revert', { targets: (targets || []).map(t => t.path) });
      outcome = { ok: true, message: '' };
      emitEvent('api_bypass', { action: 'revert', ok: true });
    } catch (e) {
      console.error('steam_api_bypass_revert failed:', e);
      outcome = { ok: false, message: window.i18n.t('emulator.bypassError', { message: window.i18n.localizeError(e) }) };
    }
  }
  if (outcome.ok) state.bypassInitialState = want;
  return outcome.message;
}

async function applySteamApiBypass(targets) {
  const windowsTargets = (targets || []).filter(t => t.platform === 'windows');
  if (windowsTargets.length === 0) return { ok: true, message: '' };
  try {
    const results = await invoke('steam_api_bypass_apply', { targets: windowsTargets });
    const success = results.filter(r => r.success).length;
    const failed = results.length - success;
    emitEvent('api_bypass', { action: 'apply', ok: failed === 0, targets: countBucket(results.length) });
    if (failed === 0) {
      return { ok: true, message: window.i18n.t('emulator.bypassSuccess', { count: success }) };
    }
    const first = results.find(r => !r.success);
    const detail = first && first.error ? `\n${first.error}` : '';
    return {
      ok: false,
      message: window.i18n.t('emulator.bypassPartial', { success, failed }) + detail,
    };
  } catch (e) {
    console.error('steam_api_bypass_apply failed:', e);
    return { ok: false, message: window.i18n.t('emulator.bypassError', { message: window.i18n.localizeError(e) }) };
  }
}

async function syncEmuGameDataKey() {
  const input = document.getElementById('emu-webapi-key');
  if (!input) return;
  try {
    const settings = await invoke('get_settings');
    input.value = settings.steam_web_api_key || '';
    setEmuGameMedia(settings.game_data_media || 'off');
  } catch (_) {}
}

async function saveSteamWebApiKey(value) {
  const settings = await invoke('get_settings');
  const next = String(value || '').trim();
  if (settings.steam_web_api_key === next) return;
  settings.steam_web_api_key = next;
  await invoke('save_settings', { settings });
  emitEvent('settings_saved', { keys: ['steam_web_api_key'], from: 'emulator' });
}

function setEmuGameDataStatus(kind, text) {
  const el = document.getElementById('emu-gamedata-status');
  if (!el) return;
  el.classList.toggle('hidden', !text);
  el.classList.toggle('dd-path__hint--error', kind === 'error');
  el.textContent = text || '';
}

function gameDataSummary(r, targets) {
  const t = (key, params) => window.i18n.t(`emulator.gameData.${key}`, params);
  const parts = [];
  if (r.languages) parts.push(t('languages', { count: r.languages }));
  if (r.depots) parts.push(t('depots', { count: r.depots }));
  if (r.branches) parts.push(t('branches', { count: r.branches }));
  if (r.achievements) {
    parts.push(r.achievement_languages > 1
      ? t('achievementLanguages', { count: r.achievements, languages: r.achievement_languages, icons: r.icons })
      : t('achievements', { count: r.achievements, icons: r.icons }));
  }
  if (r.stats) parts.push(t('stats', { count: r.stats }));
  if (r.leaderboards) parts.push(t('leaderboards', { count: r.leaderboards }));
  if (r.items) parts.push(t('items', { count: r.items }));
  if (r.controller_sets) parts.push(t('controller', { count: r.controller_sets }));
  if (r.cloud_dirs) parts.push(t('cloud', { count: r.cloud_dirs }));
  if (r.watcher_schemas) parts.push(t('watcher', { count: r.watcher_schemas }));
  if (r.media_files) parts.push(t('mediaFiles', { count: r.media_files }));
  const lines = [parts.length ? t('written', { list: parts.join(', ') }) : t('nothing')];
  (r.notes || []).forEach(note => lines.push(window.i18n.t(`emulator.${note}`)));
  const windowsDll = (targets || []).some(x => /\.dll$/i.test(x.path || ''));
  if (r.controller_sets && windowsDll && selectedEmuVariant() !== 'experimental') lines.push(t('controllerBuild'));
  return lines.join('\n');
}

async function writeEmuGameData(targets, appId, language) {
  const paths = (targets || []).map(t => t.path).filter(Boolean);
  if (!paths.length || !appId) return '';
  setEmuGameDataStatus('busy', window.i18n.t('emulator.gameData.busy'));
  try {
    const r = await invoke('emu_generate_game_data', { targets: paths, appId: String(appId), language: language || null });
    emitEvent('game_data_written', {
      source: r.source,
      achievements: countBucket(r.achievements),
      achievement_languages: countBucket(r.achievement_languages),
      stats: countBucket(r.stats),
      languages: countBucket(r.languages),
      depots: countBucket(r.depots),
      branches: countBucket(r.branches),
      leaderboards: countBucket(r.leaderboards),
      items: countBucket(r.items),
      controller: r.controller_sets > 0,
      cloud_dirs: countBucket(r.cloud_dirs),
      watcher: r.watcher_schemas > 0,
      media: currentEmuGameMedia(),
      icons: r.achievements ? r.icons >= r.achievements : null,
      notes: (r.notes || []).length ? r.notes.map(n => n.replace('gameData.', '')) : ['none'],
    });
    const text = gameDataSummary(r, targets);
    setEmuGameDataStatus(r.achievements || r.languages ? 'ok' : 'error', text);
    return text;
  } catch (e) {
    const text = window.i18n.t('emulator.gameData.failed', { message: window.i18n.localizeError(String(e)) });
    reportError('game_data', e);
    setEmuGameDataStatus('error', text);
    return text;
  }
}

function currentEmuGameMedia() {
  const active = document.querySelector('#emu-gamedata-media .segmented__option.is-active');
  return (active && active.dataset.value) || 'off';
}

function setEmuGameMedia(value) {
  document.querySelectorAll('#emu-gamedata-media .segmented__option').forEach(btn => {
    const on = btn.dataset.value === (value || 'off');
    btn.classList.toggle('is-active', on);
    btn.setAttribute('aria-checked', on ? 'true' : 'false');
  });
}

async function saveEmuGameMedia(value) {
  const settings = await invoke('get_settings');
  if ((settings.game_data_media || 'off') === value) return;
  settings.game_data_media = value;
  await invoke('save_settings', { settings });
  emitEvent('settings_saved', { keys: ['game_data_media'], from: 'emulator' });
}

function initEmuGameData() {
  const toggle = document.getElementById('emu-gamedata-toggle');
  const section = document.getElementById('emu-gamedata-section');
  if (!toggle || !section) return;
  try {
    const saved = localStorage.getItem('emuGameData');
    if (saved === '0') toggle.checked = false;
  } catch (_) {}
  const sync = () => section.classList.toggle('is-off', !toggle.checked);
  sync();
  toggle.addEventListener('change', () => {
    sync();
    try { localStorage.setItem('emuGameData', toggle.checked ? '1' : '0'); } catch (_) {}
  });
  document.querySelectorAll('#emu-gamedata-media .segmented__option').forEach(btn => {
    btn.addEventListener('click', async () => {
      setEmuGameMedia(btn.dataset.value);
      try { await saveEmuGameMedia(btn.dataset.value); } catch (e) { showToast(window.i18n.localizeError(String(e)), 'error'); }
    });
  });
  const save = document.getElementById('btn-emu-webapi-save');
  const input = document.getElementById('emu-webapi-key');
  if (save && input) {
    save.addEventListener('click', async () => {
      try {
        await saveSteamWebApiKey(input.value);
        showToast(window.i18n.t('emulator.gameData.keySaved'), 'success');
      } catch (e) {
        showToast(window.i18n.localizeError(String(e)), 'error');
      }
    });
  }
  const run = document.getElementById('btn-emu-gamedata');
  if (run) {
    run.addEventListener('click', async () => {
      const targets = getSelectedEmuTargets();
      const appId = currentAppIdForEmu();
      if (!targets.length || !appId) {
        setEmuGameDataStatus('error', window.i18n.t('emulator.applyNoSelection'));
        return;
      }
      if (input && input.value.trim()) {
        try { await saveSteamWebApiKey(input.value); } catch (_) {}
      }
      run.disabled = true;
      const gathered = gatherEmuSettings();
      await writeEmuGameData(targets, appId, gathered && gathered.language);
      run.disabled = false;
    });
  }
}

function collectInstalledAppIds(mainAppId) {
  const list = (state.parsedData && Array.isArray(state.parsedData.allAppIds))
    ? state.parsedData.allAppIds.slice()
    : [];
  const main = mainAppId != null ? String(mainAppId) : null;
  return list.filter(id => id && id !== main);
}

async function applyEmuReplacement() {
  if (state.emuApplyComplete) {
    resetApp();
    return;
  }
  const selectedTargets = getSelectedEmuTargets();
  if (selectedTargets.length === 0) {
    setEmuApplyStatus('error', window.i18n.t('emulator.applyNoSelection'));
    return;
  }
  const appId = currentAppIdForEmu();
  if (!appId) {
    setEmuApplyStatus('error', window.i18n.t('emulator.applyError', { message: 'missing app id' }));
    return;
  }
  setEmuBusy(true);
  setEmuApplyStatus('busy', window.i18n.t('emulator.applying'));

  const variant = selectedEmuVariant();
  try {
    const gathered = gatherEmuSettings();
    const installedAppIds = collectInstalledAppIds(appId);
    const results = await invoke('emu_apply_replacement', {
      targets: selectedTargets,
      variant,
      appId,
      installedAppIds,
      emuSettings: gathered || {},
      allowDownload: !!state.emuAllowDownload,
    });
    state.emuAllowDownload = false;
    const total = results.length;
    const success = results.filter(r => r.success).length;
    const failed = total - success;
    emitEvent('patch_applied', {
      entry: emuEntryPoint(),
      outcome: emuOutcome(success, total),
      variant,
      platforms: emuPlatformMix(selectedTargets),
      targets: countBucket(total),
      failures: countBucket(failed),
      fail_class: emuFailClass(results),
    });
    if (failed === 0) {
      const bypassMessage = await syncEmuBypass(selectedTargets);
      const extra = bypassMessage ? '\n\n' + bypassMessage : '';
      const dlcResult = results.find(r => typeof r.dlcCount === 'number');
      const dlcNote = dlcResult
        ? '\n' + (dlcResult.dlcCount === 0
          ? window.i18n.t('emulator.dlcNone')
          : window.i18n.t('emulator.dlcActivated', { count: dlcResult.dlcCount }))
        : '';
      const toggle = document.getElementById('emu-gamedata-toggle');
      const keyInput = document.getElementById('emu-webapi-key');
      if (keyInput && keyInput.value.trim()) {
        try { await saveSteamWebApiKey(keyInput.value); } catch (_) {}
      }
      const gameNote = toggle && toggle.checked
        ? '\n\n' + await writeEmuGameData(selectedTargets, appId, gathered && gathered.language)
        : '';
      setEmuApplyStatus('success', window.i18n.t('emulator.applySuccess', { count: success, total }) + dlcNote + extra + gameNote);
      saveLastEmuSettings(gathered);
      if (state.emuStandalone) {
        await refreshEmuScanInPlace();
      } else {
        state.emuApplyComplete = true;
        if (els.btnEmuApply) els.btnEmuApply.textContent = window.i18n.t('emulator.goBackHome');
      }
    } else {
      const failedResults = results.filter(r => !r.success);
      const details = failedResults
        .map(r => `${emuFileLabel(r.path)}\n    ${r.error || window.i18n.t('emulator.applyReasonUnknown')}`)
        .join('\n');
      const summary = withEmuHint(
        window.i18n.t('emulator.applyPartial', { success, failed }),
        ...failedResults.map(r => r.error),
      );
      setEmuApplyStatus('error', summary, details);
      if (state.emuStandalone) {
        await refreshEmuScanInPlace();
      }
    }
  } catch (e) {
    console.error('emu_apply_replacement failed:', e);
    const confirmInfo = parseEmuDownloadConfirm(e);
    if (confirmInfo) {
      state.emuPendingDownload = { targets: selectedTargets, variant };
      showEmuDownloadConfirm(confirmInfo);
      return;
    }
    state.emuAllowDownload = false;
    emitEvent('patch_applied', {
      entry: emuEntryPoint(),
      outcome: 'failed',
      variant: selectedEmuVariant(),
      platforms: emuPlatformMix(selectedTargets),
      targets: countBucket(selectedTargets.length),
      failures: countBucket(selectedTargets.length),
      fail_class: classifyEmuCommandError(e),
    });
    const msg = String(e);
    if (msg.includes('AV_BLOCKED')) {
      setEmuApplyAntivirusBlocked();
    } else {
      setEmuApplyStatus('error', withEmuHint(window.i18n.t('emulator.applyError', { message: window.i18n.localizeError(msg) }), msg));
    }
  } finally {
    setEmuBusy(false);
    cleanupEmuDownloadListener();
  }
}

function setEmuApplyAntivirusBlocked() {
  if (!els.emuApplyStatus) return;
  els.emuApplyStatus.classList.remove('hidden', 'completion-message--success');
  els.emuApplyStatus.classList.add('completion-message--error');
  const title = window.i18n.t('emulator.avBlockedTitle');
  const hint = window.i18n.t('emulator.avBlockedHint');
  const retry = window.i18n.t('emulator.avBlockedRetry');
  els.emuApplyStatus.innerHTML = `
    <div class="av-blocked">
      <div class="av-blocked__title">${escapeHtml(title)}</div>
      <p class="av-blocked__hint">${escapeHtml(hint)}</p>
      <button type="button" id="btn-av-retry" class="btn btn--primary av-blocked__retry">${escapeHtml(retry)}</button>
    </div>
  `.trim();
  const retryBtn = document.getElementById('btn-av-retry');
  if (retryBtn) {
    retryBtn.addEventListener('click', () => {
      setEmuApplyStatus('busy', window.i18n.t('emulator.applying'));
      applyEmuReplacement();
    });
  }
}

function emuSettingsDirOf(path) {
  return String(path ?? '').replace(/[\\/][^\\/]*$/, '');
}

async function saveEmuSettings() {
  if (!state.emuEditTargets || state.emuEditTargets.length === 0) return;
  if (state.emuBusy) return;
  setEmuBusy(true);
  if (state.downloadDir) {
    try {
      await invoke('emu_scan_game_dir', { gameDir: state.downloadDir });
    } catch (e) {
      console.error('game folder gone before save:', e);
      setEmuBusy(false);
      resetApp();
      showFolderMissing();
      return;
    }
  }
  setEmuApplyStatus('busy', window.i18n.t('emulator.savingSettings'));

  const settings = gatherEmuSettings() || {};
  const seenDirs = new Set();
  let success = 0;
  let failed = 0;
  for (const target of state.emuEditTargets) {
    const dir = emuSettingsDirOf(target.path);
    if (seenDirs.has(dir)) continue;
    seenDirs.add(dir);
    try {
      await invoke('emu_write_emu_settings', { targetPath: target.path, settings });
      success++;
    } catch (e) {
      console.error('emu_write_emu_settings failed:', target.path, e);
      failed++;
    }
  }

  emitEvent('patch_settings_saved', {
    entry: emuEntryPoint(),
    outcome: emuOutcome(success, success + failed),
    platforms: emuPlatformMix(state.emuEditTargets),
    targets: countBucket(success + failed),
    failures: countBucket(failed),
  });

  if (failed > 0) {
    setEmuBusy(false);
    setEmuApplyStatus('error', window.i18n.t('emulator.savePartial', { success, failed }));
    return;
  }

  const bypassMessage = await syncEmuBypass(state.emuEditTargets);
  setEmuBusy(false);

  if (!state.emuStandalone) {
    resetApp();
    return;
  }
  const extra = bypassMessage ? '\n\n' + bypassMessage : '';
  setEmuApplyStatus('success', window.i18n.t('emulator.saveSuccess', { count: success }) + extra);
}

function parseEmuDownloadConfirm(err) {
  const text = String((err && err.message) || err || '');
  const prefix = 'EMU_DOWNLOAD_CONFIRM_REQUIRED:';
  if (!text.startsWith(prefix)) return null;
  try {
    const info = JSON.parse(text.slice(prefix.length));
    if (!info || !Array.isArray(info.platforms) || info.platforms.length === 0) return null;
    return info;
  } catch (_) {
    return null;
  }
}

function platformDisplayName(platform) {
  return platform === 'windows' ? 'Windows' : platform === 'linux' ? 'Linux' : String(platform || '');
}

function showEmuDownloadConfirm(info) {
  if (els.emuDownloadScope) {
    const items = info.platforms.map(p => {
      const size = formatBytes(p.size);
      return `${platformDisplayName(p.platform)}${size ? ` (~${size})` : ''}`;
    });
    els.emuDownloadScope.textContent = window.i18n.t('modals.emuDownload.scope', {
      tag: info.tag || '',
      items: items.join(', '),
    });
  }
  if (els.emuDownloadModal) {
    const link = els.emuDownloadModal.querySelector('a[href^="http"]');
    if (link && info.tag) {
      link.href = `https://github.com/Detanup01/gbe_fork/releases/tag/${encodeURIComponent(info.tag)}`;
    }
    els.emuDownloadModal.classList.remove('hidden');
  }
}

async function confirmEmuDownload() {
  if (els.emuDownloadModal) els.emuDownloadModal.classList.add('hidden');
  state.emuAllowDownload = true;
  state.emuPendingDownload = null;
  if (state.unlistenEmuDownload) {
    state.unlistenEmuDownload();
    state.unlistenEmuDownload = null;
  }
  state.unlistenEmuDownload = await listen('emu-download-progress', (event) => {
    showEmuDownloadProgress(event.payload);
  });
  setEmuApplyStatus('busy', window.i18n.t('emulator.emuDownloading', { progress: '0%' }));
  await applyEmuReplacement();
}

function showEmuDownloadProgress(info) {
  if (!info) return;
  const label = platformDisplayName(info.platform);
  if (info.total && info.total > 0 && info.downloaded >= info.total) {
    setEmuApplyStatus(
      'busy',
      window.i18n.t('emulator.emuExtracting', { platform: label ? ` (${label})` : '' })
    );
    return;
  }
  const done = formatBytes(info.downloaded);
  let progress = done || '';
  if (info.total && info.total > 0) {
    const pct = Math.floor((info.downloaded / info.total) * 100);
    progress = `${pct}% · ${done}/${formatBytes(info.total)}`;
  }
  setEmuApplyStatus(
    'busy',
    window.i18n.t('emulator.emuDownloading', { progress: label ? `${label} ${progress}` : progress })
  );
}

function cleanupEmuDownloadListener() {
  if (state.unlistenEmuDownload) {
    state.unlistenEmuDownload();
    state.unlistenEmuDownload = null;
  }
}

function cancelEmuDownload() {
  if (els.emuDownloadModal) els.emuDownloadModal.classList.add('hidden');
  cleanupEmuDownloadListener();
  const pending = state.emuPendingDownload;
  state.emuPendingDownload = null;
  if (!pending) return;
  emitEvent('patch_applied', {
    entry: emuEntryPoint(),
    outcome: 'cancelled',
    variant: pending.variant,
    platforms: emuPlatformMix(pending.targets),
    targets: countBucket((pending.targets || []).length),
    failures: countBucket(0),
    fail_class: null,
  });
  setEmuApplyStatus('error', window.i18n.t('emulator.applyCancelled'));
}

function showEmuRevertConfirm() {
  if (els.emuRevertScope) {
    els.emuRevertScope.textContent = window.i18n.t('modals.emuRevert.scope', {
      count: (state.emuEditTargets || []).length,
    });
  }
  if (els.emuRevertModal) els.emuRevertModal.classList.remove('hidden');
}

async function confirmEmuRevert() {
  if (els.emuRevertModal) els.emuRevertModal.classList.add('hidden');
  if (!state.emuEditTargets || state.emuEditTargets.length === 0) return;
  if (state.emuBusy) return;
  setEmuBusy(true);
  if (state.downloadDir) {
    try {
      await invoke('emu_scan_game_dir', { gameDir: state.downloadDir });
    } catch (e) {
      console.error('game folder gone before revert:', e);
      setEmuBusy(false);
      resetApp();
      showFolderMissing();
      return;
    }
  }
  setEmuApplyStatus('busy', window.i18n.t('emulator.reverting'));

  const revertTargets = state.emuEditTargets.slice();
  const revertEntry = emuEntryPoint();
  let success = 0;
  let failed = 0;
  let details = '';
  try {
    const paths = revertTargets.map(t => t.path);
    const results = await invoke('emu_revert_replacement', { targets: paths });
    try { await invoke('steam_api_bypass_revert', { targets: paths }); }
    catch (e) { console.error('bypass revert during emu revert:', e); }
    success = results.filter(r => r.success).length;
    failed = results.length - success;
    details = results
      .filter(r => !r.success)
      .map(r => `${emuFileLabel(r.path)}\n    ${r.error || window.i18n.t('emulator.applyReasonUnknown')}`)
      .join('\n');
    emitEvent('patch_reverted', {
      entry: revertEntry,
      outcome: emuOutcome(success, results.length),
      platforms: emuPlatformMix(revertTargets),
      targets: countBucket(results.length),
      failures: countBucket(failed),
      fail_class: emuFailClass(results),
    });
  } catch (e) {
    console.error('emu_revert_replacement failed:', e);
    emitEvent('patch_reverted', {
      entry: revertEntry,
      outcome: 'failed',
      platforms: emuPlatformMix(revertTargets),
      targets: countBucket(revertTargets.length),
      failures: countBucket(revertTargets.length),
      fail_class: classifyEmuCommandError(e),
    });
    setEmuBusy(false);
    setEmuApplyStatus('error', window.i18n.t('emulator.revertError', { message: window.i18n.localizeError(e) }));
    return;
  }

  if (!state.emuStandalone) {
    setEmuBusy(false);
    if (failed > 0) {
      setEmuApplyStatus('error', window.i18n.t('emulator.revertPartial', { success, failed }), details);
      return;
    }
    resetApp();
    return;
  }

  state.bypassInitialState = false;
  if (els.emuBypassToggle && !els.emuBypassToggle.disabled) els.emuBypassToggle.checked = false;
  await refreshEmuScanInPlace();
  state.emuSelectedFiles = new Set(
    (state.emulatorScan || []).filter(f => !f.is_patched).map(f => f.path)
  );
  state.emuSettingsPrefillPath = state.emuEditTargets.length > 0
    ? state.emuEditTargets[0].path
    : null;
  renderEmuFileList(state.emulatorScan);
  setEmuBusy(false);
  if (failed > 0) {
    setEmuApplyStatus('error', window.i18n.t('emulator.revertPartial', { success, failed }), details);
    return;
  }
  setEmuApplyStatus('success', window.i18n.t('emulator.revertSuccess', { count: success }));
}

async function showFolderMissing() {
  if (els.historyModal && els.historyModal.classList.contains('hidden')) {
    await openHistory();
  }
  showHistoryBanner(window.i18n.t('modals.folderMissing.body'));
}

function showHistoryBanner(text, durationMs = 6000, kind = 'error') {
  const banner = document.getElementById('history-banner');
  if (!banner) return;
  banner.textContent = text;
  banner.classList.remove('hidden', 'history-banner--success');
  if (kind === 'success') banner.classList.add('history-banner--success');
  clearTimeout(banner._hideTimer);
  banner._hideTimer = setTimeout(() => banner.classList.add('hidden'), durationMs);
}

function hideHistoryBanner() {
  const banner = document.getElementById('history-banner');
  if (!banner) return;
  banner.classList.add('hidden');
  clearTimeout(banner._hideTimer);
}

function applyEmuStartOverVisibility() {
  if (!els.btnEmuStartOver) return;
  els.btnEmuStartOver.classList.toggle('hidden', state.emuEditMode || state.emuStandalone);
}

function setEmuEditMode(on) {
  state.emuEditMode = !!on;
  document.body.classList.toggle('emu-edit-mode', state.emuEditMode);
  if (els.emuReleaseStatus) els.emuReleaseStatus.classList.toggle('hidden', state.emuEditMode);
  if (els.emuDrmSection && state.emuEditMode) els.emuDrmSection.classList.add('hidden');
  applyEmuStartOverVisibility();
  if (els.emuHeader) {
    els.emuHeader.textContent = state.emuEditMode
      ? window.i18n.t('emulator.editTitle')
      : window.i18n.t('emulator.title');
  }
  if (els.emuDescription) {
    els.emuDescription.textContent = state.emuEditMode
      ? window.i18n.t('emulator.editDescription')
      : window.i18n.t('emulator.description');
  }
  if (els.btnEmuNew) {
    els.btnEmuNew.textContent = state.emuEditMode
      ? window.i18n.t('emulator.backToHome')
      : window.i18n.t('emulator.goToHome');
  }
  updateEmuActionButtons();
}

async function openEmuEditFromHistory(entry) {
  if (!entry || !entry.download_dir) return;
  state.emuStandalone = false;
  state.downloadDir = entry.download_dir;
  state.gameName = entry.game_name || null;
  state.headerImage = entry.header_image || null;
  if (!state.parsedData) {
    state.parsedData = { mainAppId: entry.app_id, depots: [] };
  } else {
    state.parsedData.mainAppId = entry.app_id;
  }

  let scanned = [];
  try {
    scanned = await invoke('emu_scan_game_dir', { gameDir: entry.download_dir });
  } catch (e) {
    console.error('emu_scan_game_dir failed:', e);
    showFolderMissing();
    return;
  }
  const patched = (scanned || []).filter(f => f.is_patched);
  if (patched.length === 0) {
    showHistoryBanner(window.i18n.t('emulator.editNoPatches'));
    return;
  }

  state.emulatorAvailable = true;
  state.emuApplyComplete = false;
  state.emuSelectionExplicit = true;
  state.emuSelectedFiles = new Set();
  closeHistory();
  refreshEmuPatchedState(patched);
  hideEmuAlreadyPatchedNotice();
  setEmuEditMode(true);

  goToStep(5);
  if (els.emuApplyStatus) els.emuApplyStatus.classList.add('hidden');
  await hydrateEmuPatchedContext();
}

function setEmuApplyStatus(kind, text, details) {
  if (!els.emuApplyStatus) return;
  els.emuApplyStatus.classList.remove(
    'hidden', 'completion-message--success', 'completion-message--error', 'completion-message--details'
  );
  if (kind === 'success') els.emuApplyStatus.classList.add('completion-message--success');
  else if (kind === 'error') els.emuApplyStatus.classList.add('completion-message--error');
  if (details) els.emuApplyStatus.classList.add('completion-message--details');
  els.emuApplyStatus.textContent = details ? `${text}\n\n${details}` : text;
}

function emuFileLabel(path) {
  const parts = String(path ?? '').split(/[\\/]/).filter(Boolean);
  return parts.slice(-2).join('/') || String(path ?? '');
}

const EMU_BOOL_KEYS = new Set([
  'offline', 'steam_deck', 'disable_networking', 'disable_lan_only',
  'record_playtime', 'achievements_bypass', 'force_steamhttp_success',
  'enable_steam_preowned_ids', 'free_weekend',
  'enable_experimental_overlay', 'disable_achievement_notification',
  'overlay_always_show_fps', 'overlay_always_show_playtime',
]);
const EMU_FLOAT_KEYS = new Set(['font_size']);

const EMU_LAST_SETTINGS_KEY = 'lastEmuSettings_v1';

function loadLastEmuSettings() {
  try {
    const raw = localStorage.getItem(EMU_LAST_SETTINGS_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function saveLastEmuSettings(settings) {
  try {
    localStorage.setItem(EMU_LAST_SETTINGS_KEY, JSON.stringify(settings || {}));
  } catch {}
}

function gatherEmuSettings() {
  const settings = {};
  let anySet = false;
  document.querySelectorAll('#step-emulator [data-emu-key]').forEach((el) => {
    const key = el.getAttribute('data-emu-key');
    if (!key) return;
    if (EMU_BOOL_KEYS.has(key)) {
      if (el.checked) {
        settings[key] = true;
        anySet = true;
      }
      return;
    }
    const raw = (el.value || '').trim();
    if (!raw) return;
    if (EMU_FLOAT_KEYS.has(key)) {
      const n = parseFloat(raw);
      if (!Number.isNaN(n)) {
        settings[key] = n;
        anySet = true;
      }
      return;
    }
    settings[key] = raw;
    anySet = true;
  });
  return anySet ? settings : null;
}

function populateEmuSettings(settings) {
  document.querySelectorAll('#step-emulator [data-emu-key]').forEach((el) => {
    const key = el.getAttribute('data-emu-key');
    if (!key) return;
    if (EMU_BOOL_KEYS.has(key)) {
      el.checked = settings && settings[key] === true;
      return;
    }
    if (!settings || settings[key] === null || settings[key] === undefined) {
      el.value = '';
      return;
    }
    el.value = String(settings[key]);
  });
}

function initEmuAccordion() {
  const toggles = document.querySelectorAll('#step-emulator .emu-accordion__toggle');
  toggles.forEach((btn) => {
    btn.addEventListener('click', () => {
      const targetId = btn.getAttribute('data-target');
      const content = targetId ? document.getElementById(targetId) : null;
      if (!content) return;
      const open = content.classList.toggle('hidden');
      btn.setAttribute('aria-expanded', String(!open));
    });
  });
}
