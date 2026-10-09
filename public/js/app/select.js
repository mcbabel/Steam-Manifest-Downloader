async function loadSettingsAndDefaults() {
  try {
    const settings = await invoke('get_settings');
    defaultDownloadDir = settings.download_location || '';
    state.notificationSoundEnabled = settings.notification_sound !== false;
    if (els.downloadDirInput) {
      els.downloadDirInput.value = defaultDownloadDir;
    }
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function getDownloadDir() {
  const val = els.downloadDirInput ? els.downloadDirInput.value.trim() : '';
  return val || defaultDownloadDir;
}

async function saveDownloadDir() {
  const dir = getDownloadDir();
  if (dir) {
    try {
      const settings = await invoke('get_settings');
      settings.download_location = dir;
      await invoke('save_settings', { settings });
    } catch (e) {
      console.error('Failed to save download dir:', e);
    }
  }
}

async function browseDownloadDir() {
  try {
    const { open } = window.__TAURI__.dialog;
    const selected = await open({
      directory: true,
      multiple: false,
      title: window.i18n.t('select.chooseLocationTitle')
    });
    if (selected) {
      els.downloadDirInput.value = selected;
    }
  } catch (e) {
    console.error('Failed to open folder dialog:', e);
  }
}

function goBackToSelect() {
  cleanupProgressListener();
  state.jobId = null;
  // Go back to Step 2 (select) — parsedData and selectedDepots are still intact
  goToStep(2);
}

async function fetchGameInfo(appId) {
  els.gameInfoBanner.classList.add('hidden');
  els.gameInfoLoading.classList.remove('hidden');

  try {
    const info = await invoke('get_steam_app_info', { appId: String(appId) });

    if (info) {
      const { name, headerImage, shortDescription } = info;

      if (headerImage) {
        els.gameHeaderImage.src = headerImage;
        els.gameHeaderImage.alt = name || window.i18n.t('common.gameCover');
        state.headerImage = headerImage;
      }

      if (name) {
        els.gameName.textContent = name;
        state.gameName = name;
      }

      if (shortDescription) {
        els.gameDescription.textContent = shortDescription;
      }

      els.gameInfoLoading.classList.add('hidden');
      els.gameInfoBanner.classList.remove('hidden');
      return;
    }
  } catch (e) {
  }

  els.gameInfoLoading.classList.add('hidden');
}

function showSelectionStep() {
  const data = state.parsedData;
  if (!data) return;

  els.appIdDisplay.textContent = data.mainAppId;
  els.depotCount.textContent = window.i18n.t('select.depotsFound', { count: data.depots.length });

  // Fetch game info from Steam (async, non-blocking) — only if not already fetched by search
  if (state.mode !== 'search' || !state.gameName) {
    fetchGameInfo(data.mainAppId);
  } else {
    if (state.headerImage) {
      els.gameHeaderImage.src = state.headerImage;
      els.gameHeaderImage.alt = state.gameName || window.i18n.t('common.gameCover');
    }
    if (state.gameName) els.gameName.textContent = state.gameName;
    els.gameInfoLoading.classList.add('hidden');
    if (state.headerImage || state.gameName) {
      els.gameInfoBanner.classList.remove('hidden');
    }
  }

  const savedApiKey = localStorage.getItem(MH_APIKEY_STORAGE_KEY);
  if (savedApiKey) els.mhApiKey.value = savedApiKey;

  if (els.downloadDirInput && defaultDownloadDir) {
    els.downloadDirInput.value = els.downloadDirInput.value || defaultDownloadDir;
  }

  els.depotList.innerHTML = '';
  state.selectedDepots.clear();
  cancelAutoStart();
  state.depotAutoPending = !autoSelectAllOnStep2;
  state.depotSelectionTouched = false;
  state.depotPicsList = null;
  state.depotSkipReasons = {};
  if (els.autoSelectNote) els.autoSelectNote.classList.add('hidden');
  if (els.btnAutoSelect) els.btnAutoSelect.disabled = true;
  state.depotIncludeDlc = null;
  syncSteamButtons();

  state.depotManifests = {};
  const markKeyless = data.depots.some(depotHasKey);

  data.depots.forEach((depot) => {
    const sizeFormatted = formatBytes(depot.sizeBytes);
    const item = document.createElement('div');
    item.className = 'depot-item';
    item.dataset.depotId = depot.depotId;
    if (depot.sizeBytes) item.dataset.sizeBytes = depot.sizeBytes;
    const safeDepotId = escapeHtml(String(depot.depotId));
    const safeManifestId = escapeHtml(String(depot.manifestId || 'N/A'));
    const safeSize = sizeFormatted ? escapeHtml(sizeFormatted) : '';
    item.innerHTML = `
      <div class="depot-item__checkbox"></div>
      <div class="depot-item__info">
        <div class="depot-item__header">
          <span class="depot-item__depot-id">Depot ${safeDepotId}<span class="depot-item__name" data-depot-name="${safeDepotId}"></span>${safeSize ? `<span class="depot-item__size">${safeSize}</span>` : ''}</span>
          <span class="depot-item__tags" data-depot-tags="${safeDepotId}"></span>
        </div>
        <div class="depot-item__manifest-id">Manifest: ${safeManifestId}${markKeyless && !depotHasKey(depot) ? ` <span class="depot-tag depot-tag--nokey" title="${escapeHtml(window.i18n.t('select.noKeyHint'))}">${escapeHtml(window.i18n.t('select.skipReason.no_key'))}</span>` : ''}</div>
        <div class="depot-item__manifest-row">
          <input type="text" data-depot-id="${safeDepotId}" class="custom-manifest-input"
            placeholder="${escapeHtml(window.i18n.t('select.customManifestPlaceholder'))}"
            onclick="event.stopPropagation()">
          <button type="button" class="depot-manifest-action depot-manifest-fetch-btn" data-depot-id="${safeDepotId}"
            title="${escapeHtml(window.i18n.t('depots.fetchLatest'))}" aria-label="${escapeHtml(window.i18n.t('depots.fetchLatest'))}">
            ${ICONS.refresh}
          </button>
          <button type="button" class="depot-manifest-action depot-manifest-btn" data-depot-id="${safeDepotId}"
            title="${escapeHtml(window.i18n.t('depots.uploadManifest'))}" aria-label="${escapeHtml(window.i18n.t('depots.uploadManifest'))}">
            ${ICONS.upload}
          </button>
        </div>
        <span class="depot-manifest-status" data-depot-id="${safeDepotId}"></span>
      </div>
    `;

    const fetchBtn = item.querySelector('.depot-manifest-fetch-btn');
    if (fetchBtn) {
      fetchBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        fetchLatestManifestForDepot(depot.depotId, fetchBtn);
      });
    }
    const manifestBtn = item.querySelector('.depot-manifest-btn');
    if (manifestBtn) {
      manifestBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        handleDepotManifestFile(depot.depotId);
      });
    }

    item.addEventListener('click', (e) => {
      if (e.target.tagName === 'INPUT') return;
      if (e.target.closest('button')) return;
      toggleDepot(depot.depotId, item);
    });
    els.depotList.appendChild(item);
  });

  if (els.depotSearch) els.depotSearch.value = '';
  if (els.showSelectedOnly) els.showSelectedOnly.checked = false;

  if (autoSelectAllOnStep2) {
    autoSelectAllOnStep2 = false;
    if (autoSelectDepotIds && autoSelectDepotIds.length > 0) {
      document.querySelectorAll('.depot-item').forEach(item => {
        const depotId = item.dataset.depotId;
        if (autoSelectDepotIds.includes(depotId)) {
          state.selectedDepots.add(depotId);
          item.classList.add('selected');
        }
      });
      autoSelectDepotIds = null;
    } else {
      selectAll();
    }
  }

  updateDownloadButton();
  goToStep(2);

  fetchDepotMetadataAsync(data.mainAppId);
  fetchDepotNamesFromSteam(data.mainAppId);
}

async function fetchDepotMetadataAsync(appId) {
  try {
    const depots = await invoke('fetch_depot_metadata', { appId: String(appId) });
    if (!Array.isArray(depots)) return;
    depots.forEach((d) => renderDepotTags(d));
  } catch (e) {
    console.warn('fetch_depot_metadata failed:', e);
  }
}

async function fetchDepotNamesFromSteam(appId) {
  try {
    const depots = await invoke('fetch_depot_metadata_steam', { appId: String(appId) });
    if (!Array.isArray(depots)) return;
    if (!state.parsedData || String(state.parsedData.mainAppId) !== String(appId)) return;
    state.depotNames = state.depotNames || {};
    state.depotPicsInfo = {};
    depots.forEach((d) => {
      if (!d || !d.depotId) return;
      state.depotPicsInfo[String(d.depotId)] = d;
      const label = d.name && d.name.trim() ? d.name.trim() : null;
      if (label) state.depotNames[String(d.depotId)] = label;
      const el = document.querySelector(`[data-depot-name="${CSS.escape(String(d.depotId))}"]`);
      if (!el) return;
      el.innerHTML = '';
      if (label) {
        const nameSpan = document.createElement('span');
        nameSpan.className = 'depot-item__name-label';
        nameSpan.textContent = ` — ${label}`;
        el.appendChild(nameSpan);
      }
      const depotIdContainer = el.closest('.depot-item__depot-id');
      if (depotIdContainer) {
        depotIdContainer
          .querySelectorAll('.depot-tag[data-role-badge]')
          .forEach(n => n.remove());
      }
      const badge = depotRoleBadge(d);
      if (badge && depotIdContainer) {
        badge.setAttribute('data-role-badge', '1');
        depotIdContainer.appendChild(document.createTextNode(' '));
        depotIdContainer.appendChild(badge);
      }
    });
    reorderDepotCards();
    state.depotPicsList = depots;
    if (els.btnAutoSelect) els.btnAutoSelect.disabled = depots.length === 0;
    syncDlcButton();
    await maybeAutoSelectDepots(depots);
  } catch (e) {
    state.depotAutoPending = false;
    console.warn('fetch_depot_metadata_steam failed:', e);
  }
}

async function maybeAutoSelectDepots(depots) {
  if (!state.parsedData) return;
  const pending = state.depotAutoPending;
  state.depotAutoPending = false;
  let settings = {};
  try {
    settings = (await invoke('get_settings')) || {};
  } catch (_) {}
  state.steamMode = !!settings.auto_select_depots;
  syncSteamButtons();
  if (!pending || !state.steamMode || state.depotSelectionTouched || depots.length === 0) return;
  const selection = await applyRecommendedDepots(depots);
  if (selection && selection.known && selection.selected.length > 0 && settings.auto_start_download
      && !state.depotSelectionTouched && !state.queueRunning) {
    scheduleAutoStart();
  }
}

async function applyRecommendedDepots(depots) {
  const data = state.parsedData;
  if (!data) return null;
  let selection;
  try {
    selection = await invoke('recommend_depots', {
      depots,
      candidates: data.depots.map(d => String(d.depotId)),
      keyed: data.depots.filter(depotHasKey).map(d => String(d.depotId)),
      uiLanguage: window.i18n.getCurrentLocale(),
      includeDlc: state.depotIncludeDlc,
    });
  } catch (e) {
    console.warn('recommend_depots failed:', e);
    return null;
  }
  if (!selection || !Array.isArray(selection.selected) || state.parsedData !== data) return null;
  state.selectedDepots = new Set(selection.selected.map(String));
  state.depotSkipReasons = {};
  (selection.skipped || []).forEach(s => { state.depotSkipReasons[String(s.depotId)] = s.reason; });
  $$('.depot-item').forEach(el => {
    el.classList.toggle('selected', state.selectedDepots.has(el.dataset.depotId));
    renderSkipTag(el);
  });
  reorderDepotCards();
  updateDownloadButton();
  renderAutoSelectNote(selection, data.depots.length);
  if (state.depotIncludeDlc === null) {
    state.depotIncludeDlc = !(selection.skipped || []).some(s => s.reason === 'dlc')
      && candidateDlcDepots().length > 0;
  }
  syncDlcButton();
  return selection;
}

function depotHasKey(depot) {
  if (depot.depotKey) return true;
  const keys = state.mode === 'search' ? state.searchKeyVdfKeys : null;
  return !!(keys && keys[String(depot.depotId)]);
}

function candidateDlcDepots() {
  const data = state.parsedData;
  if (!data || !state.depotPicsInfo) return [];
  return data.depots.filter(d => {
    const info = state.depotPicsInfo[String(d.depotId)];
    return info && info.role === 'dlc' && !info.optionalDlc;
  });
}

function syncSteamButtons() {
  if (els.btnAutoSelect) els.btnAutoSelect.classList.toggle('hidden', !state.steamMode);
  syncDlcButton();
}

function syncDlcButton() {
  const btn = els.btnAutoSelectDlc;
  if (!btn) return;
  const show = !!state.steamMode && !!state.depotPicsList && candidateDlcDepots().length > 0;
  btn.classList.toggle('hidden', !show);
  btn.textContent = window.i18n.t(state.depotIncludeDlc ? 'select.autoSelectNoDlc' : 'select.autoSelectDlc');
}

function renderSkipTag(item) {
  const id = item.dataset.depotId;
  const container = item.querySelector('.depot-item__header');
  if (!container) return;
  container.querySelectorAll('.depot-tag--skipped').forEach(n => n.remove());
  const reason = state.depotSkipReasons && state.depotSkipReasons[id];
  item.classList.toggle('is-skipped', !!reason);
  if (!reason) return;
  const tag = document.createElement('span');
  tag.className = 'depot-tag depot-tag--skipped';
  tag.textContent = window.i18n.t(`select.skipReason.${reason}`);
  container.appendChild(tag);
}

function steamLanguageName(code) {
  const key = `steamLanguages.${code}`;
  const name = window.i18n.t(key);
  return name === key ? capitalize(code) : name;
}

function platformName(platform) {
  const names = { windows: 'Windows', linux: 'Linux', macos: 'macOS' };
  return names[platform] || platform;
}

function renderAutoSelectNote(selection, total) {
  if (!els.autoSelectNote) return;
  if (!selection.known) {
    els.autoSelectNote.classList.add('hidden');
    return;
  }
  els.autoSelectTitle.textContent = window.i18n.t('select.autoSelectTitle', {
    count: selection.selected.length,
    total,
  });
  const counts = {};
  (selection.skipped || []).forEach(s => { counts[s.reason] = (counts[s.reason] || 0) + 1; });
  const parts = [`${platformName(selection.platform)} · ${steamLanguageName(selection.language)}`];
  const skipped = Object.entries(counts)
    .map(([reason, count]) => `${count}× ${window.i18n.t(`select.skipReason.${reason}`)}`);
  if (skipped.length) parts.push(window.i18n.t('select.autoSelectSkipped', { list: skipped.join(', ') }));
  els.autoSelectDetail.textContent = parts.join(' — ');
  els.btnAutoStartCancel.classList.add('hidden');
  els.autoSelectNote.classList.remove('hidden');
}

function scheduleAutoStart() {
  cancelAutoStart();
  let seconds = 5;
  const tick = () => {
    if (seconds <= 0) {
      cancelAutoStart();
      if (state.currentStep === 2 && state.selectedDepots.size > 0) startDownload();
      return;
    }
    els.btnAutoStartCancel.textContent = window.i18n.t('select.autoStartIn', { seconds });
    seconds -= 1;
  };
  els.btnAutoStartCancel.classList.remove('hidden');
  tick();
  state.autoStartTimer = setInterval(tick, 1000);
}

function cancelAutoStart() {
  if (state.autoStartTimer) {
    clearInterval(state.autoStartTimer);
    state.autoStartTimer = null;
  }
  if (els.btnAutoStartCancel) els.btnAutoStartCancel.classList.add('hidden');
}

function markDepotSelectionTouched() {
  state.depotSelectionTouched = true;
  state.depotAutoPending = false;
  cancelAutoStart();
}

function reorderDepotCards() {
  const list = els.depotList;
  if (!list) return;
  const hostOs = navigator.platform.toLowerCase().includes('linux')
    ? 'linux'
    : navigator.platform.toLowerCase().includes('mac')
      ? 'macos'
      : 'windows';
  const items = Array.from(list.querySelectorAll('.depot-item'));
  items.sort((a, b) => depotSortKey(a, hostOs) - depotSortKey(b, hostOs)
    || depotIdNumeric(a) - depotIdNumeric(b));
  items.forEach(el => list.appendChild(el));
}

function depotSortKey(itemEl, hostOs) {
  const depotId = itemEl.dataset.depotId;
  const skipped = state.depotSkipReasons && state.depotSkipReasons[String(depotId)] ? 200 : 0;
  return skipped + depotRoleSortKey(depotId, hostOs);
}

function depotRoleSortKey(depotId, hostOs) {
  const info = state.depotPicsInfo ? state.depotPicsInfo[String(depotId)] : null;
  if (!info) return 100;
  switch (info.role) {
    case 'shared_content':
      return 10;
    case 'platform': {
      const os = (info.oslist || '').toLowerCase();
      if (os.includes(hostOs)) return 20;
      if (os.includes('windows')) return 30;
      if (os.includes('linux')) return 31;
      if (os.includes('mac')) return 32;
      return 33;
    }
    case 'language':
      return 50;
    case 'dlc':
      return 60;
    default:
      return 80;
  }
}

function depotIdNumeric(itemEl) {
  const id = parseInt(itemEl.dataset.depotId, 10);
  return isNaN(id) ? Number.MAX_SAFE_INTEGER : id;
}

function depotRoleBadge(d) {
  const role = d.role;
  const tag = document.createElement('span');
  tag.className = 'depot-tag depot-tag--' + role;
  switch (role) {
    case 'dlc':
      tag.textContent = window.i18n.t('depots.roleDlc');
      break;
    case 'language':
      tag.textContent = d.language
        ? window.i18n.t('depots.roleLanguageWithName', { name: steamLanguageName(d.language.toLowerCase()) })
        : window.i18n.t('depots.roleLanguage');
      break;
    case 'shared_content':
      tag.textContent = window.i18n.t('depots.roleContent');
      break;
    case 'platform':
      tag.textContent = window.i18n.t('depots.rolePlatform');
      break;
    default:
      return null;
  }
  return tag;
}

function renderDepotTags(info) {
  if (!info || !info.depot_id) return;
  const container = document.querySelector(`[data-depot-tags="${CSS.escape(String(info.depot_id))}"]`);
  if (!container) return;

  const tags = [];
  const osList = (info.oslist || '').toLowerCase();
  if (osList.includes('windows')) tags.push(['windows', window.i18n.t('depotTags.windows')]);
  if (osList.includes('linux')) tags.push(['linux', window.i18n.t('depotTags.linux')]);
  if (osList.includes('macos') || osList.includes('mac')) tags.push(['mac', window.i18n.t('depotTags.macos')]);
  if (info.osarch === '64') tags.push(['arch', window.i18n.t('depotTags.arch64')]);
  else if (info.osarch === '32') tags.push(['arch', window.i18n.t('depotTags.arch32')]);
  if (info.language) tags.push(['lang', steamLanguageName(String(info.language).toLowerCase())]);

  container.innerHTML = tags.map(([cls, label]) =>
    `<span class="depot-tag depot-tag--${cls}">${escapeHtml(label)}</span>`
  ).join('');
}

function toggleDepot(depotId, element) {
  markDepotSelectionTouched();
  if (state.selectedDepots.has(depotId)) {
    state.selectedDepots.delete(depotId);
    element.classList.remove('selected');
  } else {
    state.selectedDepots.add(depotId);
    element.classList.add('selected');
  }
  updateDownloadButton();
}

function selectAll() {
  state.parsedData.depots.forEach((depot) => {
    state.selectedDepots.add(depot.depotId);
  });
  $$('.depot-item').forEach((el) => {
    el.classList.add('selected');
  });
  updateDownloadButton();
}

function deselectAll() {
  state.selectedDepots.clear();
  $$('.depot-item').forEach((el) => el.classList.remove('selected'));
  updateDownloadButton();
}

function updateDownloadButton() {
  const count = state.selectedDepots.size;
  els.btnDownload.disabled = count === 0;
  if (els.btnQueueAdd) els.btnQueueAdd.disabled = count === 0;

  let totalBytes = 0;
  let hasSizeInfo = false;
  if (state.parsedData && state.parsedData.depots) {
    for (const depot of state.parsedData.depots) {
      if (state.selectedDepots.has(depot.depotId) && depot.sizeBytes) {
        totalBytes += depot.sizeBytes;
        hasSizeInfo = true;
      }
    }
  }
  let label = window.i18n.t('select.downloadSelected');
  if (count > 0) {
    label = hasSizeInfo
      ? window.i18n.t('select.downloadCountSize', { count, size: formatBytes(totalBytes) })
      : window.i18n.t('select.downloadCount', { count });
  }

  els.btnDownload.innerHTML = `
    ${escapeHtml(label)}
    <svg class="btn__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
      <polyline points="7 10 12 15 17 10"/>
      <line x1="12" y1="15" x2="12" y2="3"/>
    </svg>
  `;
}

function collectDepotConfigs(selectedDepots, repairManifests) {
  return selectedDepots.map(depot => {
    const input = document.querySelector(`.custom-manifest-input[data-depot-id="${depot.depotId}"]`);
    const customManifestId = input ? input.value.trim() : '';
    const depotManifest = state.depotManifests[depot.depotId];
    const displayName = state.depotNames ? state.depotNames[String(depot.depotId)] : null;
    const result = {
      ...depot,
      customManifestId: customManifestId || null,
      displayName: displayName || null,
    };
    if (repairManifests && repairManifests[String(depot.depotId)]) {
      result.customManifestId = repairManifests[String(depot.depotId)];
    }
    if (depotManifest) {
      result.uploadedManifestPath = depotManifest.storedPath;
      if (!result.customManifestId) {
        const nameMatch = depotManifest.originalName.match(/^(\d+)_(\d+)\.manifest$/);
        if (nameMatch) {
          result.customManifestId = nameMatch[2];
        }
      }
    }
    return result;
  });
}

function buildDownloadConfig(data, depots, mhApiKey, repairManifests) {
  const downloadConfig = {
    mainAppId: String(data.mainAppId),
    selectedDepots: depots,
    manifestHubApiKey: mhApiKey || null,
    downloadDir: getDownloadDir() || null,
    gameName: state.gameName || null,
    headerImage: state.headerImage || null,
    updateDir: activeUpdateDir(data.mainAppId),
    repair: !!repairManifests,
    allAppIds: Array.isArray(data.allAppIds) ? data.allAppIds.map(String) : [],
    includeDlc: typeof state.depotIncludeDlc === 'boolean' ? state.depotIncludeDlc : null
  };
  if (state.mode === 'search') {
    if (state.searchRepo) downloadConfig.repo = state.searchRepo;
    if (state.searchSha) downloadConfig.sha = state.searchSha;
    if (state.searchKeyVdfKeys) downloadConfig.keyVdfKeys = state.searchKeyVdfKeys;
    if (state.selectedRepo && state.selectedRepo.type) {
      downloadConfig.sourceType = state.selectedRepo.type;
    }
  }
  return downloadConfig;
}

function rememberDownloadInputs() {
  const mhApiKey = els.mhApiKey.value.trim();
  hideMhKeyRequiredHint();
  if (mhApiKey) {
    localStorage.setItem(MH_APIKEY_STORAGE_KEY, mhApiKey);
  } else {
    localStorage.removeItem(MH_APIKEY_STORAGE_KEY);
  }
  saveDownloadDir();
  return mhApiKey;
}

async function startDownload() {
  const data = state.parsedData;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));

  if (selectedDepots.length === 0) return;
  if (!(await confirmFreeSpace(selectedDepots, activeUpdateDir(data.mainAppId)))) return;

  const mhApiKey = rememberDownloadInputs();
  const repairManifests = activeRepair(data.mainAppId);
  state.repairRunning = !!repairManifests;
  const depots = collectDepotConfigs(selectedDepots, repairManifests);
  await runDownload(buildDownloadConfig(data, depots, mhApiKey, repairManifests), depots);
}

function downloadContext(downloadConfig, depots, fromQueue) {
  const keys = downloadConfig.keyVdfKeys || {};
  let selection = 'default';
  if (fromQueue) selection = 'queued';
  else if (state.depotSelectionTouched) selection = 'manual';
  else if (state.steamMode) selection = 'like_steam';
  return {
    mode: downloadConfig.repair ? 'repair' : downloadConfig.updateDir ? 'update' : 'new',
    selection,
    source: downloadConfig.sourceType || (state.mode === 'search' ? 'search' : 'upload'),
    queue: !!fromQueue,
    dlc: typeof downloadConfig.includeDlc === 'boolean' ? downloadConfig.includeDlc : null,
    keyless: countBucket(depots.filter(d => !d.depotKey && !keys[String(d.depotId)]).length),
    custom_manifest: depots.some(d => !!d.customManifestId),
  };
}

async function runDownload(downloadConfig, depots, fromQueue = false) {
  let sourceCount = null;
  try {
    const s = await invoke('get_settings');
    state.currentEngine = s.use_native_downloader !== false ? 'native' : 'ddm';
    if (Array.isArray(s.depot_sources)) sourceCount = s.depot_sources.length;
  } catch (_) {
    state.currentEngine = 'native';
  }

  state.dlNonce = telemetryNonce();
  state.dlSourceCount = sourceCount;
  state.dlHadMhKey = !!downloadConfig.manifestHubApiKey;
  state.dlContext = downloadContext(downloadConfig, depots, fromQueue);
  emitEvent('download_started', Object.assign({
    job: state.dlNonce,
    depot_count: depots.length,
    engine: state.currentEngine,
    source_count: sourceCount,
    had_mh_key: state.dlHadMhKey,
  }, state.dlContext));

  requestNotificationPermission();
  if (els.btnReportFailure) els.btnReportFailure.classList.add('hidden');

  goToStep(3);
  initProgressUI(depots);
  await commitPendingHistory();
  state.downloadStartedAt = new Date().toISOString();

  try {
    state.dlStartedAt = Date.now();
    state.dlStage = 'starting';
    state.dlDepotCount = depots.length;

    await connectProgressListener();

    const result = await invoke('start_download', { config: downloadConfig });

    if (!state.jobId) state.jobId = result.jobId;
    state.downloadDir = result.downloadDir;
  } catch (error) {
    const errorText = window.i18n.localizeError(error);
    appendTerminalLine(window.i18n.t('progress.errorLine', { message: errorText }), 'error');
    emitEvent('download_completed', Object.assign({ success: false }, jobContext(), {
      outcome: 'failed',
      depots_total: state.dlDepotCount ?? 0,
      depots_ok: 0,
      duration_bucket: durationBucket(Date.now() - (state.dlStartedAt || Date.now())),
      engine: state.currentEngine || 'native',
      err_key: window.i18n.errorKey(String(error)) || 'unmatched',
    }, classifyStartFailure(error)));
    showCompletion(false, errorText);
    queueAfterDownload('failed');
  }
}

function applyDepotFilters() {
  const searchText = (els.depotSearch ? els.depotSearch.value.trim().toLowerCase() : '');
  const showSelectedOnly = els.showSelectedOnly ? els.showSelectedOnly.checked : false;

  const items = document.querySelectorAll('.depot-item');
  items.forEach(item => {
    const depotId = item.dataset.depotId || '';
    const name = (state.depotNames && state.depotNames[depotId]) || '';
    const info = state.depotPicsInfo && state.depotPicsInfo[depotId];
    const haystack = [
      depotId,
      name,
      info && info.role,
      info && info.oslist,
      info && info.language,
    ]
      .filter(Boolean)
      .join(' ')
      .toLowerCase();
    const matchesSearch = !searchText || haystack.includes(searchText);
    const matchesSelected = !showSelectedOnly || state.selectedDepots.has(depotId);
    item.style.display = (matchesSearch && matchesSelected) ? '' : 'none';
  });
}
