function goToStep(step) {
  state.currentStep = step;
  const appRoot = document.querySelector('.app');
  if (appRoot) appRoot.classList.toggle('app--wide', step === 5);
  if (step === 5) {
    syncEmuGameDataKey();
    selectEmuTab(state.emuEditMode ? 'emulator' : 'files');
    updateEmuOverview();
  }
  if (step !== 2) cancelAutoStart();
  renderUpdateNotice(step);

  const stepMap = {
    1: els.stepUpload,
    2: els.stepSelect,
    3: els.stepProgress,
    4: els.stepShortcut,
    5: els.stepEmulator,
    6: els.stepSteamLibrary,
  };
  Object.entries(stepMap).forEach(([n, el]) => {
    if (!el) return;
    const match = parseInt(n) === step;
    el.classList.toggle('active', match);
    el.classList.toggle('hidden', !match);
  });

  const visibleItems = Array.from($$('.steps__item:not(.hidden)'));
  const currentPos = visibleItems.findIndex(el => parseInt(el.dataset.step) === step);
  $$('.steps__item').forEach((el) => {
    const s = parseInt(el.dataset.step);
    const myPos = visibleItems.findIndex(it => parseInt(it.dataset.step) === s);
    el.classList.toggle('active', s === step);
    const skippedEntry = state.emuStandalone || state.emuEditMode;
    el.classList.toggle('completed', !skippedEntry && currentPos >= 0 && myPos >= 0 && myPos < currentPos);
  });

  const stepsIndicator = document.getElementById('steps-indicator');
  if (stepsIndicator) {
    const visibleItems = $$('.steps__item:not(.hidden)');
    const max = Math.max(3, visibleItems.length);
    const positionOfStep = Array.from(visibleItems)
      .findIndex(el => parseInt(el.dataset.step) === step);
    const valueNow = positionOfStep >= 0 ? positionOfStep + 1 : Math.min(step, max);
    stepsIndicator.setAttribute('aria-valuemax', String(max));
    stepsIndicator.setAttribute('aria-valuenow', String(valueNow));
  }
}

function renumberSteps() {
  const visible = $$('.steps__item:not(.hidden)');
  visible.forEach((el, i) => {
    const numberEl = el.querySelector('.steps__number');
    if (numberEl) numberEl.textContent = String(i + 1);
  });
}

function switchTab(tabName) {
  state.mode = tabName;

  els.tabUpload.classList.toggle('active', tabName === 'upload');
  els.tabSearch.classList.toggle('active', tabName === 'search');
  els.tabPatchOnly.classList.toggle('active', tabName === 'patchOnly');

  els.tabContentUpload.classList.toggle('active', tabName === 'upload');
  els.tabContentSearch.classList.toggle('active', tabName === 'search');
  els.tabContentPatchOnly.classList.toggle('active', tabName === 'patchOnly');
}

function initUpload() {
  const dropZone = els.dropZone;

  dropZone.addEventListener('click', openFileDialog);

  dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('drag-over');
  });

  dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('drag-over');
  });

  // HTML5 drag-drop inside a WebView doesn't surface file paths; we rely on the
  // 'tauri://drag-drop' event below instead of e.dataTransfer.
  dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('drag-over');
  });

  listen('tauri://drag-drop', (event) => {
    const paths = event.payload.paths || event.payload;
    if (Array.isArray(paths) && paths.length > 0) {
      handleFilePath(paths[0]);
    }
  });

  els.fileRemove.addEventListener('click', (e) => {
    e.stopPropagation();
    resetUpload();
  });
}

async function openFileDialog() {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      filters: [{ name: window.i18n.t('upload.filterName'), extensions: ['lua', 'st'] }]
    });
    if (filePath) {
      await handleFilePath(filePath);
    }
  } catch (e) {
    console.error('File dialog error:', e);
  }
}

function resetUpload() {
  els.fileInfo.classList.add('hidden');
  els.dropZone.classList.remove('hidden');
  els.uploadError.classList.add('hidden');
  els.uploadLoading.classList.add('hidden');
  state.parsedData = null;
}

async function handleDepotManifestFile(depotId) {
  try {
    const { open } = window.__TAURI__.dialog;
    const filePath = await open({
      filters: [{ name: window.i18n.t('select.manifestFilesFilter'), extensions: ['manifest'] }]
    });
    if (!filePath) return;

    const fileName = filePath.split(/[\\/]/).pop();

    state.depotManifests[depotId] = {
      originalName: fileName,
      storedPath: filePath
    };
    emitEvent('manifest_tool', { action: 'upload', ok: true });

    const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
    const btnEl = document.querySelector(`.depot-manifest-btn[data-depot-id="${depotId}"]`);
    if (statusEl) statusEl.innerHTML = `<span class="manifest-uploaded">${ICONS.check} ${escapeHtml(fileName)}</span>`;
    if (btnEl) btnEl.classList.add('depot-manifest-action--active');
  } catch (error) {
    console.error('Failed to select manifest file:', error);
    alert(window.i18n.t('select.manifestFileError', { message: window.i18n.localizeError(error) }));
    delete state.depotManifests[depotId];
  }
}

function removeDepotManifest(depotId) {
  delete state.depotManifests[depotId];
  const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
  const btnEl = document.querySelector(`.depot-manifest-btn[data-depot-id="${depotId}"]`);
  if (statusEl) statusEl.innerHTML = '';
  if (btnEl) btnEl.classList.remove('depot-manifest-action--active');
}

async function fetchLatestManifestForDepot(depotId, btnEl) {
  const appId = state.parsedData && state.parsedData.mainAppId ? String(state.parsedData.mainAppId) : null;
  if (!appId) return;
  const input = document.querySelector(`.custom-manifest-input[data-depot-id="${depotId}"]`);
  const statusEl = document.querySelector(`.depot-manifest-status[data-depot-id="${depotId}"]`);
  if (statusEl && statusEl._fetchClearTimer) {
    clearTimeout(statusEl._fetchClearTimer);
    statusEl._fetchClearTimer = null;
  }
  if (btnEl) {
    btnEl.disabled = true;
    btnEl.classList.add('depot-manifest-action--loading');
  }
  if (statusEl) statusEl.innerHTML = `<span class="manifest-uploading">${window.i18n.t('depots.fetchingLatest')}</span>`;
  try {
    const result = await invoke('fetch_latest_manifest_id', { appId, depotId: String(depotId) });
    const manifestId = result.manifestId;
    const sourceLabel = result.source === 'steam'
      ? window.i18n.t('depots.fetchSourceSteam')
      : window.i18n.t('depots.fetchSourceFallback');
    if (input) input.value = manifestId;
    emitEvent('manifest_tool', { action: 'fetch_latest', ok: true, source: result.source === 'steam' ? 'steam' : 'fallback' });
    if (statusEl) statusEl.innerHTML = `<span class="manifest-uploaded">${ICONS.check} ${escapeHtml(manifestId)}</span> <span class="manifest-source manifest-source--${escapeHtml(result.source)}">${escapeHtml(sourceLabel)}</span>`;
  } catch (e) {
    console.error('fetch_latest_manifest_id failed:', e);
    emitEvent('manifest_tool', { action: 'fetch_latest', ok: false, key: window.i18n.errorKey(String(e)) || 'unmatched' });
    if (statusEl) statusEl.innerHTML = `<span class="status-error">${escapeHtml(window.i18n.t('depots.fetchLatestError', { message: window.i18n.localizeError(e) }))}</span>`;
  } finally {
    if (btnEl) {
      btnEl.disabled = false;
      btnEl.classList.remove('depot-manifest-action--loading');
    }
    if (statusEl) {
      statusEl._fetchClearTimer = setTimeout(() => {
        const uploaded = state.depotManifests[depotId];
        if (uploaded) {
          statusEl.innerHTML = `<span class="manifest-uploaded">${ICONS.check} ${escapeHtml(uploaded.originalName)}</span>`;
        } else {
          statusEl.innerHTML = '';
        }
        statusEl._fetchClearTimer = null;
      }, 5000);
    }
  }
}

async function handleFilePath(filePath) {
  const ext = filePath.split('.').pop().toLowerCase();
  if (ext !== 'lua' && ext !== 'st') {
    showUploadError(window.i18n.t('upload.wrongFile'));
    return;
  }

  const fileName = filePath.split(/[\\/]/).pop();

  els.dropZone.classList.add('hidden');
  els.fileInfo.classList.remove('hidden');
  els.fileName.textContent = fileName;
  els.uploadError.classList.add('hidden');
  els.uploadLoading.classList.remove('hidden');

  try {
    const raw = await invoke('parse_lua_file', { path: filePath });

    state.parsedData = {
      mainAppId: raw.main_app_id,
      depots: (raw.depots || []).map(d => ({
        depotId: String(d.depot_id),
        manifestId: d.manifest_id || 'N/A',
        depotKey: d.depot_key || null,
        sizeBytes: d.size_bytes || null
      })),
      allAppIds: Array.isArray(raw.all_app_ids) ? raw.all_app_ids.map(String) : [],
    };
    state.mode = 'upload';
    els.uploadLoading.classList.add('hidden');
    emitEvent('lua_parsed', { depot_count: state.parsedData.depots.length });

    showSelectionStep();
  } catch (error) {
    els.uploadLoading.classList.add('hidden');
    showUploadError(window.i18n.localizeError(error));
    reportError('upload', error);
  }
}

function showUploadError(message) {
  els.uploadError.textContent = message;
  els.uploadError.classList.remove('hidden');
}

let autocompleteDebounceTimer = null;

function isNumericInput(str) {
  return /^\d+$/.test(str.trim());
}

function isValidAppId(str) {
  const v = String(str || '').trim();
  if (!/^[1-9]\d{0,9}$/.test(v)) return false;
  return Number(v) <= 4294967295;
}

function hideAutocomplete(dropdown) {
  clearTimeout(autocompleteDebounceTimer);
  autocompleteDebounceTimer = null;
  dropdown.classList.add('hidden');
  dropdown.innerHTML = '';
}

function showAutocompleteLoading(dropdown) {
  dropdown.innerHTML = `<div class="search-autocomplete__loading">${escapeHtml(window.i18n.t('search.loading'))}</div>`;
  dropdown.classList.remove('hidden');
}

function renderAutocompleteResults(dropdown, results) {
  if (!results || results.length === 0) {
    dropdown.innerHTML = `<div class="search-autocomplete__empty">${escapeHtml(window.i18n.t('search.noGamesFound'))}</div>`;
    dropdown.classList.remove('hidden');
    return;
  }

  dropdown.innerHTML = results.map(item => `
    <div class="search-autocomplete__item" data-appid="${escapeHtml(String(item.appId))}">
      <img class="search-autocomplete__img" src="${escapeHtml(item.image || '')}" alt="" loading="lazy" onerror="this.style.display='none'">
      <span class="search-autocomplete__name">${escapeHtml(item.name || '')}</span>
      <span class="search-autocomplete__appid">${escapeHtml(String(item.appId))}</span>
    </div>
  `).join('');
  dropdown.classList.remove('hidden');
}

async function triggerAutocomplete(input, dropdown, query) {
  showAutocompleteLoading(dropdown);
  try {
    const results = await invoke('search_steam_games', { query });
    const currentVal = input.value.trim();
    if (currentVal === query || (!isNumericInput(currentVal) && currentVal.length >= 2)) {
      renderAutocompleteResults(dropdown, results);
    }
  } catch (err) {
    console.error('[Autocomplete]', err);
    hideAutocomplete(dropdown);
  }
}

function onAutocompleteInput(input, dropdown) {
  const val = input.value.trim();

  if (autocompleteDebounceTimer) {
    clearTimeout(autocompleteDebounceTimer);
    autocompleteDebounceTimer = null;
  }

  // If empty or numeric → no autocomplete
  if (!val || isNumericInput(val)) {
    hideAutocomplete(dropdown);
    return;
  }

  if (val.length < 2) {
    hideAutocomplete(dropdown);
    return;
  }

  // Debounce: 400ms
  autocompleteDebounceTimer = setTimeout(() => {
    triggerAutocomplete(input, dropdown, val);
  }, 400);
}

async function browsePatchOnlyDir() {
  try {
    const { open } = window.__TAURI__.dialog;
    const selected = await open({
      directory: true,
      multiple: false,
      title: window.i18n.t('patchOnly.folderDialogTitle')
    });
    if (selected) {
      els.patchOnlyDir.value = selected;
      hidePatchOnlyError();
    }
  } catch (e) {
    console.error('Failed to open folder dialog:', e);
  }
}

function showPatchOnlyError(message) {
  if (!els.patchOnlyError) return;
  els.patchOnlyError.textContent = message;
  els.patchOnlyError.classList.remove('hidden');
}

function hidePatchOnlyError() {
  if (els.patchOnlyError) els.patchOnlyError.classList.add('hidden');
}

function resetPatchOnlyTab() {
  if (els.patchOnlyDir) els.patchOnlyDir.value = '';
  if (els.patchOnlyAppIdInput) els.patchOnlyAppIdInput.value = '';
  if (els.patchOnlyAutocomplete) hideAutocomplete(els.patchOnlyAutocomplete);
  if (els.patchOnlyLoading) els.patchOnlyLoading.classList.add('hidden');
  if (els.btnPatchOnlyStart) els.btnPatchOnlyStart.disabled = false;
  hidePatchOnlyError();
}

async function startPatchOnlyEmulator() {
  hideAutocomplete(els.patchOnlyAutocomplete);
  hidePatchOnlyError();

  const gameDir = (els.patchOnlyDir.value || '').trim();
  const appId = (els.patchOnlyAppIdInput.value || '').trim();
  if (!gameDir) {
    showPatchOnlyError(window.i18n.t('patchOnly.errorNoFolder'));
    return;
  }
  if (!isValidAppId(appId)) {
    showPatchOnlyError(window.i18n.t('patchOnly.errorAppId'));
    return;
  }

  els.btnPatchOnlyStart.disabled = true;
  els.patchOnlyLoading.classList.remove('hidden');
  let scanned = [];
  try {
    scanned = await invoke('emu_scan_game_dir', { gameDir });
  } catch (e) {
    console.error('emu_scan_game_dir failed:', e);
    showPatchOnlyError(window.i18n.t('patchOnly.errorScan', { message: window.i18n.localizeError(e) }));
    return;
  } finally {
    els.patchOnlyLoading.classList.add('hidden');
    els.btnPatchOnlyStart.disabled = false;
  }

  if (!Array.isArray(scanned) || scanned.length === 0) {
    showPatchOnlyError(window.i18n.t('patchOnly.errorNoApiFiles'));
    return;
  }

  const unpatched = scanned.filter(f => !f.is_patched);
  state.emuSelectionExplicit = true;

  state.emuStandalone = true;
  state.downloadDir = gameDir;
  state.parsedData = { mainAppId: appId, depots: [], allAppIds: [] };
  state.emulatorScan = scanned;
  state.emuSelectedFiles = new Set(unpatched.map(f => f.path));
  state.emulatorAvailable = true;
  await goToEmulatorStep({ prefillSettings: unpatched.length === scanned.length });
  refreshEmuPatchedState(scanned);
  await hydrateEmuPatchedContext();
}
