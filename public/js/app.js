const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const state = {
  currentStep: 1,
  updateDir: null,
  updateAppId: null,
  mode: 'upload', // 'upload' or 'search'
  parsedData: null,
  selectedDepots: new Set(),
  jobId: null,
  unlistenProgress: null,
  gameName: null,
  headerImage: null,
  downloadDir: null,
  shortcutSupported: false,
  shortcutsCreated: false,
  downloadFailed: false,
  emulatorAvailable: false,
  emulatorScan: [],
  emulatorReleaseInfo: null,
  drmTargets: [],
  emuEditMode: false,
  emuStandalone: false,
  emuEditTargets: [],
  emuPatchedPaths: new Set(),
  emuSettingsPrefillPath: null,
  emuBusy: false,
  emuApplyComplete: false,
  emuAllowDownload: false,
  emuPendingDownload: null,
  bypassInitialState: false,
  pendingHistoryRemoveId: null,
  steamLibrarySupported: false,
  steamLibraryUser: null,
  steamLibraryDetectedExes: [],
  downloadStartedAt: null,
  pendingHistoryEntry: null,
  notificationsEnabled: false,
  notificationSoundEnabled: true,
  depotManifests: {}, // depotId -> { originalName, storedPath }
  searchRepos: [],
  selectedRepo: null,
  searchAppId: null,
  searchRepo: null,
  searchSha: null,
  searchKeyVdfKeys: null,
  speedTracker: {
    lastPercent: 0,
    lastTime: 0,
    samples: [],
    currentDepotSize: 0,
    currentDepotId: null,
    depotStartTime: 0,
    staleTimer: null,
    lastUpdateTime: 0,
  },
};

const SVG_BASE = 'viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"';
const ICONS = {
  upload: `<svg class="btn-icon" ${SVG_BASE}><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`,
  folderOpen: `<svg class="btn-icon" ${SVG_BASE}><path d="M6 14l1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"/></svg>`,
  refresh: `<svg class="btn-icon" ${SVG_BASE}><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>`,
  update: `<svg class="btn-icon" ${SVG_BASE}><circle cx="12" cy="12" r="10"/><polyline points="16 12 12 8 8 12"/><line x1="12" y1="16" x2="12" y2="8"/></svg>`,
  trash: `<svg class="btn-icon" ${SVG_BASE}><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/><path d="M9 6V4a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2"/></svg>`,
  x: `<svg class="btn-icon" ${SVG_BASE}><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>`,
  check: `<svg class="btn-icon" ${SVG_BASE}><polyline points="20 6 9 17 4 12"/></svg>`,
  checkCircle: `<svg class="btn-icon" ${SVG_BASE}><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>`,
  alertTriangle: `<svg class="btn-icon" ${SVG_BASE}><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>`,
  sun: `<svg class="theme-icon theme-icon--sun" id="theme-icon" width="16" height="16" ${SVG_BASE}><circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/></svg>`,
  moon: `<svg class="theme-icon theme-icon--moon" id="theme-icon" width="16" height="16" ${SVG_BASE}><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`,
  settings: `<svg class="btn-icon" ${SVG_BASE}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>`,
  play: `<svg class="btn-icon" ${SVG_BASE}><polygon points="5 3 19 12 5 21 5 3"/></svg>`,
};

const MH_APIKEY_STORAGE_KEY = 'manifestHubApiKey';
let autoRedownloadPending = false;
let autoSelectAllOnStep2 = false;
let autoSelectDepotIds = null; // specific depot IDs to re-select, or null for all
let defaultDownloadDir = '';

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const els = {
  stepUpload: $('#step-upload'),
  stepSelect: $('#step-select'),
  stepProgress: $('#step-progress'),
  tabUpload: $('#tab-upload'),
  tabSearch: $('#tab-search'),
  tabPatchOnly: $('#tab-patch-only'),
  tabContentUpload: $('#tab-content-upload'),
  tabContentSearch: $('#tab-content-search'),
  tabContentPatchOnly: $('#tab-content-patch-only'),
  dropZone: $('#drop-zone'),
  fileInfo: $('#file-info'),
  fileName: $('#file-name'),
  fileRemove: $('#file-remove'),
  uploadError: $('#upload-error'),
  uploadLoading: $('#upload-loading'),
  searchAppIdInput: $('#search-appid-input'),
  searchAutocomplete: $('#search-autocomplete'),
  btnSearch: $('#btn-search'),
  searchInputBlock: $('#search-input-block'),
  sourcesEmpty: $('#sources-empty-state'),
  sourcesEmptyInput: $('#sources-empty-input'),
  btnSourcesEmptyAdd: $('#btn-sources-empty-add'),
  sourcesEmptyError: $('#sources-empty-error'),
  sourcesList: $('#sources-list'),
  sourcesAddInput: $('#sources-add-input'),
  btnSourcesAdd: $('#btn-sources-add'),
  sourcesAddError: $('#sources-add-error'),
  searchError: $('#search-error'),
  searchLoading: $('#search-loading'),
  searchResults: $('#search-results'),
  repoList: $('#repo-list'),
  searchNextRow: $('#search-next-row'),
  btnSearchNext: $('#btn-search-next'),
  manifestLoading: $('#manifest-loading'),
  searchGameBanner: $('#search-game-banner'),
  searchGameImage: $('#search-game-image'),
  searchGameName: $('#search-game-name'),
  searchGameDescription: $('#search-game-description'),
  patchOnlyDir: $('#patch-only-dir'),
  btnPatchOnlyBrowse: $('#btn-patch-only-browse'),
  patchOnlyAppIdInput: $('#patch-only-appid'),
  patchOnlyAutocomplete: $('#patch-only-autocomplete'),
  patchOnlyError: $('#patch-only-error'),
  patchOnlyLoading: $('#patch-only-loading'),
  btnPatchOnlyStart: $('#btn-patch-only-start'),
  appIdDisplay: $('#app-id-display'),
  depotCount: $('#depot-count'),
  depotList: $('#depot-list'),
  btnSelectAll: $('#btn-select-all'),
  btnDeselectAll: $('#btn-deselect-all'),
  btnBack: $('#btn-back'),
  btnDownload: $('#btn-download'),
  depotProgressFill: $('#depot-progress-fill'),
  depotProgressText: $('#depot-progress-text'),
  downloadSpeedInfo: $('#download-speed-info'),
  downloadSpeed: $('#download-speed'),
  downloadEta: $('#download-eta'),
  progressHeader: $('#progress-header'),
  progressBarFill: $('#progress-bar-fill'),
  progressStatus: $('#progress-status'),
  depotProgressList: $('#depot-progress-list'),
  terminalOutput: $('#terminal-output'),
  completionMessage: $('#completion-message'),
  btnCancel: $('#btn-cancel'),
  btnPause: $('#btn-pause'),
  nativeAlphaBanner: $('#native-alpha-banner'),
  mhApiKey: $('#mh-apikey'),
  downloadDirInput: $('#download-dir'),
  btnBrowseDir: $('#btn-browse-dir'),
  diskSpaceInfo: $('#disk-space-info'),
  diskSpaceText: $('#disk-space-text'),
  gameInfoBanner: $('#game-info-banner'),
  gameInfoLoading: $('#game-info-loading'),
  gameHeaderImage: $('#game-header-image'),
  gameName: $('#game-name'),
  gameDescription: $('#game-description'),
  cancelModal: $('#cancel-modal'),
  shutdownAfterToggle: $('#shutdown-after-toggle'),
  shutdownModal: $('#shutdown-modal'),
  shutdownModalBody: $('#shutdown-modal-body'),
  btnShutdownAbort: $('#btn-shutdown-abort'),
  btnShutdownNow: $('#btn-shutdown-now'),
  followupModal: $('#followup-modal'),
  followupModalBody: $('#followup-modal-body'),
  btnFollowupLater: $('#btn-followup-later'),
  btnFollowupDiscard: $('#btn-followup-discard'),
  btnFollowupResume: $('#btn-followup-resume'),
  btnCancelYes: $('#btn-cancel-yes'),
  btnCancelNo: $('#btn-cancel-no'),
  btnThemeToggle: $('#btn-theme-toggle'),
  depotSearch: $('#depotSearch'),
  showSelectedOnly: $('#showSelectedOnly'),
  btnSettings: $('#btn-settings'),
  settingsModal: $('#settings-modal'),
  btnSettingsSave: $('#btn-settings-save'),
  btnSettingsCancel: $('#btn-settings-cancel'),
  autoUpdateToggle: $('#auto-update-toggle'),
  btnToggleAdvanced: $('#btn-toggle-advanced'),
  advancedSettingsContent: $('#advanced-settings-content'),
  ddExtraArgsInput: $('#dd-extra-args-input'),
  maxRetriesInput: $('#max-retries-input'),
  chunkConcurrencyInput: $('#chunk-concurrency-input'),
  speedLimitToggle: $('#speed-limit-toggle'),
  speedLimitControls: $('#speed-limit-controls'),
  speedLimitInput: $('#speed-limit-input'),
  proxyInput: $('#proxy-input'),
  hubcapApiKeyInput: $('#hubcap-apikey-input'),
  ryuuApiKeyInput: $('#ryuu-apikey-input'),
  notificationSoundToggle: $('#notification-sound-toggle'),
  nativeDownloaderToggle: $('#native-downloader-toggle'),
  cancelKeepFilesToggle: $('#cancel-keep-files-toggle'),
  telemetryModal: $('#telemetry-modal'),
  btnTelemetryAccept: $('#btn-telemetry-accept'),
  btnTelemetryDecline: $('#btn-telemetry-decline'),
  telemetryToggle: $('#telemetry-toggle'),
  buildInfoChannel: $('#build-info-channel'),
  buildInfoVersion: $('#build-info-version'),
  buildInfoSha: $('#build-info-sha'),
  buildInfoDate: $('#build-info-date'),
  buildInfoProfile: $('#build-info-profile'),
  buildInfoPlatform: $('#build-info-platform'),
  btnCopyBuildInfo: $('#btn-copy-build-info'),
  btnHistory: $('#btn-history'),
  historyModal: $('#history-modal'),
  historyList: $('#history-list'),
  historyToolbar: $('#history-toolbar'),
  historySearch: $('#history-search'),
  historySort: $('#history-sort'),
  btnHistoryClear: $('#btn-history-clear'),
  btnHistoryClose: $('#btn-history-close'),
  updateModal: $('#update-modal'),
  updateVersion: $('#update-version'),
  updateDate: $('#update-date'),
  updateDateRow: $('#update-date-row'),
  updateNotes: $('#update-notes'),
  updateProgressWrap: $('#update-progress-wrap'),
  updateProgressFill: $('#update-progress-fill'),
  updateProgressText: $('#update-progress-text'),
  updateActions: $('#update-actions'),
  btnUpdateNow: $('#btn-update-now'),
  btnUpdateLater: $('#btn-update-later'),
  btnUpdateSkip: $('#btn-update-skip'),
  stepShortcut: $('#step-shortcut'),
  step4Connector: $('#step4-connector'),
  step4Indicator: $('#step4-indicator'),
  btnNextStep: $('#btn-next-step'),
  shortcutExePath: $('#shortcut-exe-path'),
  btnBrowseExe: $('#btn-browse-exe'),
  shortcutDetectedSection: $('#shortcut-detected-section'),
  btnToggleDetected: $('#btn-toggle-detected'),
  shortcutDetectedList: $('#shortcut-detected-list'),
  shortcutDesktop: $('#shortcut-desktop'),
  shortcutStartMenu: $('#shortcut-startmenu'),
  shortcutStatus: $('#shortcut-status'),
  btnCreateShortcuts: $('#btn-create-shortcuts'),
  btnShortcutSkip: $('#btn-shortcut-skip'),
  btnShortcutStartOver: $('#btn-shortcut-start-over'),
  stepEmulator: $('#step-emulator'),
  step5Connector: $('#step5-connector'),
  step5Indicator: $('#step5-indicator'),
  emuReleaseStatus: $('#emu-release-status'),
  emuFileList: $('#emu-file-list'),
  emuFileEmpty: $('#emu-file-empty'),
  emuAlreadyPatched: $('#emu-already-patched'),
  emuSettingsHint: $('#emu-settings-hint'),
  emuApplyStatus: $('#emu-apply-status'),
  btnEmuApply: $('#btn-emu-apply'),
  btnEmuSaveSettings: $('#btn-emu-save-settings'),
  btnEmuNew: $('#btn-emu-new'),
  btnEmuStartOver: $('#btn-emu-start-over'),
  emuDrmSection: $('#emu-drm-section'),
  emuDrmList: $('#emu-drm-list'),
  emuDrmStatusWrap: $('#emu-drm-status-wrap'),
  emuDrmStatus: $('#emu-drm-status'),
  btnEmuDrmRemove: $('#btn-emu-drm-remove'),
  btnEmuDrmCopy: $('#btn-emu-drm-copy'),
  emuBypassSection: $('#emu-bypass-section'),
  emuBypassToggle: $('#emu-bypass-toggle'),
  emuDlcMergeSection: $('#emu-dlc-merge-section'),
  emuDlcMergeHint: $('#emu-dlc-merge-hint'),
  emuDlcMergeStatus: $('#emu-dlc-merge-status'),
  btnEmuMergeDlcs: $('#btn-emu-merge-dlcs'),
  emuHeader: $('.emu-header'),
  emuDescription: $('.emu-description'),
  emuVariantSection: $('#emu-variant-section'),
  btnEmuRevert: $('#btn-emu-revert'),
  emuRevertModal: $('#emu-revert-modal'),
  emuRevertScope: $('#emu-revert-scope'),
  btnEmuRevertYes: $('#btn-emu-revert-yes'),
  btnEmuRevertNo: $('#btn-emu-revert-no'),
  emuDownloadModal: $('#emu-download-modal'),
  emuDownloadScope: $('#emu-download-scope'),
  btnEmuDownloadYes: $('#btn-emu-download-yes'),
  btnEmuDownloadNo: $('#btn-emu-download-no'),
  historyRemoveModal: $('#history-remove-modal'),
  btnHistoryRemoveYes: $('#btn-history-remove-yes'),
  btnHistoryRemoveNo: $('#btn-history-remove-no'),
  historyClearModal: $('#history-clear-modal'),
  btnHistoryClearYes: $('#btn-history-clear-yes'),
  btnHistoryClearNo: $('#btn-history-clear-no'),
  stepSteamLibrary: $('#step-steam-library'),
  step6Connector: $('#step6-connector'),
  step6Indicator: $('#step6-indicator'),
  steamLibraryStatus: $('#steam-library-status'),
  steamExePath: $('#steam-exe-path'),
  btnSteamBrowseExe: $('#btn-steam-browse-exe'),
  steamDetectedSection: $('#steam-detected-section'),
  btnSteamToggleDetected: $('#btn-steam-toggle-detected'),
  steamDetectedList: $('#steam-detected-list'),
  steamGameName: $('#steam-game-name'),
  steamLaunchOptions: $('#steam-launch-options'),
  steamLibraryResult: $('#steam-library-result'),
  btnSteamAdd: $('#btn-steam-add'),
  btnSteamSkip: $('#btn-steam-skip'),
  shortcutSteamRow: $('#shortcut-steam-row'),
  shortcutSteamLibrary: $('#shortcut-steam-library'),
};

function goToStep(step) {
  state.currentStep = step;
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
      filters: [{ name: 'Lua/ST Files', extensions: ['lua', 'st'] }]
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
    if (statusEl) statusEl.innerHTML = `<span class="manifest-uploaded">${ICONS.check} ${escapeHtml(manifestId)}</span> <span class="manifest-source manifest-source--${escapeHtml(result.source)}">${escapeHtml(sourceLabel)}</span>`;
  } catch (e) {
    console.error('fetch_latest_manifest_id failed:', e);
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

async function loadDepotSources() {
  try {
    const settings = await invoke('get_settings');
    return Array.isArray(settings.depot_sources) ? settings.depot_sources : [];
  } catch {
    return [];
  }
}

async function saveDepotSources(sources) {
  const settings = await invoke('get_settings');
  settings.depot_sources = sources;
  await invoke('save_settings', { settings });
}

function validateSourceUrl(raw) {
  const url = (raw || '').trim();
  if (!url) return i18n.t('errors.sourceEmpty');
  if (!/^https?:\/\//i.test(url)) return i18n.t('errors.sourceProtocol');
  return null;
}

async function refreshSourcesUI() {
  const sources = await loadDepotSources();
  const empty = sources.length === 0;
  if (els.sourcesEmpty) els.sourcesEmpty.classList.toggle('hidden', !empty);
  if (els.searchInputBlock) els.searchInputBlock.classList.toggle('hidden', empty);
  if (els.sourcesList) {
    const removeLabel = escapeHtml(i18n.t('settings.removeSource'));
    els.sourcesList.innerHTML = sources
      .map((s, i) => {
        const safe = escapeHtml(s);
        return `<li><span title="${safe}">${safe}</span><button data-source-idx="${i}">${removeLabel}</button></li>`;
      })
      .join('');
  }
}

async function addDepotSource(rawUrl, errorEl) {
  if (errorEl) errorEl.classList.add('hidden');
  const err = validateSourceUrl(rawUrl);
  if (err) {
    if (errorEl) {
      errorEl.textContent = err;
      errorEl.classList.remove('hidden');
    }
    return false;
  }
  const url = rawUrl.trim();
  const sources = await loadDepotSources();
  if (sources.includes(url)) {
    if (errorEl) {
      errorEl.textContent = window.i18n.t('search.sourceExists');
      errorEl.classList.remove('hidden');
    }
    return false;
  }
  sources.push(url);
  await saveDepotSources(sources);
  await refreshSourcesUI();
  return true;
}

async function removeDepotSource(index) {
  const settings = await invoke('get_settings');
  const sources = Array.isArray(settings.depot_sources) ? settings.depot_sources : [];
  if (index < 0 || index >= sources.length) return;
  const target = sources[index];
  const pristine = Array.isArray(settings.pristine_default_sources) ? settings.pristine_default_sources : [];

  if (pristine.includes(target)) {
    const confirmed = await confirmRemoveDefaultSource();
    if (!confirmed) return;
  }

  sources.splice(index, 1);
  await saveDepotSources(sources);
  await refreshSourcesUI();
}

function confirmRemoveDefaultSource() {
  const modal = document.getElementById('remove-source-modal');
  const yes = document.getElementById('btn-remove-source-yes');
  const no = document.getElementById('btn-remove-source-no');
  if (!modal || !yes || !no) return Promise.resolve(true);

  return new Promise((resolve) => {
    const cleanup = () => {
      yes.removeEventListener('click', onYes);
      no.removeEventListener('click', onNo);
      modal.classList.add('hidden');
    };
    const onYes = () => { cleanup(); resolve(true); };
    const onNo = () => { cleanup(); resolve(false); };
    yes.addEventListener('click', onYes);
    no.addEventListener('click', onNo);
    modal.classList.remove('hidden');
  });
}

async function performSearch() {
  hideAutocomplete(els.searchAutocomplete);
  const appIdStr = els.searchAppIdInput.value.trim();
  if (!appIdStr) return;

  const appId = parseInt(appIdStr, 10);
  if (isNaN(appId) || appId <= 0) {
    showSearchError(window.i18n.t('search.invalidAppId'));
    return;
  }

  els.searchError.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  state.selectedRepo = null;
  state.searchRepos = [];
  state.searchAppId = appId;

  els.searchLoading.classList.remove('hidden');
  els.btnSearch.disabled = true;

  fetchSearchGameInfo(appId);

  try {
    const raw = await invoke('search_repos', {
      appId: String(appId),
    });

    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;

    const repos = (raw.repos || []).map(r => ({
      name: r.repo,
      date: r.date,
      sha: r.sha,
      type: r.type || 'unknown',
      source: r.source || r.type || 'unknown'
    }));
    state.searchRepos = repos;
    emitSearchOutcome(repos.length, raw.sourceProbe);

    if (repos.length === 0) {
      showSearchError(window.i18n.t('search.noResults'));
      return;
    }

    renderRepoList(repos);
    els.searchResults.classList.remove('hidden');
  } catch (error) {
    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;
    emitSearchOutcome(0, 'error');
    showSearchError(window.i18n.localizeError(error));
  }
}

function emitSearchOutcome(repoCount, probe) {
  const parts = String(probe || '').split(':');
  emitEvent('search_performed', {
    found: repoCount > 0,
    repo_count: repoCount,
    source_probe: parts[0] || null,
    probe_class: parts[1] || null,
  });
}

function showSearchError(message) {
  els.searchError.textContent = message;
  els.searchError.classList.remove('hidden');
}

async function fetchSearchGameInfo(appId) {
  els.searchGameBanner.classList.add('hidden');

  try {
    const info = await invoke('get_steam_app_info', { appId: String(appId) });

    if (info) {
      const { name, headerImage, shortDescription } = info;

      if (headerImage) {
        els.searchGameImage.src = headerImage;
        els.searchGameImage.alt = name || window.i18n.t('common.gameCover');
        state.headerImage = headerImage;
      }

      if (name) {
        els.searchGameName.textContent = name;
        state.gameName = name;
      }

      if (shortDescription) {
        els.searchGameDescription.textContent = shortDescription;
      }

      els.searchGameBanner.classList.remove('hidden');
    }
  } catch (e) {
  }
}

function renderRepoList(repos) {
  els.repoList.innerHTML = '';

  repos.forEach((repo, index) => {
    const card = document.createElement('div');
    card.className = 'repo-card';
    card.dataset.repoIndex = index;

    const displayName = repoDisplayName(repo);
    const badgeText = repoBadgeText(repo);
    const dateHtml = repo.date
      ? `<div class="repo-card__date">${window.i18n.t('search.repoUpdated')}: ${formatRepoDate(repo.date)}</div>`
      : '';

    card.innerHTML = `
      <div class="repo-card__radio"></div>
      <div class="repo-card__info">
        <div class="repo-card__name">${escapeHtml(displayName)}</div>
        ${dateHtml}
      </div>
      <span class="repo-card__badge repo-card__badge--archive">${escapeHtml(badgeText)}</span>
    `;
    card.addEventListener('click', () => selectRepo(index));
    els.repoList.appendChild(card);
  });

  if (repos.length === 1) {
    selectRepo(0);
  }

  if (autoRedownloadPending && repos.length > 0) {
    autoRedownloadPending = false;
    selectRepo(0);
    proceedFromSearch();
  }
}

function repoDisplayName(repo) {
  switch (repo.type) {
    case 'hubcap':
      return window.i18n.t('search.repoNameHubcap');
    case 'remote':
      return window.i18n.t('search.repoNameRemote');
    default:
      return repo.name || repo.type || 'Unknown';
  }
}

function repoBadgeText(repo) {
  switch (repo.type) {
    case 'hubcap':
      return window.i18n.t('search.repoBadgeHubcap');
    case 'remote':
      return window.i18n.t('search.repoBadgeRemote');
    default:
      return repo.source || repo.type || window.i18n.t('search.sourceFallback');
  }
}

function formatRepoDate(dateStr) {
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleString(window.i18n.getCurrentLocale(), { dateStyle: 'medium', timeStyle: 'short' });
  } catch {
    return dateStr;
  }
}

function escapeHtml(str) {
  return String(str ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function selectRepo(index) {
  $$('.repo-card').forEach(c => c.classList.remove('selected'));

  state.selectedRepo = state.searchRepos[index];

  const card = els.repoList.querySelector(`[data-repo-index="${index}"]`);
  if (card) card.classList.add('selected');

  els.searchNextRow.classList.remove('hidden');
}

async function proceedFromSearch() {
  if (!state.selectedRepo) return;

  const repo = state.selectedRepo;
  const appId = state.searchAppId;

  els.searchNextRow.classList.add('hidden');
  els.manifestLoading.classList.remove('hidden');
  els.searchError.classList.add('hidden');

  try {
    const mRaw = await invoke('get_repo_manifests', {
      appId: String(appId),
      repo: repo.name,
      sha: repo.sha || null,
    });

    const depots = (mRaw.manifests || []).map(m => ({
      depotId: String(m.depot_id),
      manifestId: m.manifest_id || 'N/A',
      depotKey: m.depot_key || null,
      sizeBytes: m.size_bytes || null
    }));

    state.searchRepo = repo.name;
    state.searchSha = repo.sha;
    state.searchKeyVdfKeys = mRaw.depot_keys || null;

    els.manifestLoading.classList.add('hidden');

    if (depots.length === 0) {
      showSearchError(window.i18n.t('errors.noManifests'));
      els.searchNextRow.classList.remove('hidden');
      return;
    }

    state.parsedData = {
      mainAppId: appId,
      depots: depots
    };

    showSelectionStep();
  } catch (error) {
    els.manifestLoading.classList.add('hidden');
    showSearchError(window.i18n.localizeError(error));
    els.searchNextRow.classList.remove('hidden');
  }
}

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

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return null;
  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(2)} GB`;
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(2)} MB`;
  const kb = bytes / 1024;
  return `${kb.toFixed(2)} KB`;
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

  state.depotManifests = {};

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
        <div class="depot-item__manifest-id">Manifest: ${safeManifestId}</div>
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
  } catch (e) {
    console.warn('fetch_depot_metadata_steam failed:', e);
  }
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
        ? window.i18n.t('depots.roleLanguageWithName', { name: capitalize(d.language) })
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
  if (info.language) tags.push(['lang', capitalize(info.language)]);

  container.innerHTML = tags.map(([cls, label]) =>
    `<span class="depot-tag depot-tag--${cls}">${escapeHtml(label)}</span>`
  ).join('');
}

function capitalize(s) {
  if (!s) return '';
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function toggleDepot(depotId, element) {
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

async function startDownload() {
  const data = state.parsedData;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));

  if (selectedDepots.length === 0) return;
  if (!(await confirmFreeSpace(selectedDepots, activeUpdateDir(data.mainAppId)))) return;

  let sourceCount = null;
  try {
    const s = await invoke('get_settings');
    state.currentEngine = s.use_native_downloader !== false ? 'native' : 'ddm';
    if (Array.isArray(s.depot_sources)) sourceCount = s.depot_sources.length;
  } catch (_) {
    state.currentEngine = 'native';
  }

  const mhApiKey = els.mhApiKey.value.trim();
  hideMhKeyRequiredHint();

  state.dlNonce = telemetryNonce();
  state.dlSourceCount = sourceCount;
  state.dlHadMhKey = !!mhApiKey;
  emitEvent('download_started', {
    job: state.dlNonce,
    depot_count: selectedDepots.length,
    engine: state.currentEngine,
    source_count: sourceCount,
    had_mh_key: state.dlHadMhKey,
  });

  requestNotificationPermission();

  if (mhApiKey) {
    localStorage.setItem(MH_APIKEY_STORAGE_KEY, mhApiKey);
  } else {
    localStorage.removeItem(MH_APIKEY_STORAGE_KEY);
  }
  saveDownloadDir();

  const depotsWithCustomManifests = selectedDepots.map(depot => {
    const input = document.querySelector(`.custom-manifest-input[data-depot-id="${depot.depotId}"]`);
    const customManifestId = input ? input.value.trim() : '';
    const depotManifest = state.depotManifests[depot.depotId];
    const displayName = state.depotNames ? state.depotNames[String(depot.depotId)] : null;
    const result = {
      ...depot,
      customManifestId: customManifestId || null,
      displayName: displayName || null,
    };
    if (depotManifest) {
      result.uploadedManifestPath = depotManifest.storedPath;
      // If no custom manifest ID typed, try to extract from filename
      if (!result.customManifestId) {
        const nameMatch = depotManifest.originalName.match(/^(\d+)_(\d+)\.manifest$/);
        if (nameMatch) {
          result.customManifestId = nameMatch[2];
        }
      }
    }
    return result;
  });

  goToStep(3);
  initProgressUI(depotsWithCustomManifests);
  await commitPendingHistory();
  state.downloadStartedAt = new Date().toISOString();

  try {
    const downloadConfig = {
      mainAppId: String(data.mainAppId),
      selectedDepots: depotsWithCustomManifests,
      manifestHubApiKey: mhApiKey || null,
      downloadDir: getDownloadDir() || null,
      gameName: state.gameName || null,
      headerImage: state.headerImage || null,
      updateDir: activeUpdateDir(data.mainAppId)
    };

    if (state.mode === 'search') {
      if (state.searchRepo) downloadConfig.repo = state.searchRepo;
      if (state.searchSha) downloadConfig.sha = state.searchSha;
      if (state.searchKeyVdfKeys) downloadConfig.keyVdfKeys = state.searchKeyVdfKeys;
      if (state.selectedRepo && state.selectedRepo.type) {
        downloadConfig.sourceType = state.selectedRepo.type;
      }
    }

    state.dlStartedAt = Date.now();
    state.dlStage = 'starting';
    state.dlDepotCount = selectedDepots.length;

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
    }, classifyStartFailure(error)));
    showCompletion(false, errorText);
  }
}

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
    case 'depot_up_to_date':
    case 'removed_stale_files':
      if (msg.message) appendTerminalLine(eventText(msg), msg.step === 'manifest_hub_rate_limited' ? 'warn' : 'info');
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

function updateDepotDownloadProgressBytes(percent, completedBytes, totalBytes, networkBytes) {
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
    emitEvent('download_completed', Object.assign({ success: false }, jobContext(), msg.diag || {}));
    showCompletion(false, errorText);
    showBrowserNotification(window.i18n.t('notifications.failedTitle'), window.i18n.t('notifications.failedBody', { message: errorText }));
    playNotificationSound();
    cleanupProgressListener();
    maybeScheduleShutdown();
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
  state.jobId = null;
  els.completionMessage.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  els.completionMessage.classList.add(success ? 'completion-message--success' : 'completion-message--error');
  els.completionMessage.textContent = message;
  els.btnCancel.classList.add('hidden');
  if (els.btnPause) els.btnPause.classList.add('hidden');
  state.downloadFailed = !success;
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

function renderUpdateNotice(step) {
  const notice = document.getElementById('update-mode-notice');
  if (!notice) return;
  const appId = state.parsedData && state.parsedData.mainAppId;
  if (step === 2 && activeUpdateDir(appId)) {
    notice.innerHTML = window.i18n.t('select.updateNotice', { path: escapeHtml(state.updateDir) });
    notice.classList.remove('hidden');
  } else {
    notice.classList.add('hidden');
  }
}

function resetApp() {
  commitPendingHistory();
  state.updateDir = null;
  state.updateAppId = null;
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
}

async function openSettings() {
  try {
    const settings = await invoke('get_settings');
    els.autoUpdateToggle.checked = settings.auto_update !== false;

    els.ddExtraArgsInput.value = (settings.dd_extra_args || []).join(' ');
    els.maxRetriesInput.value = settings.max_retries ?? 3;
    els.chunkConcurrencyInput.value = settings.native_chunk_concurrency ?? 8;
    const speedLimit = splitSpeedLimit(settings.download_speed_limit);
    els.speedLimitInput.value = speedLimit.value;
    setSpeedLimitUnit(speedLimit.unit);
    setSpeedLimitEnabled(speedLimit.value !== '', false);
    els.proxyInput.value = settings.proxy || '';
    if (els.hubcapApiKeyInput) els.hubcapApiKeyInput.value = settings.hubcap_api_key || '';
    if (els.ryuuApiKeyInput) els.ryuuApiKeyInput.value = settings.ryuu_api_key || '';
    if (els.nativeDownloaderToggle) els.nativeDownloaderToggle.checked = !!settings.use_native_downloader;
    if (els.cancelKeepFilesToggle) els.cancelKeepFilesToggle.checked = !!settings.cancel_keep_files;
    els.notificationSoundToggle.checked = settings.notification_sound !== false;
    els.telemetryToggle.checked = settings.telemetry_consent === 'accepted';
  } catch (e) {
    els.autoUpdateToggle.checked = true;
  }
  loadBuildInfo();
  refreshSourcesUI();
  emitEvent('settings_opened');
  els.settingsModal.classList.remove('hidden');
}

function channelLabel(channel) {
  switch (channel) {
    case 'stable': return { text: window.i18n.t('settings.channelStable'), cls: 'build-info__badge--stable' };
    case 'dev':    return { text: window.i18n.t('settings.channelDev'), cls: 'build-info__badge--dev' };
    case 'dev-local': return { text: window.i18n.t('settings.channelDevLocal'), cls: 'build-info__badge--local' };
    default:       return { text: channel || '—', cls: 'build-info__badge--local' };
  }
}

async function loadBuildInfo() {
  try {
    const info = await invoke('get_build_info');
    state.buildInfo = info;

    const { text, cls } = channelLabel(info.channel);
    els.buildInfoChannel.textContent = text;
    els.buildInfoChannel.className = `build-info__badge ${cls}`;

    els.buildInfoVersion.textContent = info.version || '—';
    els.buildInfoSha.textContent = info.gitSha || window.i18n.t('common.unknown');
    els.buildInfoDate.textContent = info.buildDate || window.i18n.t('common.unknown');
    els.buildInfoProfile.textContent = info.profile || '—';
    els.buildInfoPlatform.textContent = `${info.targetOs || '?'} / ${info.targetArch || '?'}`;
  } catch (e) {
    console.error('Failed to load build info:', e);
  }
}

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
      invoke('emit_telemetry_event', { kind: 'app_start' }).catch(() => {});
    }
  } catch (e) {
    console.error('Failed to load telemetry status:', e);
  }
}

async function acceptTelemetry() {
  try {
    await invoke('set_telemetry_consent', { accept: true });
    invoke('emit_telemetry_event', { kind: 'app_start' }).catch(() => {});
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

async function onTelemetryToggleChanged() {
  const accept = els.telemetryToggle.checked;
  try {
    await invoke('set_telemetry_consent', { accept });
  } catch (e) {
    console.error('Failed to update telemetry consent:', e);
    els.telemetryToggle.checked = !accept;
  }
}

function emitEvent(kind, props) {
  invoke('emit_telemetry_event', { kind, props: props ?? null }).catch(() => {});
}

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
  return {
    job: state.dlNonce || null,
    source_count: state.dlSourceCount ?? null,
    had_mh_key: !!state.dlHadMhKey,
  };
}

function clearJobTelemetryState() {
  state.dlNonce = null;
  state.dlStartedAt = null;
  state.dlStage = null;
  state.dlDepotCount = null;
  state.dlSourceCount = null;
  state.dlHadMhKey = false;
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
  ].join('\n');

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

function closeSettings() {
  els.settingsModal.classList.add('hidden');
}

async function saveSettings() {
  const autoUpdate = els.autoUpdateToggle.checked;
  try {
    const currentSettings = await invoke('get_settings');
    currentSettings.auto_update = autoUpdate;

    const argsStr = els.ddExtraArgsInput.value.trim();
    if (argsStr) {
      currentSettings.dd_extra_args = argsStr.split(/\s+/).filter(a => a.length > 0);
    } else {
      currentSettings.dd_extra_args = ["-max-downloads", "8", "-verify-all"];
    }
    currentSettings.max_retries = parseInt(els.maxRetriesInput.value) || 3;
    currentSettings.native_chunk_concurrency = parseInt(els.chunkConcurrencyInput.value) || 8;
    currentSettings.download_speed_limit = els.speedLimitToggle && els.speedLimitToggle.checked
      ? joinSpeedLimit(els.speedLimitInput.value, getSpeedLimitUnit())
      : '';
    currentSettings.proxy = els.proxyInput.value.trim();
    if (els.hubcapApiKeyInput) {
      currentSettings.hubcap_api_key = els.hubcapApiKeyInput.value.trim();
    }
    if (els.ryuuApiKeyInput) {
      currentSettings.ryuu_api_key = els.ryuuApiKeyInput.value.trim();
    }
    if (els.nativeDownloaderToggle) {
      currentSettings.use_native_downloader = els.nativeDownloaderToggle.checked;
    }
    if (els.cancelKeepFilesToggle) {
      currentSettings.cancel_keep_files = els.cancelKeepFilesToggle.checked;
    }
    currentSettings.notification_sound = els.notificationSoundToggle.checked;

    await invoke('save_settings', { settings: currentSettings });
    state.notificationSoundEnabled = currentSettings.notification_sound;
    checkDotNet();

    if (els.btnSettingsSave && els.btnSettingsSave.dataset.languageRestart === '1') {
      await invoke('restart_app');
      return;
    }
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
  closeSettings();
}

const SPEED_LIMIT_UNITS = {
  '': ['MB/s', 1], m: ['MB/s', 1], mb: ['MB/s', 1], mib: ['MB/s', 1],
  k: ['MB/s', 1 / 1024], kb: ['MB/s', 1 / 1024], kib: ['MB/s', 1 / 1024],
  g: ['MB/s', 1024], gb: ['MB/s', 1024], gib: ['MB/s', 1024],
  mbit: ['Mbit/s', 1], mbits: ['Mbit/s', 1], mbps: ['Mbit/s', 1],
  kbit: ['Mbit/s', 0.001], kbits: ['Mbit/s', 0.001], kbps: ['Mbit/s', 0.001],
  gbit: ['Mbit/s', 1000], gbits: ['Mbit/s', 1000], gbps: ['Mbit/s', 1000],
};

function splitSpeedLimit(text) {
  const match = /^\s*([\d.,]+)\s*([a-z/ ]*)$/i.exec(text || '');
  if (!match) return { value: '', unit: 'MB/s' };
  const number = parseFloat(match[1].replace(',', '.'));
  const unitKey = match[2].toLowerCase().replace(/\s+/g, '').replace(/\/s$/, '');
  const known = SPEED_LIMIT_UNITS[unitKey];
  if (!known || !isFinite(number) || number <= 0) return { value: '', unit: 'MB/s' };
  return { value: String(+(number * known[1]).toFixed(3)), unit: known[0] };
}

function joinSpeedLimit(value, unit) {
  const number = parseFloat(String(value).replace(',', '.'));
  if (!isFinite(number) || number <= 0) return '';
  return `${number} ${unit === 'Mbit/s' ? 'Mbit/s' : 'MB/s'}`;
}

function speedLimitUnitButtons() {
  return document.querySelectorAll('.speed-limit__unit');
}

function getSpeedLimitUnit() {
  const active = document.querySelector('.speed-limit__unit.is-active');
  return active ? active.dataset.unit : 'MB/s';
}

function setSpeedLimitUnit(unit) {
  speedLimitUnitButtons().forEach(btn => {
    const on = btn.dataset.unit === unit;
    btn.classList.toggle('is-active', on);
    btn.setAttribute('aria-checked', on ? 'true' : 'false');
  });
}

function setSpeedLimitEnabled(enabled, focus) {
  if (els.speedLimitToggle) els.speedLimitToggle.checked = enabled;
  if (els.speedLimitControls) els.speedLimitControls.classList.toggle('hidden', !enabled);
  if (enabled && focus && els.speedLimitInput) els.speedLimitInput.focus();
}

function bindSpeedLimitControls() {
  if (els.speedLimitToggle) {
    els.speedLimitToggle.addEventListener('change', () => {
      setSpeedLimitEnabled(els.speedLimitToggle.checked, true);
    });
  }
  speedLimitUnitButtons().forEach(btn => {
    btn.addEventListener('click', () => setSpeedLimitUnit(btn.dataset.unit));
  });
}

function toggleAdvancedSettings() {
  const content = els.advancedSettingsContent;
  const arrow = els.btnToggleAdvanced.querySelector('.settings-advanced__arrow');
  const isHidden = content.classList.contains('hidden');

  if (isHidden) {
    content.classList.remove('hidden');
    arrow.classList.add('expanded');
  } else {
    content.classList.add('hidden');
    arrow.classList.remove('expanded');
  }
}

const SKIPPED_VERSION_KEY = 'skippedUpdateVersion';

async function checkForUpdates() {
  try {
    const enabled = await invoke('get_auto_update_enabled');
    if (!enabled) return;

    const result = await invoke('check_for_updates');

    emitEvent('update_checked', { available: !!result.available });

    if (result.error) {
      console.error('[AutoUpdate] Error:', result.error);
    }
    if (!result.available) return;

    // Check if user has skipped this version
    const skipped = localStorage.getItem(SKIPPED_VERSION_KEY);
    if (skipped === result.version) return;

    showUpdateModal(result);
  } catch (e) {
    console.error('[AutoUpdate] Check failed:', e);
  }
}

/** Simple Markdown → HTML renderer for release notes */
function renderMarkdown(md) {
  let html = md.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  html = html.replace(/^### (.+)$/gm, '<h4>$1</h4>');
  html = html.replace(/^## (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^# (.+)$/gm, '<h2>$1</h2>');
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>');
  html = html.replace(/\n/g, '<br>');
  // Clean up double <br> after block elements
  html = html.replace(/<\/(h[234]|ul|li)><br>/g, '</$1>');
  return html;
}

function showUpdateModal(info) {
  pendingUpdateInfo = info;
  els.updateVersion.textContent = `v${info.version}`;
  if (info.date) {
    try {
      const d = new Date(info.date);
      els.updateDate.textContent = isNaN(d.getTime()) ? info.date : d.toLocaleDateString(window.i18n.getCurrentLocale(), { dateStyle: 'medium' });
    } catch { els.updateDate.textContent = info.date; }
    els.updateDateRow.style.display = '';
  } else {
    els.updateDateRow.style.display = 'none';
  }
  els.updateNotes.innerHTML = info.body
    ? renderMarkdown(info.body)
    : `<em>${escapeHtml(window.i18n.t('modals.update.noReleaseNotes'))}</em>`;
  els.updateProgressWrap.classList.add('hidden');
  els.updateActions.style.display = '';
  els.btnUpdateNow.disabled = false;

  const externallyManaged = info.installMethod && info.installMethod !== 'self';
  const systemHint = document.getElementById('update-system-hint');
  if (systemHint) {
    systemHint.classList.toggle('hidden', !externallyManaged);
    if (externallyManaged) {
      renderUpdateCommands(info.installMethod);
    }
  }
  els.btnUpdateNow.classList.toggle('hidden', externallyManaged);
  els.btnUpdateSkip.classList.toggle('hidden', externallyManaged);

  els.updateModal.classList.remove('hidden');
}

function updateCommandsFor(method) {
  switch (method) {
    case 'flatpak':
      return [{ label: '', cmd: 'flatpak update de.mcbabel.SteamManifestDownloader' }];
    case 'snap':
      return [{ label: '', cmd: 'sudo snap refresh steam-manifest-downloader' }];
    case 'system':
    default:
      return [
        { label: i18n.t('modals.update.aurBinLabel'), cmd: 'paru -Syu steam-manifest-downloader-bin' },
        { label: i18n.t('modals.update.aurSourceLabel'), cmd: 'paru -Syu steam-manifest-downloader' },
      ];
  }
}

function renderUpdateCommands(method) {
  const list = document.getElementById('update-system-cmd-list');
  if (!list) return;
  const commands = updateCommandsFor(method);
  const copyLabel = i18n.t('modals.update.copyCmd');
  list.innerHTML = commands.map((c, i) => {
    const labelHtml = c.label
      ? `<div class="update-system-cmd__label">${escapeHtml(c.label)}</div>`
      : '';
    return `
      <div class="update-system-cmd__row">
        ${labelHtml}
        <div class="update-system-cmd__inner">
          <pre><code>${escapeHtml(c.cmd)}</code></pre>
          <button type="button" class="btn btn--outline btn--small update-system-cmd__copy" data-cmd-idx="${i}">${escapeHtml(copyLabel)}</button>
        </div>
      </div>
    `;
  }).join('');

  list.querySelectorAll('.update-system-cmd__copy').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const idx = parseInt(btn.dataset.cmdIdx, 10);
      const cmd = commands[idx]?.cmd;
      if (!cmd) return;
      try {
        await navigator.clipboard.writeText(cmd);
        const original = btn.textContent;
        btn.textContent = window.i18n.t('common.copied');
        setTimeout(() => { btn.textContent = original; }, 1500);
      } catch {}
    });
  });
}

function hideUpdateModal() {
  els.updateModal.classList.add('hidden');
}

let pendingUpdateInfo = null;

async function performUpdate() {
  if (!pendingUpdateInfo || !pendingUpdateInfo.installerUrl) {
    // No direct installer — open release page in browser
    if (pendingUpdateInfo && pendingUpdateInfo.releaseUrl) {
      window.__TAURI__.shell.open(pendingUpdateInfo.releaseUrl);
    }
    hideUpdateModal();
    return;
  }

  els.btnUpdateNow.disabled = true;
  els.btnUpdateLater.style.display = 'none';
  els.btnUpdateSkip.style.display = 'none';
  els.btnUpdateNow.textContent = window.i18n.t('modals.update.downloading');
  els.updateProgressWrap.classList.remove('hidden');
  els.updateProgressText.textContent = window.i18n.t('modals.update.installerDownloading');
  els.updateProgressFill.style.width = '100%';
  els.updateProgressFill.classList.add('progress-bar__fill--indeterminate');

  emitEvent('update_installed');
  try {
    await invoke('install_update', { installerUrl: pendingUpdateInfo.installerUrl });
    // App will exit — this line may not be reached
  } catch (e) {
    console.error('[AutoUpdate] Install failed:', e);
    els.updateProgressText.textContent = window.i18n.t('modals.update.failed', { message: window.i18n.localizeError(e) });
    els.updateProgressFill.classList.remove('progress-bar__fill--indeterminate');
    els.updateProgressFill.style.width = '0%';
    els.btnUpdateNow.textContent = window.i18n.t('common.retry');
    els.btnUpdateNow.disabled = false;
    els.btnUpdateLater.style.display = '';
  }
}

function skipUpdateVersion() {
  const version = els.updateVersion.textContent.replace(/^v/, '');
  localStorage.setItem(SKIPPED_VERSION_KEY, version);
  hideUpdateModal();
}

// DEV: Test function — call window.testUpdateModal() in browser console
window.testUpdateModal = function() {
  showUpdateModal({
    available: true,
    version: '2.0.0',
    currentVersion: '1.1.0',
    date: new Date().toISOString(),
    body: '### What\'s New\n- ✨ Auto-Update feature\n- 🔧 Bug fixes\n- 🚀 Performance improvements\n\nThis is a **test** update dialog.'
  });
};

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

function initTheme() {
  const saved = localStorage.getItem('theme') || 'dark';
  document.documentElement.setAttribute('data-theme', saved);
  updateThemeButton(saved);
}

function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme') || 'dark';
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', next);
  localStorage.setItem('theme', next);
  updateThemeButton(next);
  emitEvent('theme_toggled', { to: next });
}

function updateThemeButton(theme) {
  if (els.btnThemeToggle) {
    els.btnThemeToggle.innerHTML = theme === 'dark' ? ICONS.moon : ICONS.sun;
    const themeLabel = window.i18n.t(theme === 'dark' ? 'header.themeToLight' : 'header.themeToDark');
    els.btnThemeToggle.title = themeLabel;
    els.btnThemeToggle.setAttribute('aria-label', themeLabel);
  }
}

async function showCancelModal() {
  let keep = false;
  try {
    const settings = await invoke('get_settings');
    keep = !!settings.cancel_keep_files;
  } catch (_) {}
  const body = document.getElementById('cancel-modal-body');
  if (body) {
    body.innerHTML = window.i18n.t(keep ? 'modals.cancel.bodyKeep' : 'modals.cancel.body');
  }
  if (els.btnCancelYes) {
    els.btnCancelYes.textContent = window.i18n.t(keep ? 'modals.cancel.yesKeep' : 'modals.cancel.yes');
  }
  els.cancelModal.classList.remove('hidden');
}

function hideCancelModal() {
  els.cancelModal.classList.add('hidden');
}

async function togglePauseDownload() {
  if (!state.jobId) return;
  const willPause = !state.paused;
  try {
    await invoke('pause_download', { jobId: state.jobId, paused: willPause });
    state.paused = willPause;
    if (els.btnPause) {
      els.btnPause.textContent = willPause
        ? window.i18n.t('progress.resume')
        : window.i18n.t('progress.pause');
    }
    appendTerminalLine(
      window.i18n.t(willPause ? 'progress.pausedLine' : 'progress.resumedLine'),
      'info'
    );
  } catch (e) {
    console.error('pause_download failed:', e);
  }
}

async function cancelDownload() {
  hideCancelModal();

  if (!state.jobId) return;

  els.btnCancel.disabled = true;
  els.btnCancel.innerHTML = escapeHtml(window.i18n.t('progress.cancelling'));
  appendTerminalLine(window.i18n.t('progress.cancellingLine'), 'info');

  try {
    await invoke('cancel_download', { jobId: state.jobId });
  } catch (error) {
    const errStr = String(error);
    // If job is not running, the download already finished or errored — show Start Over
    if (errStr.toLowerCase().includes('not found') || errStr.toLowerCase().includes('not running')) {
      appendTerminalLine(window.i18n.t('progress.noLongerRunning'), 'info');
      showCompletion(false, window.i18n.t('progress.ended'));
    } else {
      appendTerminalLine(window.i18n.t('progress.cancelFailed', { message: window.i18n.localizeError(errStr) }), 'error');
      // Still show Next so user isn't stuck
      if (els.btnNextStep) els.btnNextStep.classList.remove('hidden');
      els.btnCancel.classList.add('hidden');
    }
  }
}

function showDiskSpace(freeGB, drive) {
  els.diskSpaceInfo.classList.remove('hidden', 'disk-space-info--warning', 'disk-space-info--danger');

  if (freeGB < 2) {
    els.diskSpaceInfo.classList.add('disk-space-info--danger');
    els.diskSpaceText.textContent = window.i18n.t('progress.diskCritical', { gb: freeGB, drive });
  } else if (freeGB < 10) {
    els.diskSpaceInfo.classList.add('disk-space-info--warning');
    els.diskSpaceText.textContent = window.i18n.t('progress.diskLow', { gb: freeGB, drive });
  } else {
    els.diskSpaceText.textContent = window.i18n.t('progress.diskFree', { gb: freeGB, drive });
  }
}

function requestNotificationPermission() {
  if (!('Notification' in window)) return;
  if (Notification.permission === 'default') {
    Notification.requestPermission().then(perm => {
      state.notificationsEnabled = perm === 'granted';
    });
  } else {
    state.notificationsEnabled = Notification.permission === 'granted';
  }
}

function showBrowserNotification(title, body, icon) {
  if (!('Notification' in window)) return;
  if (Notification.permission !== 'granted') return;
  if (!document.hidden) return; // Only show when tab is not focused

  try {
    new Notification(title, {
      body,
      icon: icon || undefined
    });
  } catch (e) {
    // Fallback: ignore errors (e.g. service worker requirement)
  }
}

const SHUTDOWN_DELAY_SECONDS = 60;

function maybeScheduleShutdown() {
  if (!els.shutdownAfterToggle || !els.shutdownAfterToggle.checked || !els.shutdownModal) return;
  clearInterval(state.shutdownTimer);
  state.shutdownRemaining = SHUTDOWN_DELAY_SECONDS;
  renderShutdownCountdown();
  els.btnShutdownNow.disabled = false;
  els.shutdownModal.classList.remove('hidden');
  state.shutdownTimer = setInterval(() => {
    state.shutdownRemaining -= 1;
    if (state.shutdownRemaining <= 0) {
      shutdownNow();
    } else {
      renderShutdownCountdown();
    }
  }, 1000);
}

function renderShutdownCountdown() {
  els.shutdownModalBody.textContent = window.i18n.t('modals.shutdown.body', { seconds: state.shutdownRemaining });
}

function abortScheduledShutdown() {
  clearInterval(state.shutdownTimer);
  state.shutdownTimer = null;
  if (els.shutdownModal) els.shutdownModal.classList.add('hidden');
  if (els.shutdownAfterToggle) els.shutdownAfterToggle.checked = false;
}

async function shutdownNow() {
  clearInterval(state.shutdownTimer);
  state.shutdownTimer = null;
  els.btnShutdownNow.disabled = true;
  els.shutdownModalBody.textContent = window.i18n.t('modals.shutdown.running');
  await commitPendingHistory();
  const followupSaved = await savePendingFollowup();
  try {
    await invoke('power_off_system');
  } catch (e) {
    if (followupSaved) invoke('clear_pending_followup').catch(() => {});
    els.shutdownModalBody.textContent = window.i18n.t('modals.shutdown.failed', { message: window.i18n.localizeError(e) });
    els.btnShutdownNow.disabled = false;
    if (els.shutdownAfterToggle) els.shutdownAfterToggle.checked = false;
  }
}

function askConfirm({ title, body, confirm, cancel, danger = false }) {
  const modal = document.getElementById('confirm-modal');
  if (!modal) return Promise.resolve(true);
  document.getElementById('confirm-modal-title').textContent = title;
  document.getElementById('confirm-modal-body').textContent = body;
  const yes = document.getElementById('btn-confirm-yes');
  const no = document.getElementById('btn-confirm-no');
  yes.textContent = confirm;
  no.textContent = cancel || window.i18n.t('common.cancel');
  yes.classList.toggle('btn--danger', danger);
  yes.classList.toggle('btn--primary', !danger);
  return new Promise((resolve) => {
    const finish = (ok) => {
      modal.classList.add('hidden');
      resolve(ok);
    };
    yes.onclick = () => finish(true);
    no.onclick = () => finish(false);
    modal.querySelector('.modal__backdrop').onclick = () => finish(false);
    modal.classList.remove('hidden');
  });
}

function selectedDownloadBytes(depots) {
  return depots.reduce((sum, d) => sum + (Number(d.sizeBytes) || 0), 0);
}

async function confirmFreeSpace(depots, updateDir) {
  if (updateDir) return true;
  const needed = selectedDownloadBytes(depots);
  if (needed <= 0) return true;
  let result = null;
  try {
    result = await invoke('check_free_space', { downloadDir: getDownloadDir() || null, needed });
  } catch (e) {
    console.error('check_free_space failed:', e);
    return true;
  }
  if (!result || result.enough || result.free == null) return true;
  return askConfirm({
    title: window.i18n.t('modals.space.title'),
    body: window.i18n.t('modals.space.body', {
      needed: formatBytes(result.needed) || '0 MB',
      free: formatBytes(result.free) || '0 MB',
      path: result.path,
    }),
    confirm: window.i18n.t('modals.space.start'),
  });
}

function askResumeMode() {
  const modal = document.getElementById('resume-modal');
  if (!modal) return Promise.resolve('verify');
  return new Promise((resolve) => {
    const finish = (mode) => {
      modal.classList.add('hidden');
      resolve(mode);
    };
    document.getElementById('btn-resume-fast').onclick = () => finish('fast');
    document.getElementById('btn-resume-verify').onclick = () => finish('verify');
    document.getElementById('btn-resume-cancel').onclick = () => finish(null);
    modal.querySelector('.modal__backdrop').onclick = () => finish(null);
    modal.classList.remove('hidden');
  });
}

async function savePendingFollowup() {
  const hasNextSteps = state.shortcutSupported || state.steamLibrarySupported || state.emulatorAvailable;
  const appId = currentAppIdForSteam();
  if (state.downloadFailed || !state.downloadDir || !appId || !hasNextSteps) return false;
  try {
    await invoke('save_pending_followup', {
      followup: {
        app_id: appId,
        game_name: state.gameName || null,
        header_image: state.headerImage || null,
        download_dir: state.downloadDir,
        created_at: new Date().toISOString(),
      },
    });
    return true;
  } catch (e) {
    console.error('save_pending_followup failed:', e);
    return false;
  }
}

async function showPendingFollowupIfAny() {
  if (!els.followupModal) return;
  let followup = null;
  try {
    followup = await invoke('get_pending_followup');
  } catch (e) {
    console.error('get_pending_followup failed:', e);
  }
  if (!followup) return;
  const name = followup.game_name || `App ${followup.app_id}`;
  els.followupModalBody.textContent = window.i18n.t('modals.followup.body', { name });
  const close = () => els.followupModal.classList.add('hidden');
  els.btnFollowupLater.onclick = close;
  els.followupModal.querySelector('.modal__backdrop').onclick = close;
  els.btnFollowupDiscard.onclick = () => {
    invoke('clear_pending_followup').catch(() => {});
    close();
  };
  els.btnFollowupResume.onclick = () => {
    invoke('clear_pending_followup').catch(() => {});
    close();
    resumePendingFollowup(followup);
  };
  els.followupModal.classList.remove('hidden');
}

async function resumePendingFollowup(followup) {
  state.downloadDir = followup.download_dir;
  state.gameName = followup.game_name || null;
  state.headerImage = followup.header_image || null;
  state.parsedData = { mainAppId: followup.app_id, depots: [] };
  state.downloadFailed = false;
  await checkEmulatorSupport();
  if (state.shortcutSupported) {
    goToShortcutStep();
  } else if (state.steamLibrarySupported) {
    goToSteamLibraryStep();
  } else if (state.emulatorAvailable) {
    goToEmulatorStep();
  }
}

function playNotificationSound() {
  if (!state.notificationSoundEnabled) return;
  try {
    const ctx = new (window.AudioContext || window.webkitAudioContext)();
    const oscillator = ctx.createOscillator();
    const gainNode = ctx.createGain();
    oscillator.connect(gainNode);
    gainNode.connect(ctx.destination);
    oscillator.frequency.value = 800;
    oscillator.type = 'sine';
    gainNode.gain.setValueAtTime(0.3, ctx.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.5);
    oscillator.onended = () => {
      ctx.close().catch(() => {});
    };
    oscillator.start(ctx.currentTime);
    oscillator.stop(ctx.currentTime + 0.5);
  } catch (e) {
  }
}

async function checkDotNet() {
  const banner = document.getElementById('dotnet-warning');
  try {
    const settings = await invoke('get_settings');
    if (settings.use_native_downloader !== false) {
      if (banner) banner.classList.add('hidden');
      return;
    }

    // Skip if user already dismissed the warning this session
    if (sessionStorage.getItem('dotnetWarningDismissed') === 'true') return;

    const result = await invoke('check_dotnet');
    if (!result.installed) {
      console.warn('.NET 9 runtime not found. DepotDownloader requires .NET 9.');
      showDotNetWarning();
    }
  } catch (e) {
    console.error('Failed to check .NET:', e);
  }
}

function showDotNetWarning() {
  const banner = document.getElementById('dotnet-warning');
  if (!banner) return;
  banner.classList.remove('hidden');

  const dismissBtn = document.getElementById('dotnet-warning-dismiss');
  if (dismissBtn) {
    dismissBtn.addEventListener('click', () => {
      banner.classList.add('hidden');
      // Remember dismissal for this session
      sessionStorage.setItem('dotnetWarningDismissed', 'true');
    });
  }

  const installLink = document.getElementById('dotnet-install-link');
  if (installLink) {
    installLink.addEventListener('click', (e) => {
      e.preventDefault();
      try {
        window.__TAURI__.shell.open('https://dotnet.microsoft.com/en-us/download/dotnet/thank-you/runtime-desktop-9.0.16-windows-x64-installer');
      } catch {
        // Fallback: just let the link work normally
        window.open('https://dotnet.microsoft.com/en-us/download/dotnet/thank-you/runtime-desktop-9.0.16-windows-x64-installer', '_blank');
      }
    });
  }
}

function initEvents() {
  els.tabUpload.addEventListener('click', () => switchTab('upload'));
  els.tabSearch.addEventListener('click', () => { switchTab('search'); refreshSourcesUI(); });
  els.tabPatchOnly.addEventListener('click', () => switchTab('patchOnly'));

  if (els.btnSourcesEmptyAdd) {
    els.btnSourcesEmptyAdd.addEventListener('click', async () => {
      const ok = await addDepotSource(els.sourcesEmptyInput.value, els.sourcesEmptyError);
      if (ok) els.sourcesEmptyInput.value = '';
    });
  }
  if (els.btnSourcesAdd) {
    els.btnSourcesAdd.addEventListener('click', async () => {
      const ok = await addDepotSource(els.sourcesAddInput.value, els.sourcesAddError);
      if (ok) els.sourcesAddInput.value = '';
    });
  }
  if (els.sourcesList) {
    els.sourcesList.addEventListener('click', async (e) => {
      const btn = e.target.closest('button[data-source-idx]');
      if (!btn) return;
      const idx = parseInt(btn.dataset.sourceIdx, 10);
      if (Number.isInteger(idx)) await removeDepotSource(idx);
    });
  }

  els.btnSearch.addEventListener('click', performSearch);
  els.searchAppIdInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      hideAutocomplete(els.searchAutocomplete);
      performSearch();
    }
    if (e.key === 'Escape') {
      hideAutocomplete(els.searchAutocomplete);
    }
  });
  els.searchAppIdInput.addEventListener('input', () => onAutocompleteInput(els.searchAppIdInput, els.searchAutocomplete));
  document.addEventListener('click', (e) => {
    if (!els.searchAppIdInput.contains(e.target) && !els.searchAutocomplete.contains(e.target)) {
      hideAutocomplete(els.searchAutocomplete);
    }
    if (els.patchOnlyAppIdInput && !els.patchOnlyAppIdInput.contains(e.target) && !els.patchOnlyAutocomplete.contains(e.target)) {
      hideAutocomplete(els.patchOnlyAutocomplete);
    }
  });
  els.searchAutocomplete.addEventListener('click', (e) => {
    const item = e.target.closest('.search-autocomplete__item');
    if (!item) return;
    const appId = item.dataset.appid;
    if (!appId) return;
    els.searchAppIdInput.value = appId;
    hideAutocomplete(els.searchAutocomplete);
    performSearch();
  });
  els.btnSearchNext.addEventListener('click', proceedFromSearch);

  if (els.btnPatchOnlyBrowse) {
    els.btnPatchOnlyBrowse.addEventListener('click', browsePatchOnlyDir);
  }
  if (els.btnPatchOnlyStart) {
    els.btnPatchOnlyStart.addEventListener('click', startPatchOnlyEmulator);
  }
  if (els.patchOnlyAppIdInput) {
    els.patchOnlyAppIdInput.addEventListener('input', () => onAutocompleteInput(els.patchOnlyAppIdInput, els.patchOnlyAutocomplete));
    els.patchOnlyAppIdInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        hideAutocomplete(els.patchOnlyAutocomplete);
        startPatchOnlyEmulator();
      }
      if (e.key === 'Escape') {
        hideAutocomplete(els.patchOnlyAutocomplete);
      }
    });
  }
  if (els.patchOnlyAutocomplete) {
    els.patchOnlyAutocomplete.addEventListener('click', (e) => {
      const item = e.target.closest('.search-autocomplete__item');
      if (!item) return;
      const appId = item.dataset.appid;
      if (!appId) return;
      els.patchOnlyAppIdInput.value = appId;
      hideAutocomplete(els.patchOnlyAutocomplete);
    });
  }

  els.btnSelectAll.addEventListener('click', selectAll);
  els.btnDeselectAll.addEventListener('click', deselectAll);
  els.btnBack.addEventListener('click', () => goToStep(1));
  els.btnDownload.addEventListener('click', startDownload);
  if (els.mhApiKey) {
    els.mhApiKey.addEventListener('input', () => {
      if (els.mhApiKey.value.trim()) hideMhKeyRequiredHint();
    });
  }
  els.btnCancel.addEventListener('click', showCancelModal);
  if (els.btnPause) {
    els.btnPause.addEventListener('click', togglePauseDownload);
  }
  if (els.btnBrowseDir) {
    els.btnBrowseDir.addEventListener('click', browseDownloadDir);
  }
  els.btnCancelYes.addEventListener('click', cancelDownload);
  els.btnCancelNo.addEventListener('click', hideCancelModal);
  els.cancelModal.querySelector('.modal__backdrop').addEventListener('click', hideCancelModal);
  if (els.shutdownModal) {
    els.btnShutdownAbort.addEventListener('click', abortScheduledShutdown);
    els.btnShutdownNow.addEventListener('click', shutdownNow);
    els.shutdownModal.querySelector('.modal__backdrop').addEventListener('click', abortScheduledShutdown);
  }

  els.btnHistory.addEventListener('click', openHistory);
  els.btnHistoryClose.addEventListener('click', closeHistory);
  els.btnHistoryClear.addEventListener('click', showHistoryClearConfirm);
  if (els.btnHistoryClearYes) els.btnHistoryClearYes.addEventListener('click', confirmHistoryClear);
  if (els.btnHistoryClearNo) els.btnHistoryClearNo.addEventListener('click', () => els.historyClearModal.classList.add('hidden'));
  if (els.historyClearModal) els.historyClearModal.querySelector('.modal__backdrop').addEventListener('click', () => els.historyClearModal.classList.add('hidden'));
  if (els.btnHistoryRemoveYes) els.btnHistoryRemoveYes.addEventListener('click', confirmHistoryRemove);
  if (els.btnHistoryRemoveNo) els.btnHistoryRemoveNo.addEventListener('click', () => els.historyRemoveModal.classList.add('hidden'));
  if (els.historyRemoveModal) els.historyRemoveModal.querySelector('.modal__backdrop').addEventListener('click', () => els.historyRemoveModal.classList.add('hidden'));
  els.historyModal.querySelector('.modal__backdrop').addEventListener('click', closeHistory);

  els.btnSettings.addEventListener('click', openSettings);
  els.btnSettingsSave.addEventListener('click', saveSettings);
  els.btnSettingsCancel.addEventListener('click', closeSettings);
  els.settingsModal.querySelector('.modal__backdrop').addEventListener('click', closeSettings);

  bindSpeedLimitControls();
  if (els.btnToggleAdvanced) {
    els.btnToggleAdvanced.addEventListener('click', toggleAdvancedSettings);
  }

  if (els.btnCopyBuildInfo) {
    els.btnCopyBuildInfo.addEventListener('click', copyBuildInfo);
  }

  if (els.btnTelemetryAccept) els.btnTelemetryAccept.addEventListener('click', acceptTelemetry);
  if (els.btnTelemetryDecline) els.btnTelemetryDecline.addEventListener('click', declineTelemetry);
  if (els.telemetryToggle) els.telemetryToggle.addEventListener('change', onTelemetryToggleChanged);

  els.btnUpdateNow.addEventListener('click', performUpdate);
  els.btnUpdateLater.addEventListener('click', hideUpdateModal);
  els.btnUpdateSkip.addEventListener('click', skipUpdateVersion);
  els.updateModal.querySelector('.modal__backdrop').addEventListener('click', hideUpdateModal);

  els.btnThemeToggle.addEventListener('click', toggleTheme);

  if (els.depotSearch) {
    els.depotSearch.addEventListener('input', applyDepotFilters);
  }
  if (els.showSelectedOnly) {
    els.showSelectedOnly.addEventListener('change', applyDepotFilters);
  }

  if (els.btnNextStep) {
    els.btnNextStep.addEventListener('click', () => {
      if (state.downloadFailed) {
        state.downloadFailed = false;
        goToStep(2);
        if (state.suggestMhKey) {
          state.suggestMhKey = false;
          showMhKeySuggestionHint();
        }
        return;
      }
      if (state.shortcutSupported) {
        goToShortcutStep();
      } else if (state.steamLibrarySupported) {
        goToSteamLibraryStep();
      } else if (state.emulatorAvailable) {
        goToEmulatorStep();
      } else {
        resetApp();
      }
    });
  }

  if (els.btnBrowseExe) {
    els.btnBrowseExe.addEventListener('click', browseExe);
  }
  if (els.btnCreateShortcuts) {
    els.btnCreateShortcuts.addEventListener('click', () => {
      if (state.shortcutsCreated) {
        advanceFromShortcutStep();
      } else {
        createShortcuts();
      }
    });
  }
  if (els.shortcutDesktop) {
    els.shortcutDesktop.addEventListener('change', updateCreateShortcutsButton);
  }
  if (els.shortcutStartMenu) {
    els.shortcutStartMenu.addEventListener('change', updateCreateShortcutsButton);
  }
  if (els.btnShortcutSkip) {
    els.btnShortcutSkip.addEventListener('click', advanceFromShortcutStep);
  }
  if (els.btnShortcutStartOver) {
    els.btnShortcutStartOver.addEventListener('click', resetApp);
  }
  if (els.btnEmuApply) {
    els.btnEmuApply.addEventListener('click', applyEmuReplacement);
  }
  if (els.btnEmuSaveSettings) {
    els.btnEmuSaveSettings.addEventListener('click', saveEmuSettings);
  }
  if (els.btnEmuMergeDlcs) {
    els.btnEmuMergeDlcs.addEventListener('click', performDlcMerge);
  }
  if (els.btnEmuDrmRemove) {
    els.btnEmuDrmRemove.addEventListener('click', removeDrm);
  }
  if (els.btnEmuDrmCopy) {
    els.btnEmuDrmCopy.addEventListener('click', copyDrmLog);
  }
  if (els.btnEmuNew) {
    els.btnEmuNew.addEventListener('click', () => {
      resetApp();
    });
  }
  if (els.btnEmuStartOver) {
    els.btnEmuStartOver.addEventListener('click', () => goToStep(2));
  }
  if (els.btnSteamAdd) {
    els.btnSteamAdd.addEventListener('click', async () => {
      if (els.btnSteamAdd.dataset.mode === 'next') {
        steamLibraryContinue();
        return;
      }
      const ok = await performSteamLibraryAdd();
      if (ok) switchSteamButtonToNext();
    });
  }
  if (els.btnSteamBrowseExe) {
    els.btnSteamBrowseExe.addEventListener('click', browseSteamExe);
  }
  if (els.btnSteamSkip) {
    els.btnSteamSkip.addEventListener('click', steamLibraryContinue);
  }
  if (els.btnSteamToggleDetected) {
    els.btnSteamToggleDetected.addEventListener('click', () => {
      const list = els.steamDetectedList;
      const arrow = els.btnSteamToggleDetected.querySelector('.settings-advanced__arrow');
      if (!list) return;
      list.classList.toggle('hidden');
      if (arrow) arrow.textContent = list.classList.contains('hidden') ? '▶' : '▼';
    });
  }
  if (els.btnEmuRevert) {
    els.btnEmuRevert.addEventListener('click', showEmuRevertConfirm);
  }
  if (els.btnEmuRevertYes) {
    els.btnEmuRevertYes.addEventListener('click', confirmEmuRevert);
  }
  if (els.btnEmuRevertNo) {
    els.btnEmuRevertNo.addEventListener('click', () => els.emuRevertModal.classList.add('hidden'));
  }
  if (els.btnEmuDownloadYes) {
    els.btnEmuDownloadYes.addEventListener('click', confirmEmuDownload);
  }
  if (els.btnEmuDownloadNo) {
    els.btnEmuDownloadNo.addEventListener('click', cancelEmuDownload);
  }
  if (els.emuDownloadModal) {
    els.emuDownloadModal.querySelector('.modal__backdrop').addEventListener('click', cancelEmuDownload);
    els.emuDownloadModal.addEventListener('click', (e) => {
      const anchor = e.target.closest('a[href^="http"]');
      if (!anchor) return;
      e.preventDefault();
      window.__TAURI__.shell.open(anchor.href);
    });
  }
  if (els.emuRevertModal) {
    els.emuRevertModal.querySelector('.modal__backdrop').addEventListener('click', () => els.emuRevertModal.classList.add('hidden'));
  }
  initEmuAccordion();
  if (els.btnToggleDetected) {
    els.btnToggleDetected.addEventListener('click', () => {
      const list = els.shortcutDetectedList;
      const arrow = els.btnToggleDetected.querySelector('.settings-advanced__arrow');
      list.classList.toggle('hidden');
      if (arrow) arrow.textContent = list.classList.contains('hidden') ? '\u25B6' : '\u25BC';
    });
  }
}

function initTauri() {
  document.getElementById('btn-minimize').addEventListener('click', () => invoke('minimize_window'));
  document.getElementById('btn-maximize').addEventListener('click', () => invoke('maximize_window'));
  document.getElementById('btn-close').addEventListener('click', () => invoke('close_window'));

  // data-tauri-drag-region and -webkit-app-region:drag do NOT work
  // reliably on Linux/WebKitGTK. This manual mousedown handler ensures
  // window dragging works on all platforms by directly calling startDragging().
  const titleBar = document.getElementById('title-bar');
  if (titleBar) {
    titleBar.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      if (e.target.closest('.title-bar__controls')) return;

      // Reserve the top edge for resize; without this the drag eats the resize handle.
      const resizeThreshold = 5;
      if (e.clientY <= resizeThreshold) return;

      // Fire-and-forget. Under Wayland the compositor rejects drag requests that
      // arrive after the event tick, so we can't await here.
      window.__TAURI__.window.getCurrentWindow().startDragging();
    });

    titleBar.addEventListener('dblclick', (e) => {
      if (e.target.closest('.title-bar__controls')) return;
      invoke('maximize_window');
    });
  }

  const closeModal = document.getElementById('close-modal');
  const btnCloseYes = document.getElementById('btn-close-yes');
  const btnCloseNo = document.getElementById('btn-close-no');

  listen('close-requested', () => {
    closeModal.classList.remove('hidden');
  });

  btnCloseNo.addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

  btnCloseYes.addEventListener('click', async () => {
    closeModal.classList.add('hidden');
    if (state.jobId) {
      await invoke('emit_telemetry_event', {
        kind: 'download_abandoned',
        props: abandonProps(),
      }).catch(() => {});
    }
    invoke('close_window');
  });

  closeModal.querySelector('.modal__backdrop').addEventListener('click', () => {
    closeModal.classList.add('hidden');
  });

  checkDotNet();

  checkShortcutSupport();
}

async function openHistory() {
  hideHistoryBanner();
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
    if (['all', 'complete', 'resumable', 'problem'].includes(saved.status)) view.status = saved.status;
    if (['newest', 'oldest', 'name', 'size'].includes(saved.sort)) view.sort = saved.sort;
  } catch (_) {}
  return view;
}

function saveHistoryView() {
  try {
    localStorage.setItem(HISTORY_VIEW_KEY, JSON.stringify({ status: state.historyView.status, sort: state.historyView.sort }));
  } catch (_) {}
}

function historyEntryKind(entry) {
  if (entry.status === 'complete') return 'complete';
  if (entry.status === 'cancelled_resumable' && entry.resume_payload) return 'resumable';
  return 'problem';
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

function syncHistoryToolbar() {
  const view = state.historyView;
  els.historyToolbar.classList.toggle('hidden', (state.cachedHistory || []).length === 0);
  if (els.historySearch.value !== view.query) els.historySearch.value = view.query;
  els.historySort.value = view.sort;
  els.historyToolbar.querySelectorAll('.history-chip').forEach((chip) => {
    const active = chip.dataset.status === view.status;
    chip.classList.toggle('is-active', active);
    chip.setAttribute('aria-checked', active ? 'true' : 'false');
  });
}

function initHistoryToolbar() {
  state.historyView = loadHistoryView();
  els.historySearch.addEventListener('input', () => {
    state.historyView.query = els.historySearch.value;
    renderHistoryEntries(visibleHistory());
  });
  els.historySort.addEventListener('change', () => {
    state.historyView.sort = els.historySort.value;
    saveHistoryView();
    renderHistoryEntries(visibleHistory());
  });
  els.historyToolbar.querySelectorAll('.history-chip').forEach((chip) => {
    chip.addEventListener('click', () => {
      state.historyView.status = chip.dataset.status;
      saveHistoryView();
      syncHistoryToolbar();
      renderHistoryEntries(visibleHistory());
    });
  });
}

function renderHistory(entries) {
  state.cachedHistory = entries || [];
  if (!state.historyView) state.historyView = loadHistoryView();
  syncHistoryToolbar();
  renderHistoryEntries(visibleHistory());
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
    const badgeClass = entry.status === 'complete' ? 'history-entry__badge--complete'
      : entry.status === 'partial' ? 'history-entry__badge--partial'
      : isResumable ? 'history-entry__badge--resumable'
      : entry.status === 'cancelled' ? 'history-entry__badge--cancelled'
      : 'history-entry__badge--failed';
    const statusLabel = entry.status === 'complete' ? window.i18n.t('history.statusComplete')
      : entry.status === 'partial' ? window.i18n.t('history.statusPartial')
      : isResumable ? window.i18n.t('history.statusResumable')
      : entry.status === 'cancelled' ? window.i18n.t('history.statusCancelled')
      : window.i18n.t('history.statusFailed');
    const imgHtml = entry.header_image
      ? `<img class="history-entry__image" src="${escapeHtml(entry.header_image)}" alt="" loading="lazy" onerror="this.style.display='none'">`
      : '<div class="history-entry__image history-entry__image--placeholder"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.4"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></div>';
    const name = entry.game_name ? escapeHtml(entry.game_name) : `App ${escapeHtml(entry.app_id)}`;
    const canUpdate = entry.status === 'complete' && !!entry.download_dir;
    const updateTip = window.i18n.t(canUpdate ? 'history.updateTooltip' : 'history.updateUnavailable');

    return `
      <div class="history-entry" data-entry-id="${escapeHtml(entry.id)}">
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
          </div>
        </div>
        <div class="history-entry__actions">
          ${canResumeNow
            ? `<button class="btn btn--small btn--primary history-action-resume" data-entry-id="${escapeHtml(entry.id)}" title="${escapeHtml(window.i18n.t('history.resumeTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.resumeTooltip'))}">${ICONS.play}</button>`
            : ''}
          <button class="btn btn--small btn--outline history-action-redownload" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" title="${escapeHtml(window.i18n.t('history.redownloadTooltip'))}" aria-label="${escapeHtml(window.i18n.t('history.redownloadTooltip'))}">${ICONS.refresh}</button>
          <button class="btn btn--small btn--outline history-action-update" data-app-id="${escapeHtml(entry.app_id)}" data-depot-ids="${escapeHtml((entry.depot_ids || []).join(','))}" data-path="${escapeHtml(entry.download_dir)}" title="${escapeHtml(updateTip)}" aria-label="${escapeHtml(updateTip)}"${canUpdate ? '' : ' disabled'}>${ICONS.update}</button>
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
        emitEvent('download_started', {
          job: state.dlNonce,
          depot_count: resumeDepots,
          engine: state.currentEngine,
          source_count: state.dlSourceCount,
          had_mh_key: state.dlHadMhKey,
          resumed: true,
        });

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

  els.historyList.querySelectorAll('.history-action-redownload, .history-action-update').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.stopPropagation();
      const appId = btn.dataset.appId;
      const depotIds = (btn.dataset.depotIds || '').split(',').filter(Boolean);
      const isUpdate = btn.classList.contains('history-action-update');
      state.updateDir = isUpdate ? (btn.dataset.path || null) : null;
      state.updateAppId = isUpdate ? String(appId) : null;
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
    });
  });

  els.historyList.querySelectorAll('.history-action-folder').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await invoke('open_folder', { path: btn.dataset.path });
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
    await loadHistory();
  } catch (err) {
    console.error('Failed to remove history entry:', err);
    alert(window.i18n.localizeError(err));
    await loadHistory();
  }
}

async function checkShortcutSupport() {
  try {
    const result = await invoke('is_shortcut_supported');
    state.shortcutSupported = result.supported;
    if (state.shortcutSupported) {
      if (els.step4Connector) els.step4Connector.classList.remove('hidden');
      if (els.step4Indicator) els.step4Indicator.classList.remove('hidden');
    }
  } catch (e) {
    console.error('Failed to check shortcut support:', e);
  }
  await checkSteamLibrarySupport();
  renumberSteps();
}

async function checkSteamLibrarySupport() {
  try {
    const install = await invoke('steam_library_detect');
    state.steamLibrarySupported = true;
    state.steamLibraryUser = install;
  } catch (e) {
    state.steamLibrarySupported = false;
    state.steamLibraryUser = null;
  }

  const showLinuxStep = state.steamLibrarySupported && !state.shortcutSupported;
  const showWindowsToggle = state.shortcutSupported;

  if (els.step6Connector) els.step6Connector.classList.toggle('hidden', !showLinuxStep);
  if (els.step6Indicator) els.step6Indicator.classList.toggle('hidden', !showLinuxStep);
  if (els.shortcutSteamRow) {
    els.shortcutSteamRow.classList.toggle('hidden', !showWindowsToggle);
    const toggle = els.shortcutSteamLibrary;
    const hint = els.shortcutSteamRow.querySelector('.shortcut-option__hint');
    if (toggle) {
      toggle.disabled = !state.steamLibrarySupported;
      if (!state.steamLibrarySupported) toggle.checked = false;
    }
    if (hint) {
      hint.textContent = state.steamLibrarySupported
        ? window.i18n.t('steamLibrary.windowsToggleHint')
        : window.i18n.t('steamLibrary.notDetected');
    }
  }
}

async function goToSteamLibraryStep() {
  goToStep(6);
  resetSteamButtons();
  if (els.steamLibraryStatus) {
    if (state.steamLibraryUser) {
      els.steamLibraryStatus.classList.remove('emu-release-status--busy');
      els.steamLibraryStatus.classList.add('emu-release-status--ready');
      els.steamLibraryStatus.textContent = window.i18n.t('steamLibrary.detected', {
        name: state.steamLibraryUser.persona_name || state.steamLibraryUser.user_id3,
      });
    } else {
      els.steamLibraryStatus.classList.remove('emu-release-status--ready');
      els.steamLibraryStatus.textContent = window.i18n.t('steamLibrary.notDetected');
    }
  }
  if (els.steamGameName) {
    els.steamGameName.value = state.gameName || '';
  }
  if (els.steamLaunchOptions) {
    els.steamLaunchOptions.value = '';
  }
  if (els.steamLibraryResult) {
    els.steamLibraryResult.classList.add('hidden');
  }
  await detectSteamExecutables();
}

async function detectSteamExecutables() {
  if (!state.downloadDir) return;
  if (els.steamExePath) {
    els.steamExePath.value = window.i18n.t('shortcut.exePlaceholder') || 'Scanning...';
  }
  if (els.btnSteamAdd) els.btnSteamAdd.disabled = true;
  if (els.steamDetectedSection) els.steamDetectedSection.classList.add('hidden');

  try {
    const result = await invoke('detect_executables', { downloadDir: state.downloadDir });
    const exes = result.executables || [];
    state.steamLibraryDetectedExes = exes;

    if (exes.length === 0) {
      if (els.steamExePath) {
        els.steamExePath.value = '';
        els.steamExePath.placeholder = window.i18n.t('shortcut.noneFound');
      }
      if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;
      return;
    }

    const recommended = exes.find(e => e.recommended) || exes[0];
    if (els.steamExePath) els.steamExePath.value = recommended.path;
    if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;

    if (exes.length > 1 && els.steamDetectedSection && els.steamDetectedList) {
      els.steamDetectedSection.classList.remove('hidden');
      els.steamDetectedList.innerHTML = exes.map(exe => {
        const sizeStr = formatShortcutFileSize(exe.size);
        const recBadge = exe.recommended ? ` <span class="shortcut-exe-badge">${escapeHtml(window.i18n.t('common.recommended'))}</span>` : '';
        return `<div class="shortcut-exe-item" data-path="${escapeHtml(exe.path)}">
          <span class="shortcut-exe-item__name">${escapeHtml(exe.name)}${recBadge}</span>
          <span class="shortcut-exe-item__size">${sizeStr}</span>
        </div>`;
      }).join('');

      els.steamDetectedList.querySelectorAll('.shortcut-exe-item').forEach(item => {
        item.addEventListener('click', () => {
          if (els.steamExePath) els.steamExePath.value = item.dataset.path;
          if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;
        });
      });
    }
  } catch (e) {
    console.error('detect_executables failed:', e);
    if (els.steamExePath) els.steamExePath.value = '';
    if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;
  }
}

async function browseSteamExe() {
  try {
    const { open } = window.__TAURI__.dialog;
    const opts = {
      defaultPath: state.downloadDir || undefined,
      title: window.i18n.t('shortcut.chooseExeTitle'),
    };
    if (state.shortcutSupported) {
      opts.filters = [{ name: 'Executables', extensions: ['exe'] }];
    }
    const filePath = await open(opts);
    if (filePath) {
      if (els.steamExePath) els.steamExePath.value = filePath;
      if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;
    }
  } catch (e) {
    console.error('Failed to browse for exe:', e);
  }
}

function setSteamLibraryResult(kind, text) {
  if (!els.steamLibraryResult) return;
  els.steamLibraryResult.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  if (kind === 'success') els.steamLibraryResult.classList.add('completion-message--success');
  else if (kind === 'error') els.steamLibraryResult.classList.add('completion-message--error');
  els.steamLibraryResult.textContent = text;
}

function showSteamRunningPrompt() {
  setSteamLibraryResult('error', window.i18n.t('steamLibrary.steamRunning'));
  if (!els.steamLibraryResult) return;
  const actions = document.createElement('div');
  actions.className = 'completion-message__actions';
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'btn btn--outline btn--small';
  btn.textContent = window.i18n.t('steamLibrary.closeAndAdd');
  btn.addEventListener('click', async () => {
    btn.disabled = true;
    const ok = await performSteamLibraryAdd(true);
    if (ok) switchSteamButtonToNext();
  });
  actions.appendChild(btn);
  els.steamLibraryResult.appendChild(actions);
}

function currentAppIdForSteam() {
  if (state.parsedData && state.parsedData.mainAppId) return String(state.parsedData.mainAppId);
  if (state.searchAppId) return String(state.searchAppId);
  return '';
}

function deriveStartDir(exePath) {
  if (!exePath) return '';
  const lastSlash = Math.max(exePath.lastIndexOf('/'), exePath.lastIndexOf('\\'));
  if (lastSlash <= 0) return '';
  const dir = exePath.slice(0, lastSlash);
  return dir.endsWith('/') || dir.endsWith('\\') ? dir : dir + (exePath.includes('\\') ? '\\' : '/');
}

function switchSteamButtonToNext() {
  if (!els.btnSteamAdd) return;
  els.btnSteamAdd.textContent = window.i18n.t('steamLibrary.next');
  els.btnSteamAdd.dataset.mode = 'next';
  els.btnSteamAdd.disabled = false;
  if (els.btnSteamSkip) els.btnSteamSkip.classList.add('hidden');
}

function resetSteamButtons() {
  if (els.btnSteamAdd) {
    els.btnSteamAdd.textContent = window.i18n.t('steamLibrary.add');
    delete els.btnSteamAdd.dataset.mode;
    els.btnSteamAdd.disabled = false;
  }
  if (els.btnSteamSkip) els.btnSteamSkip.classList.remove('hidden');
}

function steamLibraryContinue() {
  if (state.emulatorAvailable) goToEmulatorStep();
  else resetApp();
}

async function performSteamLibraryAdd(closeSteam = false) {
  const exePath = (els.steamExePath && els.steamExePath.value || '').trim();
  if (!exePath) {
    setSteamLibraryResult('error', window.i18n.t('steamLibrary.error', { message: 'no executable selected' }));
    return false;
  }
  const appId = currentAppIdForSteam();
  if (!appId) {
    setSteamLibraryResult('error', window.i18n.t('steamLibrary.error', { message: 'missing app id' }));
    return false;
  }
  const appName = (els.steamGameName && els.steamGameName.value || state.gameName || '').trim()
    || `App ${appId}`;
  const launchOptions = (els.steamLaunchOptions && els.steamLaunchOptions.value || '').trim();
  const startDir = deriveStartDir(exePath);

  if (els.btnSteamAdd) els.btnSteamAdd.disabled = true;
  setSteamLibraryResult('busy', window.i18n.t('steamLibrary.adding'));

  try {
    const result = await invoke('steam_library_add', {
      appId,
      appName,
      exePath,
      startDir,
      launchOptions,
      closeSteam,
    });
    const gridCount = (result.grid_files || []).length;
    const isWindowsExe = exePath.toLowerCase().endsWith('.exe');
    const successKey = result.steam_restarted ? 'steamLibrary.successRestarted' : 'steamLibrary.success';
    let successMsg = window.i18n.t(successKey, { name: appName })
      + '\n\n' + window.i18n.t('steamLibrary.gridArtCount', { count: gridCount });
    if (isWindowsExe) {
      successMsg += '\n' + window.i18n.t('steamLibrary.protonNote');
    }
    setSteamLibraryResult('success', successMsg);
    return true;
  } catch (e) {
    console.error('steam_library_add failed:', e);
    if (String(e) === 'STEAM_RUNNING') {
      showSteamRunningPrompt();
      return false;
    }
    setSteamLibraryResult('error', window.i18n.t('steamLibrary.error', { message: window.i18n.localizeError(e) }));
    return false;
  } finally {
    if (els.btnSteamAdd) els.btnSteamAdd.disabled = false;
  }
}

async function goToShortcutStep() {
  goToStep(4);
  state.shortcutsCreated = false;
  resetShortcutFooter();
  await checkSteamLibrarySupport();
  renumberSteps();
  await detectExecutables();
}

function resetShortcutFooter() {
  if (els.btnCreateShortcuts) {
    els.btnCreateShortcuts.disabled = false;
    els.btnCreateShortcuts.textContent = window.i18n.t('shortcut.createShortcuts');
  }
  if (els.btnShortcutSkip) {
    els.btnShortcutSkip.classList.remove('hidden');
  }
}

function advanceFromShortcutStep() {
  if (state.emulatorAvailable) {
    goToEmulatorStep();
  } else {
    resetApp();
  }
}

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
      setEmuApplyStatus('success', window.i18n.t('emulator.applySuccess', { count: success, total }) + extra);
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

async function detectExecutables() {
  if (!state.downloadDir) return;
  els.shortcutExePath.value = window.i18n.t('shortcut.scanning');
  els.btnCreateShortcuts.disabled = true;
  els.shortcutDetectedSection.classList.add('hidden');

  try {
    const result = await invoke('detect_executables', { downloadDir: state.downloadDir });
    const exes = result.executables || [];

    if (exes.length === 0) {
      els.shortcutExePath.value = '';
      els.shortcutExePath.placeholder = window.i18n.t('shortcut.noneFound');
      els.btnCreateShortcuts.disabled = false;
      return;
    }

    const recommended = exes.find(e => e.recommended) || exes[0];
    els.shortcutExePath.value = recommended.path;
    els.btnCreateShortcuts.disabled = false;

    if (exes.length > 1) {
      els.shortcutDetectedSection.classList.remove('hidden');
      els.shortcutDetectedList.innerHTML = exes.map(exe => {
        const sizeStr = formatShortcutFileSize(exe.size);
        const recBadge = exe.recommended ? ` <span class="shortcut-exe-badge">${escapeHtml(window.i18n.t('common.recommended'))}</span>` : '';
        return `<div class="shortcut-exe-item" data-path="${escapeHtml(exe.path)}">
          <span class="shortcut-exe-item__name">${escapeHtml(exe.name)}${recBadge}</span>
          <span class="shortcut-exe-item__size">${sizeStr}</span>
        </div>`;
      }).join('');

      els.shortcutDetectedList.querySelectorAll('.shortcut-exe-item').forEach(item => {
        item.addEventListener('click', () => {
          els.shortcutExePath.value = item.dataset.path;
          els.btnCreateShortcuts.disabled = false;
        });
      });
    }
  } catch (e) {
    console.error('Failed to detect executables:', e);
    els.shortcutExePath.value = '';
    els.shortcutExePath.placeholder = window.i18n.t('shortcut.detectFailed');
    els.btnCreateShortcuts.disabled = false;
  }
}

async function browseExe() {
  try {
    const { open } = window.__TAURI__.dialog;
    const opts = {
      defaultPath: state.downloadDir || undefined,
      title: window.i18n.t('shortcut.chooseExeTitle')
    };
    if (state.shortcutSupported) {
      opts.filters = [{ name: 'Executables', extensions: ['exe'] }];
    }
    const filePath = await open(opts);
    if (filePath) {
      els.shortcutExePath.value = filePath;
      els.btnCreateShortcuts.disabled = false;
    }
  } catch (e) {
    console.error('Failed to browse for exe:', e);
  }
}

async function createShortcuts() {
  const exePath = els.shortcutExePath.value.trim();
  if (!exePath) return;

  const createDesktop = els.shortcutDesktop.checked;
  const createStartMenu = els.shortcutStartMenu.checked;

  if (!createDesktop && !createStartMenu) {
    showShortcutStatus(false, window.i18n.t('shortcut.noLocation'));
    return;
  }

  els.btnCreateShortcuts.disabled = true;
  els.btnCreateShortcuts.textContent = window.i18n.t('shortcut.creating');

  try {
    const gameName = state.gameName || 'Game';
    const result = await invoke('create_shortcuts', {
      exePath,
      gameName,
      iconPath: null,
      createDesktop,
      createStartMenu
    });

    const messages = [];
    if (result.desktop) messages.push(window.i18n.t('shortcut.desktopCreated'));
    if (result.startMenu) messages.push(window.i18n.t('shortcut.startMenuCreated'));
    if (result.errors && result.errors.length > 0) {
      messages.push(window.i18n.t('shortcut.errors', { list: result.errors.join(', ') }));
    }

    const allGood = (!createDesktop || result.desktop) && (!createStartMenu || result.startMenu);
    if (allGood) emitEvent('shortcut_created', { desktop: !!result.desktop, start_menu: !!result.startMenu });
    showShortcutStatus(allGood, messages.join('. ') + '.');

    if (allGood) {
      state.shortcutsCreated = true;
      els.btnCreateShortcuts.disabled = false;
      els.btnCreateShortcuts.textContent = window.i18n.t('common.next');
      if (els.btnShortcutSkip) els.btnShortcutSkip.classList.add('hidden');
    } else {
      els.btnCreateShortcuts.disabled = false;
      els.btnCreateShortcuts.textContent = window.i18n.t('shortcut.createShortcuts');
    }

    if (els.shortcutSteamLibrary && els.shortcutSteamLibrary.checked && state.steamLibrarySupported) {
      await addToSteamLibraryFromShortcutStep(exePath);
    }
  } catch (e) {
    showShortcutStatus(false, window.i18n.t('shortcut.failed', { message: window.i18n.localizeError(e) }));
    els.btnCreateShortcuts.disabled = false;
    els.btnCreateShortcuts.textContent = window.i18n.t('shortcut.createShortcuts');
  }
}

async function addToSteamLibraryFromShortcutStep(exePath) {
  const appId = currentAppIdForSteam();
  if (!appId) return;
  const appName = state.gameName || `App ${appId}`;
  const startDir = deriveStartDir(exePath);
  try {
    const result = await invoke('steam_library_add', {
      appId,
      appName,
      exePath,
      startDir,
      launchOptions: '',
    });
    const gridCount = (result.grid_files || []).length;
    const msg = window.i18n.t('steamLibrary.success', { name: appName })
      + ' (' + window.i18n.t('steamLibrary.gridArtCount', { count: gridCount }) + ')';
    if (els.shortcutStatus) {
      const existing = els.shortcutStatus.textContent || '';
      els.shortcutStatus.textContent = existing ? existing + '\n\n' + msg : msg;
    }
  } catch (e) {
    console.error('steam_library_add (windows toggle) failed:', e);
    const errMsg = window.i18n.t('steamLibrary.error', { message: window.i18n.localizeError(e) });
    if (els.shortcutStatus) {
      const existing = els.shortcutStatus.textContent || '';
      els.shortcutStatus.textContent = existing ? existing + '\n\n' + errMsg : errMsg;
    }
  }
}

function showShortcutStatus(success, message) {
  els.shortcutStatus.classList.remove('hidden', 'completion-message--success', 'completion-message--error');
  els.shortcutStatus.classList.add(success ? 'completion-message--success' : 'completion-message--error');
  els.shortcutStatus.textContent = message;
}

function updateCreateShortcutsButton() {
  if (!els.btnCreateShortcuts) return;
  const anySelected = els.shortcutDesktop.checked || els.shortcutStartMenu.checked;
  els.btnCreateShortcuts.disabled = !anySelected;
}

function formatShortcutFileSize(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(2) + ' GB';
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

const modalFocusReturn = new WeakMap();

function getFocusable(modal) {
  return Array.from(modal.querySelectorAll(FOCUSABLE_SELECTOR))
    .filter((el) => el.offsetParent !== null || el === document.activeElement);
}

function topmostVisibleDialog() {
  const dialogs = document.querySelectorAll('[role="dialog"]');
  for (let i = dialogs.length - 1; i >= 0; i--) {
    if (!dialogs[i].classList.contains('hidden')) return dialogs[i];
  }
  return null;
}

function setupModalA11y() {
  document.querySelectorAll('[role="dialog"]').forEach((modal) => {
    modal.setAttribute('aria-hidden', modal.classList.contains('hidden') ? 'true' : 'false');

    const observer = new MutationObserver(() => {
      const isHidden = modal.classList.contains('hidden');
      modal.setAttribute('aria-hidden', isHidden ? 'true' : 'false');

      if (!isHidden) {
        modalFocusReturn.set(modal, document.activeElement);
        const focusables = getFocusable(modal);
        if (focusables.length > 0) {
          requestAnimationFrame(() => focusables[0].focus());
        }
      } else {
        const returnTo = modalFocusReturn.get(modal);
        if (returnTo && typeof returnTo.focus === 'function') {
          returnTo.focus();
        }
        modalFocusReturn.delete(modal);
      }
    });

    observer.observe(modal, { attributes: true, attributeFilter: ['class'] });
  });

  document.addEventListener('keydown', (e) => {
    const modal = topmostVisibleDialog();
    if (!modal) return;

    if (e.key === 'Escape') {
      const cancelBtn = modal.querySelector('[data-modal-cancel]')
        || modal.querySelector('.btn--outline')
        || modal.querySelector('button');
      if (cancelBtn) {
        e.preventDefault();
        cancelBtn.click();
      }
      return;
    }

    if (e.key === 'Tab') {
      const focusables = getFocusable(modal);
      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  });
}

const LANGUAGE_FLAGS = {
  en: `<svg viewBox="0 0 60 30" preserveAspectRatio="xMidYMid slice">
    <rect width="60" height="30" fill="#012169"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#fff" stroke-width="6"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#C8102E" stroke-width="2.5"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#fff" stroke-width="10"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#C8102E" stroke-width="6"/>
  </svg>`,
  de: `<svg viewBox="0 0 5 3" preserveAspectRatio="none">
    <rect width="5" height="1" y="0" fill="#000"/>
    <rect width="5" height="1" y="1" fill="#DD0000"/>
    <rect width="5" height="1" y="2" fill="#FFCE00"/>
  </svg>`,
};

function renderLanguageCards(container, activeCode, onSelect) {
  if (!container) return;
  container.innerHTML = '';
  for (const locale of i18n.getAvailableLocales()) {
    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'language-card' + (locale.code === activeCode ? ' is-active' : '');
    card.setAttribute('data-lang', locale.code);
    card.setAttribute('aria-label', locale.label);
    card.innerHTML = `
      <span class="language-card__flag" aria-hidden="true">${LANGUAGE_FLAGS[locale.code] || ''}</span>
      <span class="language-card__label">${escapeHtml(locale.label)}</span>
    `;
    card.addEventListener('click', () => onSelect(locale.code));
    container.appendChild(card);
  }
}

async function initI18n() {
  let settings = null;
  try {
    settings = await invoke('get_settings');
  } catch (e) {
    console.error('Failed to read settings during i18n init:', e);
  }
  const stored = settings && settings.language ? settings.language : '';
  const code = stored || i18n.detectBrowserLocale();
  try {
    await i18n.loadLocale(code);
  } catch (e) {
    console.error('Failed to load locale, falling back to en:', e);
    await i18n.loadLocale(i18n.FALLBACK);
  }
  i18n.applyTranslations(document);
  return { settings, hasStored: !!stored };
}

async function showLanguagePickerIfNeeded(initSettings, hasStored) {
  if (hasStored) return;
  const picker = document.getElementById('language-picker');
  const cards = document.getElementById('language-picker-cards');
  if (!picker || !cards) return;

  await new Promise((resolve) => {
    renderLanguageCards(cards, i18n.getCurrentLocale(), async (code) => {
      try {
        const fresh = await invoke('get_settings');
        fresh.language = code;
        await invoke('save_settings', { settings: fresh });
      } catch (e) {
        console.error('Failed to save language choice:', e);
      }
      if (code !== i18n.getCurrentLocale()) {
        try { await i18n.loadLocale(code); } catch {}
        i18n.applyTranslations(document);
      }
      picker.classList.add('hidden');
      resolve();
    });

    picker.classList.remove('hidden');
    const active = cards.querySelector('.language-card.is-active') || cards.querySelector('.language-card');
    if (active) active.focus();
  });
}

function bindSettingsLanguageCards() {
  const cards = document.getElementById('settings-language-cards');
  if (!cards) return;

  let pendingCode = i18n.getCurrentLocale();
  const initial = i18n.getCurrentLocale();

  const updateSaveLabel = () => {
    if (!els.btnSettingsSave) return;
    if (pendingCode !== initial) {
      els.btnSettingsSave.textContent = i18n.t('settings.languageRestartButton');
      els.btnSettingsSave.dataset.languageRestart = '1';
    } else {
      els.btnSettingsSave.textContent = i18n.t('settings.save');
      delete els.btnSettingsSave.dataset.languageRestart;
    }
  };

  const render = () => {
    renderLanguageCards(cards, pendingCode, async (code) => {
      pendingCode = code;
      try {
        const fresh = await invoke('get_settings');
        fresh.language = code;
        await invoke('save_settings', { settings: fresh });
      } catch (e) {
        console.error('Failed to save language:', e);
      }
      render();
      updateSaveLabel();
    });
  };

  render();
}

document.addEventListener('DOMContentLoaded', async () => {
  const { settings: initSettings, hasStored } = await initI18n();

  initTheme();
  initUpload();
  initEvents();
  initHistoryToolbar();
  loadSettingsAndDefaults();
  initTauri();
  refreshSourcesUI();
  setupModalA11y();

  bindSettingsLanguageCards();
  await showLanguagePickerIfNeeded(initSettings, hasStored);
  await initTelemetryConsent();
  await showPendingFollowupIfAny();
  setTimeout(checkForUpdates, 1500);
});
