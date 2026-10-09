async function openHistory() {
  hideHistoryBanner();
  emitEvent('history_action', { action: 'opened' });
  if (state.historyView) state.historyView.query = '';
  els.historyModal.classList.remove('hidden');
  await loadHistory();
}

function closeHistory() {
  els.historyModal.classList.add('hidden');
}

async function loadHistory() {
  els.historyList.innerHTML = `<div class="history-loading"><div class="spinner"></div><span>${escapeHtml(window.i18n.t('history.loading'))}</span></div>`;

  try {
    try {
      const s = await invoke('get_settings');
      state.useNativeDownloader = s.use_native_downloader !== false;
    } catch (_) {
      state.useNativeDownloader = true;
    }
    const entries = await invoke('get_history');
    renderHistory(entries);
  } catch (e) {
    els.historyList.innerHTML = `<div class="history-empty">${escapeHtml(window.i18n.t('history.loadFailed'))}</div>`;
    console.error('Failed to load history:', e);
  }
}

const HISTORY_VIEW_KEY = 'smd.historyView';

function loadHistoryView() {
  const view = { query: '', status: 'all', sort: 'newest' };
  try {
    const saved = JSON.parse(localStorage.getItem(HISTORY_VIEW_KEY) || '{}');
    if (['all', ...HISTORY_KINDS].includes(saved.status)) view.status = saved.status;
    if (['newest', 'oldest', 'name', 'size'].includes(saved.sort)) view.sort = saved.sort;
  } catch (_) {}
  return view;
}

function saveHistoryView() {
  try {
    localStorage.setItem(HISTORY_VIEW_KEY, JSON.stringify({ status: state.historyView.status, sort: state.historyView.sort }));
  } catch (_) {}
}

function relativeToDir(path, dir) {
  const norm = (p) => String(p || '').replace(/\\/g, '/').replace(/\/+$/, '');
  const base = norm(dir);
  const full = norm(path);
  return base && full.toLowerCase().startsWith(`${base.toLowerCase()}/`) ? full.slice(base.length + 1) : full;
}

function exePlatformBadge(exe, all) {
  const platforms = new Set((all || []).map((e) => e.platform).filter(Boolean));
  if (platforms.size < 2 || !exe.platform) return '';
  const label = window.i18n.t(exe.platform === 'linux' ? 'depotTags.linux' : 'depotTags.windows');
  return ` <span class="depot-tag depot-tag--${exe.platform === 'linux' ? 'linux' : 'windows'}">${escapeHtml(label)}</span>`;
}

function chooseLaunchExe(candidates, dir) {
  const modal = document.getElementById('launch-modal');
  const list = document.getElementById('launch-list');
  let chosen = (candidates.find((c) => c.recommended) || candidates[0]).path;
  list.innerHTML = candidates.map((c, i) => `
    <label class="launch-option">
      <input type="radio" name="launch-exe" value="${i}"${c.path === chosen ? ' checked' : ''}>
      <span class="launch-option__name">${escapeHtml(c.name)}${exePlatformBadge(c, candidates)}</span>
      <span class="launch-option__path" title="${escapeHtml(c.path)}">${escapeHtml(relativeToDir(c.path, dir))}</span>
      <span class="launch-option__size">${escapeHtml(formatBytes(c.size) || '')}</span>
    </label>`).join('');
  list.querySelectorAll('input').forEach((input) => {
    input.addEventListener('change', () => { chosen = candidates[Number(input.value)].path; });
  });
  return new Promise((resolve) => {
    const finish = (value) => {
      modal.classList.add('hidden');
      resolve(value);
    };
    document.getElementById('btn-launch-start').onclick = () => finish(chosen);
    document.getElementById('btn-launch-cancel').onclick = () => finish(null);
    modal.querySelector('.modal__backdrop').onclick = () => finish(null);
    modal.classList.remove('hidden');
  });
}

async function playGame(entry, choose) {
  const dir = entry.download_dir;
  let exe = null;
  try {
    if (!choose) exe = await invoke('get_launch_exe', { dir });
    if (!exe) {
      const detected = await invoke('detect_executables', { downloadDir: dir });
      const candidates = (detected && detected.executables) || [];
      if (candidates.length === 0) {
        showHistoryBanner(window.i18n.t(entry.status === 'partial' ? 'history.playNoExePartial' : 'history.playNoExe'), 9000);
        return;
      }
      exe = candidates.length === 1 && !choose ? candidates[0].path : await chooseLaunchExe(candidates, dir);
      if (!exe) return;
    }
    const method = await invoke('launch_game', { dir, exe });
    emitEvent('game_launched', { method: ['steam', 'wine', 'native'].includes(method) ? method : 'direct', chosen: !!choose, partial: entry.status === 'partial' });
    const key = method === 'steam' ? 'history.playStartedSteam' : method === 'wine' ? 'history.playStartedWine' : 'history.playStarted';
    showHistoryBanner(window.i18n.t(key, { name: entry.game_name || `App ${entry.app_id}` }), 5000, 'success');
  } catch (err) {
    const text = String(err);
    if (text.includes('WINDOWS_GAME_NEEDS_PROTON')) {
      reportError('play', null, 'history.playNeedsProton');
      showHistoryBanner(window.i18n.t('history.playNeedsProton'), 12000);
    } else {
      reportError('play', text);
      showHistoryBanner(window.i18n.t('history.playFailed', { message: window.i18n.localizeError(text) }));
    }
  }
}

function startHistoryRedownload(appId, depotIds, target) {
  state.updateDir = target ? target.dir : null;
  state.updateAppId = target ? String(appId) : null;
  state.repairManifests = target && target.manifests ? target.manifests : null;
  closeHistory();
  state.emuStandalone = false;
  setEmuEditMode(false);
  state.parsedData = null;
  state.selectedDepots.clear();
  state.jobId = null;
  state.depotManifests = {};
  state.searchRepos = [];
  state.selectedRepo = null;
  state.searchAppId = null;
  state.searchSha = null;
  state.searchKeyVdfKeys = null;
  cleanupProgressListener();
  els.gameInfoBanner.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchError.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  els.manifestLoading.classList.add('hidden');
  resetUpload();
  autoRedownloadPending = true;
  autoSelectAllOnStep2 = true;
  autoSelectDepotIds = depotIds.length > 0 ? depotIds : null;
  switchTab('search');
  els.searchAppIdInput.value = appId;
  goToStep(1);
  performSearch();
}

const HISTORY_KINDS = ['complete', 'partial', 'resumable', 'cancelled', 'failed'];
const HISTORY_KIND_LABELS = {
  complete: 'history.statusComplete',
  partial: 'history.statusPartial',
  resumable: 'history.statusResumable',
  cancelled: 'history.statusCancelled',
  failed: 'history.statusFailed',
};

function historyEntryKind(entry) {
  if (entry.status === 'complete') return 'complete';
  if (entry.status === 'partial') return 'partial';
  if (entry.status === 'cancelled_resumable' && entry.resume_payload) return 'resumable';
  if (entry.status === 'cancelled' || entry.status === 'cancelled_resumable') return 'cancelled';
  return 'failed';
}

function historyEntryTime(entry) {
  const t = Date.parse(entry.completed_at || entry.started_at || '');
  return Number.isNaN(t) ? 0 : t;
}

function visibleHistory() {
  const view = state.historyView;
  const query = view.query.trim().toLowerCase();
  const list = (state.cachedHistory || []).filter((entry) => {
    if (view.status !== 'all' && historyEntryKind(entry) !== view.status) return false;
    if (!query) return true;
    return (entry.game_name || '').toLowerCase().includes(query) || String(entry.app_id).includes(query);
  });
  const byName = (e) => (e.game_name || `App ${e.app_id}`).toLowerCase();
  const sorters = {
    newest: (a, b) => historyEntryTime(b) - historyEntryTime(a),
    oldest: (a, b) => historyEntryTime(a) - historyEntryTime(b),
    name: (a, b) => byName(a).localeCompare(byName(b), window.i18n.getCurrentLocale()),
    size: (a, b) => (b.size_bytes || 0) - (a.size_bytes || 0),
  };
  return list.sort(sorters[view.sort] || sorters.newest);
}

function renderHistoryFilters() {
  const view = state.historyView;
  const counts = {};
  (state.cachedHistory || []).forEach((e) => {
    const kind = historyEntryKind(e);
    counts[kind] = (counts[kind] || 0) + 1;
  });
  if (view.status !== 'all' && !counts[view.status]) view.status = 'all';
  const chips = [['all', window.i18n.t('history.filterAll'), (state.cachedHistory || []).length]]
    .concat(HISTORY_KINDS.filter((k) => counts[k]).map((k) => [k, window.i18n.t(HISTORY_KIND_LABELS[k]), counts[k]]));
  els.historyFilters.innerHTML = chips.map(([kind, label, count]) => `
    <button type="button" class="history-chip history-chip--${kind}${kind === view.status ? ' is-active' : ''}" data-status="${kind}" role="radio" aria-checked="${kind === view.status}">
      ${escapeHtml(label)}<span class="history-chip__count">${count}</span>
    </button>`).join('');
  els.historyFilters.querySelectorAll('.history-chip').forEach((chip) => {
    chip.addEventListener('click', () => {
      state.historyView.status = chip.dataset.status;
      saveHistoryView();
      renderHistoryFilters();
      renderHistoryEntries(visibleHistory());
    });
  });
}

function syncHistorySort() {
  const sort = state.historyView.sort;
  els.historySortMenu.querySelectorAll('.history-sort__option').forEach((opt) => {
    const active = opt.dataset.value === sort;
    opt.classList.toggle('is-active', active);
    opt.setAttribute('aria-selected', active ? 'true' : 'false');
    if (active) els.historySortLabel.textContent = opt.textContent;
  });
}

function setHistorySortOpen(open) {
  els.historySortMenu.classList.toggle('hidden', !open);
  els.historySortButton.setAttribute('aria-expanded', open ? 'true' : 'false');
  els.historySort.classList.toggle('is-open', open);
  if (open) {
    const active = els.historySortMenu.querySelector('.is-active') || els.historySortMenu.firstElementChild;
    els.historySortMenu.querySelectorAll('.history-sort__option').forEach((o) => o.classList.toggle('is-focused', o === active));
  }
}

function chooseHistorySort(value) {
  state.historyView.sort = value;
  saveHistoryView();
  syncHistorySort();
  setHistorySortOpen(false);
  els.historySortButton.focus();
  renderHistoryEntries(visibleHistory());
}

function syncHistoryToolbar() {
  const view = state.historyView;
  els.historyToolbar.classList.toggle('hidden', (state.cachedHistory || []).length === 0);
  if (els.historySearch.value !== view.query) els.historySearch.value = view.query;
  renderHistoryFilters();
  syncHistorySort();
}

function initHistoryToolbar() {
  state.historyView = loadHistoryView();
  els.historySearch.addEventListener('input', () => {
    state.historyView.query = els.historySearch.value;
    renderHistoryEntries(visibleHistory());
  });
  els.historySortButton.addEventListener('click', () => {
    setHistorySortOpen(els.historySortMenu.classList.contains('hidden'));
  });
  els.historySortMenu.querySelectorAll('.history-sort__option').forEach((opt) => {
    opt.addEventListener('click', () => chooseHistorySort(opt.dataset.value));
    opt.addEventListener('mouseenter', () => {
      els.historySortMenu.querySelectorAll('.history-sort__option').forEach((o) => o.classList.toggle('is-focused', o === opt));
    });
  });
  els.historySort.addEventListener('keydown', (e) => {
    const open = !els.historySortMenu.classList.contains('hidden');
    const options = Array.from(els.historySortMenu.querySelectorAll('.history-sort__option'));
    const focused = options.findIndex((o) => o.classList.contains('is-focused'));
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      setHistorySortOpen(false);
      els.historySortButton.focus();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) {
        setHistorySortOpen(true);
        return;
      }
      const next = (focused + (e.key === 'ArrowDown' ? 1 : options.length - 1)) % options.length;
      options.forEach((o, i) => o.classList.toggle('is-focused', i === next));
    } else if ((e.key === 'Enter' || e.key === ' ') && open && focused >= 0) {
      e.preventDefault();
      chooseHistorySort(options[focused].dataset.value);
    } else if (e.key === 'Tab' && open) {
      setHistorySortOpen(false);
    }
  }, true);
  document.addEventListener('click', (e) => {
    if (!els.historySort.contains(e.target)) setHistorySortOpen(false);
  });
}

const UPDATE_CHECK_TTL = 10 * 60 * 1000;
const QUEUE_NEXT_DELAY_SECONDS = 5;

function updateCheckKey(entries) {
  return entries
    .filter((e) => e.status === 'complete' && e.download_dir)
    .map((e) => e.id)
    .sort()
    .join(',');
}

async function refreshUpdateChecks() {
  const entries = state.cachedHistory || [];
  const key = updateCheckKey(entries);
  if (!key) return;
  const cache = state.updateChecks;
  if (cache && (cache.key === key && Date.now() - cache.at < UPDATE_CHECK_TTL || cache.pending === key)) return;
  state.updateChecks = { ...(cache || { byEntry: {} }), pending: key };
  try {
    const list = await invoke('check_game_updates');
    emitEvent('updates_found', { count: countBucket((list || []).filter(c => c.update_available).length) });
    const byEntry = {};
    (list || []).forEach((c) => { byEntry[c.entry_id] = c; });
    state.updateChecks = { key, at: Date.now(), byEntry, pending: null };
    if (!els.historyModal.classList.contains('hidden')) renderHistoryEntries(visibleHistory());
  } catch (e) {
    console.warn('check_game_updates failed:', e);
    state.updateChecks = { key, at: Date.now(), byEntry: (cache && cache.byEntry) || {}, pending: null };
  }
}

function updateCheckFor(entry) {
  const checks = state.updateChecks && state.updateChecks.byEntry;
  return checks ? checks[entry.id] : null;
}

const COVER_PLACEHOLDER = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>';

function historyCoverFailed(img) {
  const fallback = img.dataset.fallback;
  if (fallback) {
    img.dataset.fallback = '';
    img.src = fallback;
    return;
  }
  const placeholder = document.createElement('div');
  placeholder.className = 'history-entry__image history-entry__image--placeholder';
  placeholder.innerHTML = COVER_PLACEHOLDER;
  img.replaceWith(placeholder);
}

async function refreshCovers() {
  state.covers = state.covers || {};
  const ids = [...new Set((state.cachedHistory || []).map((e) => String(e.app_id)))]
    .filter((id) => !(id in state.covers));
  if (ids.length === 0) return;
  ids.forEach((id) => { state.covers[id] = null; });
  let found = {};
  try {
    found = (await invoke('get_covers', { appIds: ids })) || {};
  } catch (e) {
    console.warn('get_covers failed:', e);
    ids.forEach((id) => { delete state.covers[id]; });
    return;
  }
  Object.assign(state.covers, found);
  if (Object.keys(found).length && !els.historyModal.classList.contains('hidden')) {
    renderHistoryEntries(visibleHistory());
  }
}

function renderHistory(entries) {
  state.cachedHistory = entries || [];
  if (!state.historyView) state.historyView = loadHistoryView();
  syncHistoryToolbar();
  renderHistoryEntries(visibleHistory());
  refreshUpdateChecks();
  refreshCovers();
}

function renderHistoryEntries(entries) {
  if (state.cachedHistory.length === 0) {
    els.historyList.innerHTML = `<div class="history-empty">${escapeHtml(window.i18n.t('history.empty'))}</div>`;
    els.btnHistoryClear.style.display = 'none';
    return;
  }

  els.btnHistoryClear.style.display = '';
  if (entries.length === 0) {
    els.historyList.innerHTML = `<div class="history-empty">${escapeHtml(window.i18n.t('history.noMatch'))}</div>`;
    return;
  }
  const editTip = window.i18n.t('emulator.history.editTooltip');
  const entryById = new Map(state.cachedHistory.map(e => [e.id, e]));
  els.historyList.innerHTML = entries.map(entry => {
    const date = entry.completed_at ? formatHistoryDate(entry.completed_at) : formatHistoryDate(entry.started_at);
    const isResumable = entry.status === 'cancelled_resumable' && !!entry.resume_payload;
    const canResumeNow = isResumable && state.useNativeDownloader !== false;
    const kind = historyEntryKind(entry);
    const badgeClass = `history-entry__badge--${kind}`;
    const statusLabel = window.i18n.t(HISTORY_KIND_LABELS[kind]);
    const cover = state.covers && state.covers[String(entry.app_id)];
    const imgSrc = cover || entry.header_image;
    const imgHtml = imgSrc
      ? `<img class="history-entry__image" src="${escapeHtml(imgSrc)}" data-fallback="${escapeHtml(cover && entry.header_image ? entry.header_image : '')}" alt="" loading="lazy" onerror="historyCoverFailed(this)">`
      : '<div class="history-entry__image history-entry__image--placeholder"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></div>';
    const name = entry.game_name ? escapeHtml(entry.game_name) : `App ${escapeHtml(entry.app_id)}`;
    const canUpdate = entry.status === 'complete' && !!entry.download_dir;
    const canPlay = (entry.status === 'complete' || entry.status === 'partial') && !!entry.download_dir;
    const updateCheck = canUpdate ? updateCheckFor(entry) : null;
    const hasUpdate = !!(updateCheck && updateCheck.update_available);
    const updateTip = window.i18n.t(!canUpdate ? 'history.updateUnavailable' : hasUpdate ? 'history.updateAvailableTooltip' : 'history.updateTooltip');

    return `
      <div class="history-entry" data-entry-id="${escapeHtml(entry.id)}" data-app-id="${escapeHtml(entry.app_id)}">
        ${imgHtml}
        <div class="history-entry__info">
          <div class="history-entry__name">${name}</div>
          <div class="history-entry__meta">
            <span class="history-entry__appid">App ${escapeHtml(entry.app_id)}</span>
            <span class="history-entry__date">${date}</span>
            ${entry.size_bytes ? `<span class="history-entry__size">${escapeHtml(formatBytes(entry.size_bytes) || '')}</span>` : ''}
          </div>
          <div class="history-entry__depots">${window.i18n.t('history.depotsDownloaded', { done: entry.depots_downloaded, total: entry.depot_count })}</div>
          <div class="history-entry__status">
            <span class="history-entry__badge ${badgeClass}">${statusLabel}</span>
            ${hasUpdate ? `<span class="history-entry__badge history-entry__badge--update">${escapeHtml(window.i18n.t('history.updateAvailable'))}</span>` : ''}
          </div>
        </div>
        <div class="history-entry__actions">
          ${canResumeNow
            ? `<button class="btn btn--small btn--primary history-action-resume" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(window.i18n.t('history.resumeTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.resumeTooltip'))}">${ICONS.play}</button>`
            : ''}
          ${canPlay
            ? `<button class="btn btn--small btn--outline history-action-play" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(window.i18n.t('history.playTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.playTooltip'))}">${ICONS.gamepad}</button>`
            : ''}
          <button class="btn btn--small btn--outline history-action-redownload" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" title="${escapeHtml(window.i18n.t('history.redownloadTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.redownloadTooltip'))}">${ICONS.refresh}</button>
          <button class="btn btn--small ${hasUpdate ? 'btn--primary' : 'btn--outline'} history-action-update" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" data-path="${escapeHtml(entry.download_dir)}" title="${escapeHtml(updateTip)}" aria-label="${escapeHtml(updateTip)}"${canUpdate ? '' : ' disabled'}>${ICONS.update}</button>
          <button class="btn btn--small btn--outline history-action-repair" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(window.i18n.t(canUpdate ? 'history.repairTooltip' : 'history.repairUnavailable'))}" aria-label="${escapeHtml(window.i18n.t(canUpdate ? 'history.repairTooltip' : 'history.repairUnavailable'))}"${canUpdate ? '' : ' disabled'}>${ICONS.shieldCheck}</button>
          <button class="btn btn--small btn--outline history-action-folder" data-path="${escapeHtml(entry.download_dir)}" title="${escapeHtml(window.i18n.t('history.openFolderTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.openFolderTooltip'))}"${entry.status === 'cancelled' ? ' disabled' : ''}>${ICONS.folderOpen}</button>
          <button class="btn btn--small btn--outline history-action-edit-emu" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(editTip)}" aria-label="${escapeHtml(editTip)}"${entry.status === 'cancelled' || !entry.download_dir ? ' disabled' : ''}>${ICONS.settings}</button>
          <button class="btn btn--small btn--outline history-action-remove" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(window.i18n.t('history.removeTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.removeTooltip'))}">${ICONS.trash}</button>
        </div>
      </div>
    `;
  }).join('');

  els.historyList.querySelectorAll('.history-action-resume').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const entry = entryById.get(btn.dataset.entryId);
      if (!entry || !entry.resume_payload) return;
      const resumeMode = await askResumeMode();
      if (!resumeMode) return;
      closeHistory();
      state.emuStandalone = false;
      setEmuEditMode(false);
      cleanupProgressListener();
      try {
        const settings = await invoke('get_settings');
        state.currentEngine = settings.use_native_downloader !== false ? 'native' : 'ddm';

        const resumeDepots = (entry.resume_payload.selectedDepots || []).length;
        state.dlNonce = telemetryNonce();
        state.dlStartedAt = Date.now();
        state.dlStage = 'starting';
        state.dlDepotCount = resumeDepots;
        state.dlSourceCount = Array.isArray(settings.depot_sources) ? settings.depot_sources.length : null;
        state.dlHadMhKey = !!(entry.resume_payload.manifestHubApiKey || '');
        state.dlContext = { mode: 'resume', selection: 'resume', resume_mode: resumeMode === 'fast' ? 'fast' : 'full' };
        emitEvent('download_started', Object.assign({
          job: state.dlNonce,
          depot_count: resumeDepots,
          engine: state.currentEngine,
          source_count: state.dlSourceCount,
          had_mh_key: state.dlHadMhKey,
          resumed: true,
        }, state.dlContext));

        state.repairRunning = false;
        if (els.btnReportFailure) els.btnReportFailure.classList.add('hidden');
        state.parsedData = { mainAppId: entry.app_id, depots: [] };
        state.gameName = entry.game_name || null;
        state.headerImage = entry.header_image || null;
        goToStep(3);
        initProgressUI((entry.resume_payload.selectedDepots || []).map(d => ({
          depotId: String(d.depotId),
          manifestId: String(d.manifestId || ''),
          sizeBytes: null,
        })));
        await connectProgressListener();

        const result = await invoke('start_download', { config: { ...entry.resume_payload, resumeMode } });
        if (!state.jobId) state.jobId = result.jobId;
        state.downloadDir = result.downloadDir || entry.download_dir;
        try {
          await invoke('remove_history_entry', { entryId: entry.id });
        } catch (rmErr) {
          console.warn('remove_history_entry failed:', rmErr);
        }
      } catch (err) {
        console.error('resume start_download failed:', err);
        alert(window.i18n.t('history.resumeError', { message: window.i18n.localizeError(err) }));
        openHistory();
      }
    });
  });

  els.historyList.querySelectorAll('.history-action-play').forEach(btn => {
    const entry = entryById.get(btn.dataset.entryId);
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      if (entry) playGame(entry, false);
    });
    btn.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      if (entry) playGame(entry, true);
    });
  });

  els.historyList.querySelectorAll('.history-action-repair').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      const entry = entryById.get(btn.dataset.entryId);
      if (!entry || !entry.download_dir) return;
      let installed = [];
      try {
        installed = await invoke('get_installed_depots', { dir: entry.download_dir }) || [];
      } catch (err) {
        console.error('get_installed_depots failed:', err);
      }
      const manifests = {};
      installed.forEach((d) => { manifests[String(d.depot_id)] = String(d.manifest_id); });
      const depotIds = installed.length ? Object.keys(manifests) : (entry.depot_ids || []);
      emitEvent('history_action', { action: 'repair' });
      startHistoryRedownload(entry.app_id, depotIds, { dir: entry.download_dir, manifests });
    });
  });

  els.historyList.querySelectorAll('.history-action-redownload, .history-action-update').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const depotIds = (btn.dataset.depotIds || '').split(',').filter(Boolean);
      const isUpdate = btn.classList.contains('history-action-update');
      emitEvent('history_action', { action: isUpdate ? 'update' : 'redownload' });
      startHistoryRedownload(btn.dataset.appId, depotIds, isUpdate ? { dir: btn.dataset.path || null } : null);
    });
  });

  els.historyList.querySelectorAll('.history-action-folder').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await invoke('open_folder', { path: btn.dataset.path });
        emitEvent('history_action', { action: 'open_folder' });
      } catch (err) {
        console.error('Failed to open folder:', err);
        showFolderMissing();
      }
    });
  });

  els.historyList.querySelectorAll('.history-action-remove').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const entry = entryById.get(btn.dataset.entryId);
      const resumable = !!(entry && entry.status === 'cancelled_resumable' && entry.resume_payload);
      showHistoryRemoveConfirm(btn.dataset.entryId, resumable);
    });
  });

  els.historyList.querySelectorAll('.history-action-edit-emu').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const entry = entryById.get(btn.dataset.entryId);
      if (entry) emitEvent('history_action', { action: 'edit_emu' });
      if (entry) openEmuEditFromHistory(entry);
    });
  });

}

function formatHistoryDate(dateStr) {
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleString(window.i18n.getCurrentLocale(), { dateStyle: 'medium', timeStyle: 'short' });
  } catch {
    return dateStr;
  }
}

async function clearHistory(deleteResumableFiles = false) {
  try {
    await invoke('clear_history', { deleteResumableFiles });
    emitEvent('history_action', { action: 'clear', delete_files: !!deleteResumableFiles });
    await loadHistory();
  } catch (e) {
    console.error('Failed to clear history:', e);
    alert(window.i18n.localizeError(e));
    await loadHistory();
  }
}

function showHistoryClearConfirm() {
  if (!els.historyClearModal) return;
  const warningEl = document.getElementById('history-clear-resumable-warning');
  const warningTextEl = document.getElementById('history-clear-resumable-text');
  const checkbox = document.getElementById('history-clear-delete-files');
  const bodyEl = els.historyClearModal.querySelector('.modal__text[data-i18n-html="modals.historyClear.body"]');
  const resumableCount = (state.cachedHistory || []).filter(
    e => e.status === 'cancelled_resumable' && e.resume_payload
  ).length;
  if (warningEl && warningTextEl && checkbox) {
    if (resumableCount > 0) {
      warningEl.classList.remove('hidden');
      warningTextEl.innerHTML = window.i18n.t('modals.historyClear.resumableWarning', { count: resumableCount });
      checkbox.checked = false;
      if (bodyEl) bodyEl.innerHTML = window.i18n.t('modals.historyClear.bodyShort');
    } else {
      warningEl.classList.add('hidden');
      checkbox.checked = false;
      if (bodyEl) bodyEl.innerHTML = window.i18n.t('modals.historyClear.body');
    }
  }
  els.historyClearModal.classList.remove('hidden');
}

async function confirmHistoryClear() {
  const checkbox = document.getElementById('history-clear-delete-files');
  const deleteFiles = !!(checkbox && checkbox.checked);
  if (els.historyClearModal) els.historyClearModal.classList.add('hidden');
  await clearHistory(deleteFiles);
}

function showHistoryRemoveConfirm(entryId, resumable = false) {
  state.pendingHistoryRemoveId = entryId;
  state.pendingHistoryRemoveResumable = !!resumable;
  if (!els.historyRemoveModal) return;
  const titleEl = els.historyRemoveModal.querySelector('#history-remove-modal-title');
  const bodyEl = els.historyRemoveModal.querySelector('.modal__text');
  const yesBtn = els.historyRemoveModal.querySelector('#btn-history-remove-yes');
  if (resumable) {
    if (titleEl) titleEl.innerHTML = window.i18n.t('modals.historyRemove.titleResumable');
    if (bodyEl) bodyEl.innerHTML = window.i18n.t('modals.historyRemove.bodyResumable');
    if (yesBtn) yesBtn.textContent = window.i18n.t('modals.historyRemove.yesResumable');
  } else {
    if (titleEl) titleEl.innerHTML = window.i18n.t('modals.historyRemove.title');
    if (bodyEl) bodyEl.innerHTML = window.i18n.t('modals.historyRemove.body');
    if (yesBtn) yesBtn.textContent = window.i18n.t('modals.historyRemove.yes');
  }
  els.historyRemoveModal.classList.remove('hidden');
}

async function confirmHistoryRemove() {
  const id = state.pendingHistoryRemoveId;
  const deleteFiles = !!state.pendingHistoryRemoveResumable;
  state.pendingHistoryRemoveId = null;
  state.pendingHistoryRemoveResumable = false;
  if (els.historyRemoveModal) els.historyRemoveModal.classList.add('hidden');
  if (!id) return;
  try {
    await invoke('remove_history_entry', { entryId: id, deleteFiles });
    emitEvent('history_action', { action: 'remove', delete_files: !!deleteFiles });
    await loadHistory();
  } catch (err) {
    console.error('Failed to remove history entry:', err);
    alert(window.i18n.localizeError(err));
    await loadHistory();
  }
}
