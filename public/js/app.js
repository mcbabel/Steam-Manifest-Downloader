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
  gamepad: `<svg class="btn-icon" ${SVG_BASE}><line x1="6" y1="12" x2="10" y2="12"/><line x1="8" y1="10" x2="8" y2="14"/><line x1="15" y1="13" x2="15.01" y2="13"/><line x1="18" y1="11" x2="18.01" y2="11"/><rect x="2" y="6" width="20" height="12" rx="2"/></svg>`,
  shieldCheck: `<svg class="btn-icon" ${SVG_BASE}><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><polyline points="9 12 11 14 15 10"/></svg>`,
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
  btnAutoSelect: $('#btn-auto-select'),
  btnAutoSelectDlc: $('#btn-auto-select-dlc'),
  autoSelectNote: $('#auto-select-note'),
  autoSelectTitle: $('#auto-select-title'),
  autoSelectDetail: $('#auto-select-detail'),
  btnAutoStartCancel: $('#btn-auto-start-cancel'),
  autoSelectDepotsToggle: $('#auto-select-depots-toggle'),
  autoStartDownloadToggle: $('#auto-start-download-toggle'),
  includeDlcToggle: $('#include-dlc-toggle'),
  autoSelectOptions: $('#auto-select-options'),
  targetPlatformOptions: $('#target-platform-options'),
  gameLanguage: $('#game-language'),
  gameLanguageButton: $('#game-language-button'),
  gameLanguageLabel: $('#game-language-label'),
  gameLanguageMenu: $('#game-language-menu'),
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
  settingsSearch: $('#settings-search'),
  settingsDirty: $('#settings-dirty'),
  settingsNoResults: $('#settings-no-results'),
  ddExtraArgsInput: $('#dd-extra-args-input'),
  maxRetriesInput: $('#max-retries-input'),
  chunkConcurrencyInput: $('#chunk-concurrency-input'),
  speedLimitToggle: $('#speed-limit-toggle'),
  speedLimitControls: $('#speed-limit-controls'),
  speedLimitInput: $('#speed-limit-input'),
  proxyInput: $('#proxy-input'),
  proxyError: $('#proxy-error'),
  btnProxyTest: $('#btn-proxy-test'),
  proxyStatus: $('#proxy-status'),
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
  buildInfoDiagnostic: $('#build-info-diagnostic'),
  btnCopyBuildInfo: $('#btn-copy-build-info'),
  btnHistory: $('#btn-history'),
  btnReportBug: $('#btn-report-bug'),
  btnReportFailure: $('#btn-report-failure'),
  bugReportModal: $('#bug-report-modal'),
  historyModal: $('#history-modal'),
  historyList: $('#history-list'),
  btnQueue: $('#btn-queue'),
  btnQueueAdd: $('#btn-queue-add'),
  queueModal: $('#queue-modal'),
  btnQueueStart: $('#btn-queue-start'),
  btnQueueClear: $('#btn-queue-clear'),
  btnQueueClose: $('#btn-queue-close'),
  historyToolbar: $('#history-toolbar'),
  historySearch: $('#history-search'),
  historySort: $('#history-sort'),
  historySortButton: $('#history-sort-button'),
  historySortLabel: $('#history-sort-label'),
  historySortMenu: $('#history-sort-menu'),
  historyFilters: $('#history-filters'),
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
  btnShortcutSteamChoose: $('#btn-shortcut-steam-choose'),
  steamPathInput: $('#steam-path-input'),
  btnSteamPathBrowse: $('#btn-steam-path-browse'),
  steamPathStatus: $('#steam-path-status'),
  shortcutSteamLibrary: $('#shortcut-steam-library'),
};

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
  const saved = await loadDepotSources();
  const empty = saved.length === 0;
  if (els.sourcesEmpty) els.sourcesEmpty.classList.toggle('hidden', !empty);
  if (els.searchInputBlock) els.searchInputBlock.classList.toggle('hidden', empty);
  renderSettingsSources(state.pendingSources || saved);
}

function renderSettingsSources(sources) {
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

function addPendingSource(rawUrl, errorEl) {
  if (errorEl) errorEl.classList.add('hidden');
  const err = validateSourceUrl(rawUrl);
  const url = (rawUrl || '').trim();
  const sources = state.pendingSources || [];
  const message = err || (sources.includes(url) ? window.i18n.t('search.sourceExists') : null);
  if (message) {
    if (errorEl) {
      errorEl.textContent = message;
      errorEl.classList.remove('hidden');
    }
    return false;
  }
  state.pendingSources = sources.concat(url);
  renderSettingsSources(state.pendingSources);
  return true;
}

async function removePendingSource(index) {
  const sources = state.pendingSources || [];
  if (index < 0 || index >= sources.length) return;
  if ((state.pristineSources || []).includes(sources[index])) {
    const confirmed = await confirmRemoveDefaultSource();
    if (!confirmed) return;
  }
  state.pendingSources = sources.filter((_, i) => i !== index);
  renderSettingsSources(state.pendingSources);
  updateSettingsDirty();
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
      reportError('search', null, 'search.noResults');
      return;
    }

    renderRepoList(repos);
    els.searchResults.classList.remove('hidden');
  } catch (error) {
    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;
    emitSearchOutcome(0, 'error');
    showSearchError(window.i18n.localizeError(error));
    reportError('search', error);
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
      reportError('manifests', null, 'errors.noManifests');
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
    reportError('manifests', error);
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

function capitalize(s) {
  if (!s) return '';
  return s.charAt(0).toUpperCase() + s.slice(1);
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

function showToast(text, kind = 'info', durationMs = 4000) {
  const toast = document.getElementById('app-toast');
  if (!toast) return;
  toast.textContent = text;
  toast.className = `app-toast app-toast--${kind}`;
  clearTimeout(toast._hideTimer);
  toast._hideTimer = setTimeout(() => toast.classList.add('hidden'), durationMs);
}

function updateQueueBadge(queue) {
  if (Array.isArray(queue)) state.queue = queue;
  const n = (state.queue || []).length;
  const badge = document.getElementById('queue-badge');
  if (badge) {
    badge.textContent = String(n);
    badge.classList.toggle('hidden', n === 0);
  }
  if (els.btnQueue) {
    const label = window.i18n.t('queue.title');
    els.btnQueue.title = n ? `${label} (${n})` : label;
  }
}

async function refreshQueue() {
  try {
    updateQueueBadge(await invoke('queue_list'));
  } catch (e) {
    console.error('queue_list failed:', e);
  }
  if (els.queueModal && !els.queueModal.classList.contains('hidden')) renderQueue();
}

async function addToQueue() {
  const data = state.parsedData;
  if (!data) return;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));
  if (selectedDepots.length === 0) return;
  if (!(await confirmFreeSpace(selectedDepots, activeUpdateDir(data.mainAppId)))) return;
  const mhApiKey = rememberDownloadInputs();
  const repairManifests = activeRepair(data.mainAppId);
  const depots = collectDepotConfigs(selectedDepots, repairManifests);
  const size = selectedDownloadBytes(selectedDepots);
  const item = {
    id: '',
    app_id: String(data.mainAppId),
    game_name: state.gameName || null,
    header_image: state.headerImage || null,
    depot_count: depots.length,
    size_bytes: size > 0 ? size : null,
    depots,
    config: buildDownloadConfig(data, depots, mhApiKey, repairManifests),
    added_at: new Date().toISOString(),
  };
  try {
    updateQueueBadge(await invoke('queue_add', { item }));
    emitEvent('queue_action', { action: 'added', size: countBucket((state.queue || []).length) });
  } catch (e) {
    alert(window.i18n.localizeError(e));
    return;
  }
  const name = item.game_name || `App ${item.app_id}`;
  showToast(window.i18n.t('queue.added', { name, count: state.queue.length }), 'success');
  resetApp();
}

function renderQueue() {
  const list = document.getElementById('queue-list');
  const running = document.getElementById('queue-running');
  const queue = state.queue || [];
  const current = state.queueRunning && state.queueCurrent;
  running.classList.toggle('hidden', !current);
  if (current) {
    running.textContent = window.i18n.t('queue.running', { name: current.game_name || `App ${current.app_id}` });
  }
  if (queue.length === 0) {
    list.innerHTML = `<div class="history-empty">${escapeHtml(window.i18n.t('queue.empty'))}</div>`;
  } else {
    list.innerHTML = queue.map((item, i) => `
      <div class="queue-item" data-id="${escapeHtml(item.id)}">
        <span class="queue-item__pos">${i + 1}</span>
        <div class="queue-item__info">
          <div class="queue-item__name">${escapeHtml(item.game_name || `App ${item.app_id}`)}</div>
          <div class="queue-item__meta">App ${escapeHtml(item.app_id)} · ${escapeHtml(window.i18n.t('queue.depots', { count: item.depot_count }))}${item.size_bytes ? ` · ${escapeHtml(formatBytes(item.size_bytes) || '')}` : ''}${item.config && item.config.updateDir ? ` · ${escapeHtml(window.i18n.t(item.config.repair ? 'queue.kindRepair' : 'queue.kindUpdate'))}` : ''}</div>
        </div>
        <div class="queue-item__actions">
          <button class="btn btn--small btn--outline queue-up" title="${escapeHtml(window.i18n.t('queue.up'))}" aria-label="${escapeHtml(window.i18n.t('queue.up'))}"${i === 0 ? ' disabled' : ''}>↑</button>
          <button class="btn btn--small btn--outline queue-down" title="${escapeHtml(window.i18n.t('queue.down'))}" aria-label="${escapeHtml(window.i18n.t('queue.down'))}"${i === queue.length - 1 ? ' disabled' : ''}>↓</button>
          <button class="btn btn--small btn--outline queue-remove" title="${escapeHtml(window.i18n.t('queue.remove'))}" aria-label="${escapeHtml(window.i18n.t('queue.remove'))}">${ICONS.trash}</button>
        </div>
      </div>`).join('');
  }
  list.querySelectorAll('.queue-item').forEach((row) => {
    const id = row.dataset.id;
    const act = async (cmd, args) => {
      try {
        updateQueueBadge(await invoke(cmd, args));
      } catch (e) {
        console.error(`${cmd} failed:`, e);
      }
      renderQueue();
    };
    row.querySelector('.queue-up').addEventListener('click', () => act('queue_move', { id, offset: -1 }));
    row.querySelector('.queue-down').addEventListener('click', () => act('queue_move', { id, offset: 1 }));
    row.querySelector('.queue-remove').addEventListener('click', () => act('queue_remove', { id }));
  });
  const busy = !!state.jobId || state.queueRunning;
  els.btnQueueStart.disabled = queue.length === 0 || busy;
  els.btnQueueStart.textContent = window.i18n.t(state.queueRunning ? 'queue.stop' : 'queue.start');
  if (state.queueRunning) els.btnQueueStart.disabled = false;
  els.btnQueueClear.disabled = queue.length === 0;
}

async function openQueue() {
  await refreshQueue();
  renderQueue();
  els.queueModal.classList.remove('hidden');
}

function closeQueue() {
  els.queueModal.classList.add('hidden');
}

async function startQueue() {
  if (state.jobId || state.queueRunning) return;
  state.queueRunning = true;
  emitEvent('queue_action', { action: 'started', size: countBucket((state.queue || []).length) });
  closeQueue();
  await runNextQueued();
}

function stopQueue() {
  if (state.queueRunning) emitEvent('queue_action', { action: 'stopped', size: countBucket((state.queue || []).length) });
  state.queueRunning = false;
  clearTimeout(state.queueTimer);
  state.queueTimer = null;
  state.queueCurrent = null;
  showToast(window.i18n.t('queue.stopped'));
  renderQueue();
}

async function runNextQueued() {
  state.queueTimer = null;
  if (!state.queueRunning) return;
  if (state.jobId) {
    state.queueTimer = setTimeout(runNextQueued, 2000);
    return;
  }
  let next = null;
  try {
    next = await invoke('queue_take_next');
  } catch (e) {
    console.error('queue_take_next failed:', e);
  }
  await refreshQueue();
  if (!next) {
    state.queueRunning = false;
    state.queueCurrent = null;
    return;
  }
  state.queueCurrent = next;
  closeHistory();
  state.emuStandalone = false;
  setEmuEditMode(false);
  cleanupProgressListener();
  state.jobId = null;
  state.parsedData = { mainAppId: next.app_id, depots: next.depots || [] };
  state.selectedDepots = new Set((next.depots || []).map((d) => String(d.depotId)));
  state.gameName = next.game_name || null;
  state.headerImage = next.header_image || null;
  state.updateDir = next.config.updateDir || null;
  state.updateAppId = next.config.updateDir ? String(next.app_id) : null;
  state.repairManifests = null;
  state.repairRunning = !!next.config.repair;
  await runDownload(next.config, next.depots || [], true);
}

function queueAfterDownload(outcome) {
  if (!state.queueRunning) return;
  if (outcome === 'cancelled') {
    stopQueue();
    return;
  }
  (async () => {
    await refreshQueue();
    const remaining = (state.queue || []).length;
    if (remaining === 0) {
      state.queueRunning = false;
      state.queueCurrent = null;
      showToast(window.i18n.t('queue.finished'), 'success', 6000);
      maybeScheduleShutdown();
      return;
    }
    await commitPendingHistory();
    if (outcome !== 'failed') await savePendingFollowup();
    appendTerminalLine(window.i18n.t('queue.nextIn', { seconds: QUEUE_NEXT_DELAY_SECONDS }), 'info');
    state.queueTimer = setTimeout(runNextQueued, QUEUE_NEXT_DELAY_SECONDS * 1000);
  })();
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
    if (els.steamPathInput) {
      els.steamPathInput.value = settings.steam_path || '';
      refreshSteamPathStatus();
    }
    if (els.proxyError) els.proxyError.classList.add('hidden');
    setProxyStatus(null);
    if (els.hubcapApiKeyInput) els.hubcapApiKeyInput.value = settings.hubcap_api_key || '';
    if (els.ryuuApiKeyInput) els.ryuuApiKeyInput.value = settings.ryuu_api_key || '';
    const webApiInput = document.getElementById('steam-webapi-key-input');
    if (webApiInput) webApiInput.value = settings.steam_web_api_key || '';
    if (els.nativeDownloaderToggle) els.nativeDownloaderToggle.checked = !!settings.use_native_downloader;
    if (els.cancelKeepFilesToggle) els.cancelKeepFilesToggle.checked = !!settings.cancel_keep_files;
    populateDepotSelectionSettings(settings);
    els.notificationSoundToggle.checked = settings.notification_sound !== false;
    els.telemetryToggle.checked = settings.telemetry_consent === 'accepted';
    state.savedTelemetry = settings.telemetry_consent === 'accepted';
    state.pendingSources = Array.isArray(settings.depot_sources) ? settings.depot_sources.slice() : [];
    state.pristineSources = Array.isArray(settings.pristine_default_sources) ? settings.pristine_default_sources : [];
  } catch (e) {
    els.autoUpdateToggle.checked = true;
  }
  loadBuildInfo();
  renderSettingsSources(state.pendingSources || []);
  if (els.sourcesAddInput) els.sourcesAddInput.value = '';
  if (els.sourcesAddError) els.sourcesAddError.classList.add('hidden');
  resetSettingsLanguage();
  syncEngineDependentSettings();
  let section = 'general';
  try { section = localStorage.getItem(SETTINGS_SECTION_KEY) || 'general'; } catch (_) {}
  showSettingsSection(section);
  resetSettingsDirty();
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
    els.buildInfoPlatform.textContent = `${info.targetOs || '?'} / ${info.targetArch || '?'} · ${info.package || '?'}`;
    if (els.buildInfoDiagnostic) {
      els.buildInfoDiagnostic.textContent = info.diagnosticId || window.i18n.t('settings.diagnosticIdOff');
    }
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

function issueBody(parts) {
  const section = (title, text) => `## ${title}\n\n${text.trim() || '_No answer_'}\n`;
  let body = [
    section('Description', parts.what),
    section('Steps to reproduce', parts.steps),
    section('Expected behavior', parts.expected),
  ].join('\n');
  if (parts.diag) body += `\n## Environment\n\n${parts.diag.map(l => `- ${l}`).join('\n')}\n`;
  if (parts.log && parts.log.length) {
    body += `\n## Logs / terminal output\n\n<details>\n<summary>Last lines of the download log</summary>\n\n\`\`\`\n${parts.log.join('\n')}\n\`\`\`\n\n</details>\n`;
  }
  body += '\n_Sent from the app\'s bug report form._\n';
  return body;
}

function issueUrl(title, body) {
  const params = new URLSearchParams({ template: 'bug_report.md', labels: 'bug', title: `[Bug] ${title}`, body });
  return `${ISSUE_URL}?${params.toString()}`;
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
  let url = issueUrl(title, issueBody(parts));
  while (url.length > ISSUE_URL_LIMIT && parts.log && parts.log.length > 5) {
    parts.log = parts.log.slice(Math.ceil(parts.log.length / 3));
    url = issueUrl(title, issueBody(parts));
  }
  for (const key of ['what', 'steps', 'expected']) {
    while (url.length > ISSUE_URL_LIMIT && parts[key].length > 200) {
      parts[key] = parts[key].slice(0, Math.floor(parts[key].length * 0.75)) + ' …';
      url = issueUrl(title, issueBody(parts));
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

function closeSettings() {
  els.settingsModal.classList.add('hidden');
  state.pendingSources = null;
  state.settingsBaseline = null;
  resetSettingsLanguage();
}

async function saveSettings() {
  const autoUpdate = els.autoUpdateToggle.checked;
  const changed = state.settingsBaseline != null && settingsSnapshot() !== state.settingsBaseline;
  try {
    const currentSettings = await invoke('get_settings');
    const before = JSON.parse(JSON.stringify(currentSettings));
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
    const webApiInput = document.getElementById('steam-webapi-key-input');
    if (webApiInput) currentSettings.steam_web_api_key = webApiInput.value.trim();
    if (els.nativeDownloaderToggle) {
      currentSettings.use_native_downloader = els.nativeDownloaderToggle.checked;
    }
    if (els.cancelKeepFilesToggle) {
      currentSettings.cancel_keep_files = els.cancelKeepFilesToggle.checked;
    }
    if (els.steamPathInput) {
      currentSettings.steam_path = els.steamPathInput.value.trim();
    }
    if (els.autoSelectDepotsToggle) {
      currentSettings.auto_select_depots = els.autoSelectDepotsToggle.checked;
      currentSettings.auto_start_download = els.autoStartDownloadToggle.checked;
      currentSettings.include_dlc = els.includeDlcToggle.checked;
      currentSettings.target_platform = state.targetPlatform || '';
      currentSettings.game_language = state.gameLanguage || '';
    }
    currentSettings.notification_sound = els.notificationSoundToggle.checked;
    if (state.pendingSources) currentSettings.depot_sources = state.pendingSources.slice();
    const languageChanged = state.settingsLanguage && state.settingsLanguage !== i18n.getCurrentLocale();
    if (languageChanged) currentSettings.language = state.settingsLanguage;

    await invoke('save_settings', { settings: currentSettings });
    const changedKeys = Object.keys(currentSettings)
      .filter(k => JSON.stringify(before[k]) !== JSON.stringify(currentSettings[k]));
    const telemetryOn = els.telemetryToggle.checked;
    if (telemetryOn !== state.savedTelemetry) {
      changedKeys.push('telemetry_consent');
      await invoke('set_telemetry_consent', { accept: telemetryOn });
      state.savedTelemetry = telemetryOn;
    }
    if (changedKeys.length && telemetryOn) emitEvent('settings_saved', { keys: changedKeys });
    state.pendingSources = null;
    refreshSourcesUI();
    state.notificationSoundEnabled = currentSettings.notification_sound;
    checkDotNet();
    checkSteamLibrarySupport();

    if (els.btnSettingsSave && els.btnSettingsSave.dataset.languageRestart === '1') {
      await invoke('restart_app');
      return;
    }
    if (changed) showToast(window.i18n.t('settings.saved'), 'success');
  } catch (e) {
    console.error('Failed to save settings:', e);
    if (/proxy/i.test(String(e)) && els.proxyError) {
      els.proxyError.textContent = window.i18n.localizeError(String(e));
      els.proxyError.classList.remove('hidden');
      showSettingsSection('sources');
      els.proxyInput.focus();
      return;
    }
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

const GAME_LANGUAGES = ['english', 'german', 'french', 'italian', 'spanish', 'latam', 'schinese', 'tchinese',
  'japanese', 'koreana', 'russian', 'polish', 'brazilian', 'portuguese', 'turkish', 'ukrainian', 'czech',
  'dutch', 'danish', 'finnish', 'norwegian', 'swedish', 'hungarian', 'romanian', 'thai', 'vietnamese',
  'greek', 'bulgarian', 'arabic', 'indonesian'];

function populateDepotSelectionSettings(settings) {
  if (!els.autoSelectDepotsToggle) return;
  els.autoSelectDepotsToggle.checked = !!settings.auto_select_depots;
  els.autoStartDownloadToggle.checked = !!settings.auto_start_download;
  els.includeDlcToggle.checked = !!settings.include_dlc;
  setTargetPlatform(settings.target_platform || '');
  setGameLanguage(settings.game_language || '');
  syncAutoSelectOptions();
}

function syncAutoSelectOptions() {
  els.autoSelectOptions.classList.toggle('is-disabled', !els.autoSelectDepotsToggle.checked);
}

function setTargetPlatform(value) {
  state.targetPlatform = value;
  els.targetPlatformOptions.querySelectorAll('.segmented__option').forEach(btn => {
    const on = btn.dataset.value === value;
    btn.classList.toggle('is-active', on);
    btn.setAttribute('aria-checked', on ? 'true' : 'false');
  });
}

function gameLanguageOptions() {
  return Array.from(els.gameLanguageMenu.querySelectorAll('.history-sort__option'));
}

function renderGameLanguageMenu() {
  const values = [''].concat(GAME_LANGUAGES);
  els.gameLanguageMenu.innerHTML = values.map(v => {
    const label = v ? steamLanguageName(v) : window.i18n.t('settings.gameLanguageAuto');
    return `<li class="history-sort__option" role="option" data-value="${escapeHtml(v)}">${escapeHtml(label)}</li>`;
  }).join('');
  gameLanguageOptions().forEach(opt => {
    opt.addEventListener('click', () => chooseGameLanguage(opt.dataset.value));
    opt.addEventListener('mouseenter', () => {
      gameLanguageOptions().forEach(o => o.classList.toggle('is-focused', o === opt));
    });
  });
}

function setGameLanguage(value) {
  state.gameLanguage = GAME_LANGUAGES.includes(value) ? value : '';
  gameLanguageOptions().forEach(opt => {
    const active = opt.dataset.value === state.gameLanguage;
    opt.classList.toggle('is-active', active);
    opt.setAttribute('aria-selected', active ? 'true' : 'false');
    if (active) els.gameLanguageLabel.textContent = opt.textContent;
  });
}

function setGameLanguageOpen(open) {
  els.gameLanguageMenu.classList.toggle('hidden', !open);
  els.gameLanguageButton.setAttribute('aria-expanded', open ? 'true' : 'false');
  els.gameLanguage.classList.toggle('is-open', open);
  if (open) {
    const active = els.gameLanguageMenu.querySelector('.is-active') || els.gameLanguageMenu.firstElementChild;
    gameLanguageOptions().forEach(o => o.classList.toggle('is-focused', o === active));
    if (active) active.scrollIntoView({ block: 'nearest' });
  }
}

function chooseGameLanguage(value) {
  setGameLanguage(value);
  setGameLanguageOpen(false);
  els.gameLanguageButton.focus();
}

function bindDepotSelectionSettings() {
  if (!els.autoSelectDepotsToggle) return;
  renderGameLanguageMenu();
  setGameLanguage('');
  els.autoSelectDepotsToggle.addEventListener('change', syncAutoSelectOptions);
  els.targetPlatformOptions.querySelectorAll('.segmented__option').forEach(btn => {
    btn.addEventListener('click', () => setTargetPlatform(btn.dataset.value));
  });
  els.gameLanguageButton.addEventListener('click', () => {
    setGameLanguageOpen(els.gameLanguageMenu.classList.contains('hidden'));
  });
  els.gameLanguage.addEventListener('keydown', (e) => {
    const open = !els.gameLanguageMenu.classList.contains('hidden');
    const options = gameLanguageOptions();
    const focused = options.findIndex(o => o.classList.contains('is-focused'));
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      setGameLanguageOpen(false);
      els.gameLanguageButton.focus();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) {
        setGameLanguageOpen(true);
        return;
      }
      const next = (focused + (e.key === 'ArrowDown' ? 1 : options.length - 1)) % options.length;
      options.forEach((o, i) => o.classList.toggle('is-focused', i === next));
      options[next].scrollIntoView({ block: 'nearest' });
    } else if ((e.key === 'Enter' || e.key === ' ') && open && focused >= 0) {
      e.preventDefault();
      chooseGameLanguage(options[focused].dataset.value);
    } else if (e.key === 'Tab' && open) {
      setGameLanguageOpen(false);
    }
  }, true);
  document.addEventListener('click', (e) => {
    if (!els.gameLanguage.contains(e.target)) setGameLanguageOpen(false);
  });
}

function setProxyStatus(kind, text) {
  if (!els.proxyStatus) return;
  els.proxyStatus.className = 'proxy-status' + (kind ? ` proxy-status--${kind}` : ' hidden');
  els.proxyStatus.textContent = text || '';
}

function proxyKind(proxy) {
  const p = String(proxy || '').trim().toLowerCase();
  if (!p) return 'none';
  if (p.startsWith('socks')) return 'socks';
  if (p.startsWith('https')) return 'https';
  return 'http';
}

async function testProxy() {
  const proxy = els.proxyInput.value.trim();
  if (els.proxyError) els.proxyError.classList.add('hidden');
  els.btnProxyTest.disabled = true;
  setProxyStatus('busy', window.i18n.t(proxy ? 'settings.proxyTesting' : 'settings.proxyTestingDirect'));
  try {
    const result = await invoke('test_proxy', { proxy });
    const key = result && result.proxy ? 'settings.proxyOk' : 'settings.proxyOkDirect';
    setProxyStatus('ok', '✓ ' + window.i18n.t(key, { ms: result ? result.millis : 0 }));
    emitEvent('proxy_tested', { proxy: proxyKind(proxy), ok: true });
  } catch (e) {
    emitEvent('proxy_tested', { proxy: proxyKind(proxy), ok: false, key: window.i18n.errorKey(String(e)) || 'unmatched' });
    const key = proxy ? 'settings.proxyFailed' : 'settings.proxyFailedDirect';
    setProxyStatus('error', '✗ ' + window.i18n.t(key, { error: window.i18n.localizeError(String(e)) }));
  } finally {
    els.btnProxyTest.disabled = false;
  }
}

async function pickSteamFolder() {
  try {
    const { open } = window.__TAURI__.dialog;
    const picked = await open({
      directory: true,
      title: window.i18n.t('steamLibrary.chooseFolderTitle'),
    });
    return typeof picked === 'string' ? picked : null;
  } catch (e) {
    console.error('Steam folder dialog failed:', e);
    return null;
  }
}

function setSteamPathStatus(kind, text) {
  if (!els.steamPathStatus) return;
  els.steamPathStatus.className = 'proxy-status' + (kind ? ` proxy-status--${kind}` : ' hidden');
  els.steamPathStatus.textContent = text || '';
}

async function refreshSteamPathStatus() {
  if (!els.steamPathInput) return null;
  const path = els.steamPathInput.value.trim();
  setSteamPathStatus('busy', window.i18n.t('steamLibrary.detecting'));
  try {
    const install = path
      ? await invoke('steam_library_check_dir', { path })
      : await invoke('steam_library_detect');
    const params = { dir: install.steam_dir, name: install.persona_name || install.user_id3 };
    setSteamPathStatus('ok', '✓ ' + window.i18n.t(path ? 'settings.steamPathFound' : 'settings.steamPathAuto', params));
    return install;
  } catch (e) {
    setSteamPathStatus('error', '✗ ' + window.i18n.localizeError(String(e)));
    return null;
  }
}

async function saveSteamPath(path) {
  const current = await invoke('get_settings');
  current.steam_path = path;
  await invoke('save_settings', { settings: current });
}

async function chooseSteamFolderForShortcut() {
  const picked = await pickSteamFolder();
  if (!picked) return;
  const hint = els.shortcutSteamRow ? els.shortcutSteamRow.querySelector('.shortcut-option__hint') : null;
  try {
    await invoke('steam_library_check_dir', { path: picked });
    await saveSteamPath(picked);
    await checkSteamLibrarySupport();
    if (els.shortcutSteamLibrary && state.steamLibrarySupported) els.shortcutSteamLibrary.checked = true;
  } catch (e) {
    if (hint) hint.textContent = window.i18n.localizeError(String(e));
  }
}

function bindSteamPathControls() {
  if (els.btnShortcutSteamChoose) els.btnShortcutSteamChoose.addEventListener('click', chooseSteamFolderForShortcut);
  if (!els.steamPathInput) return;
  els.btnSteamPathBrowse.addEventListener('click', async () => {
    const picked = await pickSteamFolder();
    if (!picked) return;
    els.steamPathInput.value = picked;
    refreshSteamPathStatus();
  });
  els.steamPathInput.addEventListener('change', refreshSteamPathStatus);
}

function bindProxyControls() {
  if (!els.btnProxyTest) return;
  els.btnProxyTest.addEventListener('click', testProxy);
  els.proxyInput.addEventListener('input', () => setProxyStatus(null));
  els.proxyInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      testProxy();
    }
  });
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

const SETTINGS_SECTION_KEY = 'settingsSection';

function showSettingsSection(section) {
  const modal = els.settingsModal;
  if (!modal) return;
  if (els.settingsSearch && els.settingsSearch.value) {
    els.settingsSearch.value = '';
    filterSettings('');
  }
  const tabs = modal.querySelectorAll('.settings__tab');
  const target = Array.from(tabs).some(t => t.dataset.section === section) ? section : 'general';
  tabs.forEach(t => {
    const on = t.dataset.section === target;
    t.classList.toggle('is-active', on);
    t.setAttribute('aria-selected', on ? 'true' : 'false');
  });
  modal.querySelectorAll('.settings__panel').forEach(p => p.classList.toggle('is-active', p.dataset.section === target));
  const panels = modal.querySelector('.settings__panels');
  if (panels) panels.scrollTop = 0;
  try { localStorage.setItem(SETTINGS_SECTION_KEY, target); } catch (_) {}
}

function filterSettings(query) {
  const modal = els.settingsModal;
  if (!modal) return;
  const q = query.trim().toLowerCase();
  modal.classList.toggle('settings--searching', !!q);
  let any = false;
  modal.querySelectorAll('.settings__panel').forEach(panel => {
    let panelHit = false;
    panel.querySelectorAll('.settings-group').forEach(group => {
      let groupHit = false;
      group.querySelectorAll('.settings-row').forEach(row => {
        const hit = !q || row.textContent.toLowerCase().includes(q);
        row.classList.toggle('is-filtered', !hit);
        if (hit) groupHit = true;
      });
      group.classList.toggle('is-filtered', !groupHit);
      const title = group.previousElementSibling;
      if (title && title.classList.contains('settings__group-title')) title.classList.toggle('is-filtered', !groupHit);
      if (groupHit) panelHit = true;
    });
    panel.classList.toggle('is-match', !!q && panelHit);
    if (panelHit) any = true;
  });
  if (els.settingsNoResults) els.settingsNoResults.classList.toggle('hidden', !q || any);
}

const SETTINGS_NOT_SAVED = new Set(['settings-search', 'sources-add-input']);

function settingsSnapshot() {
  const panels = els.settingsModal && els.settingsModal.querySelector('.settings__panels');
  if (!panels) return '';
  const parts = [];
  const limitOn = !!(els.speedLimitToggle && els.speedLimitToggle.checked);
  panels.querySelectorAll('input[id]').forEach(input => {
    if (SETTINGS_NOT_SAVED.has(input.id)) return;
    if (input === els.speedLimitInput && !limitOn) return;
    parts.push(`${input.id}=${input.type === 'checkbox' ? input.checked : input.value.trim()}`);
  });
  const active = limitOn
    ? '.segmented__option.is-active, .speed-limit__unit.is-active, .language-card.is-active'
    : '.segmented__option.is-active, .language-card.is-active';
  panels.querySelectorAll(active).forEach(el => {
    parts.push(`${el.className}=${el.dataset.value ?? el.dataset.unit ?? el.dataset.lang ?? ''}`);
  });
  parts.push(`gameLanguage=${state.gameLanguage || ''}`);
  parts.push(`sources=${(state.pendingSources || []).join(' ')}`);
  return parts.join('|');
}

function resetSettingsDirty() {
  state.settingsBaseline = settingsSnapshot();
  if (els.settingsDirty) els.settingsDirty.classList.add('hidden');
}

function updateSettingsDirty() {
  if (!els.settingsDirty || state.settingsBaseline == null) return;
  els.settingsDirty.classList.toggle('hidden', settingsSnapshot() === state.settingsBaseline);
}

function syncEngineDependentSettings() {
  const row = document.getElementById('dd-extra-args-row');
  if (row && els.nativeDownloaderToggle) row.classList.toggle('is-disabled', els.nativeDownloaderToggle.checked);
}

function initSettingsLayout() {
  const modal = els.settingsModal;
  if (!modal) return;
  modal.querySelectorAll('.settings__tab').forEach(tab => {
    tab.addEventListener('click', () => showSettingsSection(tab.dataset.section));
  });
  const nav = modal.querySelector('.settings__nav');
  if (nav) {
    nav.addEventListener('keydown', (e) => {
      if (!['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight'].includes(e.key)) return;
      const tabs = Array.from(nav.querySelectorAll('.settings__tab'));
      const i = tabs.indexOf(document.activeElement);
      if (i < 0) return;
      e.preventDefault();
      const step = e.key === 'ArrowDown' || e.key === 'ArrowRight' ? 1 : -1;
      const next = tabs[(i + step + tabs.length) % tabs.length];
      next.focus();
      showSettingsSection(next.dataset.section);
    });
  }
  if (els.settingsSearch) {
    els.settingsSearch.addEventListener('input', () => filterSettings(els.settingsSearch.value));
    els.settingsSearch.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && els.settingsSearch.value) {
        e.stopPropagation();
        els.settingsSearch.value = '';
        filterSettings('');
      }
    });
  }
  const panels = modal.querySelector('.settings__panels');
  if (panels) {
    const recheck = () => setTimeout(updateSettingsDirty, 0);
    panels.addEventListener('input', recheck);
    panels.addEventListener('change', recheck);
    panels.addEventListener('click', recheck);
    panels.addEventListener('keydown', recheck);
  }
  if (els.nativeDownloaderToggle) els.nativeDownloaderToggle.addEventListener('change', syncEngineDependentSettings);
}

const SKIPPED_VERSION_KEY = 'skippedUpdateVersion';

async function checkForUpdates() {
  try {
    const enabled = await invoke('get_auto_update_enabled');
    if (!enabled) return;

    const result = await invoke('check_for_updates');

    emitEvent('update_checked', { available: !!result.available, failed: !!result.error });
    if (result.error) reportError('update_check', result.error);

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

const RELEASE_REPO = 'MCbabel/Steam-Manifest-Downloader';

function releaseInline(text) {
  const codes = [];
  let out = text.replace(/`([^`]+)`/g, (_, c) => {
    codes.push(c);
    return `\u0000${codes.length - 1}\u0000`;
  });
  const links = [];
  const keep = (html) => {
    links.push(html);
    return `\u0001${links.length - 1}\u0001`;
  };
  const safeUrl = (u) => (/^https?:\/\//i.test(u) ? u : null);
  out = out.replace(/!\[[^\]]*\]\([^)]*\)/g, '');
  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, label, url) => {
    const u = safeUrl(url);
    return u ? keep(`<a href="${escapeHtml(u)}" data-external>${escapeHtml(label)}</a>`) : label;
  });
  out = out.replace(/https?:\/\/[^\s<>()]+[^\s<>().,:;!?]/g, (u) => keep(`<a href="${escapeHtml(u)}" data-external>${escapeHtml(u.replace(/^https?:\/\//, ''))}</a>`));
  out = out.replace(/\b([A-Za-z0-9-]+\/[A-Za-z0-9._-]+)#(\d+)\b/g, (m, repo, n) => keep(`<a class="update-modal__ref" href="https://github.com/${repo}/issues/${n}" data-external>${escapeHtml(m)}</a>`));
  out = out.replace(/(^|[^\w&/])#(\d+)\b/g, (m, pre, n) => pre + keep(`<a class="update-modal__ref" href="https://github.com/${RELEASE_REPO}/issues/${n}" data-external>#${n}</a>`));
  out = out.replace(/(^|[^\w/])@([A-Za-z0-9](?:[A-Za-z0-9-]{0,38}))\b/g, (m, pre, user) => pre + keep(`<a class="update-modal__ref" href="https://github.com/${user}" data-external>@${escapeHtml(user)}</a>`));
  out = escapeHtml(out);
  out = out.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  out = out.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  out = out.replace(/(^|[^*\w])\*([^*\s][^*]*?)\*(?![*\w])/g, '$1<em>$2</em>');
  out = out.replace(/(^|[^_\w])_([^_\s][^_]*?)_(?![_\w])/g, '$1<em>$2</em>');
  out = out.replace(/\u0001(\d+)\u0001/g, (_, i) => links[Number(i)]);
  out = out.replace(/\u0000(\d+)\u0000/g, (_, i) => `<code>${escapeHtml(codes[Number(i)])}</code>`);
  return out.trim();
}

function releaseCells(line) {
  return line.trim().replace(/^\||\|$/g, '').split('|').map(c => c.trim());
}

function renderReleaseNotes(md) {
  const lines = String(md || '').replace(/\r\n?/g, '\n').split('\n');
  const html = [];
  let para = [];
  let list = null;
  let skipLevel = 0;
  const flushPara = () => {
    if (para.length) html.push(`<p>${releaseInline(para.join(' '))}</p>`);
    para = [];
  };
  const flushList = () => {
    if (list) html.push(`<${list.tag}>${list.items.map(i => `<li>${releaseInline(i)}</li>`).join('')}</${list.tag}>`);
    list = null;
  };
  const flush = () => { flushPara(); flushList(); };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (heading && skipLevel && heading[1].length <= skipLevel) skipLevel = 0;
    if (skipLevel) continue;
    if (heading && /\bdownloads?\s*$/i.test(heading[2].replace(/[^\w\s]/g, '').trim()) && heading[1].length >= 2) {
      flush();
      skipLevel = heading[1].length;
      continue;
    }
    if (!trimmed) { flush(); continue; }
    if (heading) {
      flush();
      const level = Math.min(Math.max(heading[1].length, 2), 4);
      html.push(`<h${level}>${releaseInline(heading[2])}</h${level}>`);
      continue;
    }
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      flush();
      if (html.length && !html[html.length - 1].startsWith('<hr')) html.push('<hr>');
      continue;
    }
    if (trimmed.startsWith('|') && i + 1 < lines.length && /^\|?\s*:?-{2,}/.test(lines[i + 1].trim())) {
      flush();
      const head = releaseCells(trimmed);
      i += 1;
      const rows = [];
      while (i + 1 < lines.length && lines[i + 1].trim().startsWith('|')) {
        i += 1;
        rows.push(releaseCells(lines[i]));
      }
      html.push(`<table><thead><tr>${head.map(c => `<th>${releaseInline(c)}</th>`).join('')}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td>${releaseInline(c)}</td>`).join('')}</tr>`).join('')}</tbody></table>`);
      continue;
    }
    const item = trimmed.match(/^([-*+]|\d+[.)])\s+(.+)$/);
    if (item) {
      flushPara();
      const tag = /\d/.test(item[1]) ? 'ol' : 'ul';
      if (!list || list.tag !== tag) { flushList(); list = { tag, items: [] }; }
      list.items.push(item[2]);
      continue;
    }
    if (list && /^\s+/.test(line)) {
      list.items[list.items.length - 1] += ' ' + trimmed;
      continue;
    }
    flushList();
    if (!releaseInline(trimmed)) continue;
    para.push(trimmed);
  }
  flush();
  while (html.length && html[html.length - 1] === '<hr>') html.pop();
  return html.join('\n');
}

function showUpdateModal(info) {
  pendingUpdateInfo = info;
  els.updateVersion.textContent = `v${info.version}`;
  const current = document.getElementById('update-current');
  if (current) current.textContent = info.currentVersion ? `v${String(info.currentVersion).replace(/^v/, '')}` : '';
  if (info.date) {
    try {
      const d = new Date(info.date);
      els.updateDate.textContent = isNaN(d.getTime()) ? info.date : d.toLocaleDateString(window.i18n.getCurrentLocale(), { dateStyle: 'medium' });
    } catch { els.updateDate.textContent = info.date; }
    els.updateDateRow.style.display = '';
  } else {
    els.updateDateRow.style.display = 'none';
  }
  const notes = info.body ? renderReleaseNotes(info.body) : '';
  els.updateNotes.innerHTML = notes
    || `<p class="update-modal__empty">${escapeHtml(window.i18n.t('modals.update.noReleaseNotes'))}</p>`;
  const body = els.updateNotes.closest('.update-modal__body');
  if (body) body.scrollTop = 0;
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
  const githubBtn = document.getElementById('btn-update-notes-link');
  if (githubBtn) githubBtn.classList.toggle('hidden', !info.releaseUrl);

  els.updateModal.classList.remove('hidden');
  if (body) body.scrollTop = 0;
  const primary = externallyManaged ? els.btnUpdateLater : els.btnUpdateNow;
  setTimeout(() => {
    if (els.updateModal.classList.contains('hidden')) return;
    if (primary) primary.focus({ preventScroll: true });
    if (body) body.scrollTop = 0;
  }, 50);
}

function openExternalUrl(url) {
  if (!/^https?:\/\//i.test(url || '')) return;
  try {
    window.__TAURI__.shell.open(url);
  } catch (e) {
    console.error('open failed', e);
  }
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
    reportError('update_install', e);
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
  emitEvent('update_dismissed', { action: 'skip' });
  hideUpdateModal();
}

// DEV: Test function — call window.testUpdateModal() in browser console
window.testUpdateModal = function() {
  showUpdateModal({
    available: true,
    version: '2.0.0',
    currentVersion: '1.1.0',
    date: new Date().toISOString(),
    body: '### What\'s New\n- ✨ Auto-Update feature\n- 🔧 Bug fixes\n- 🚀 Performance improvements\n\nThis is a **test** update dialog.',
    releaseUrl: 'https://github.com/MCbabel/Steam-Manifest-Downloader/releases'
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
    emitEvent('download_paused', { paused: willPause, engine: state.currentEngine || 'native' });
    setTaskbarProgress(taskbar.percent);
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
  if (state.queueRunning) return;
  if (!els.shutdownAfterToggle || !els.shutdownAfterToggle.checked || !els.shutdownModal) return;
  clearInterval(state.shutdownTimer);
  state.shutdownRemaining = SHUTDOWN_DELAY_SECONDS;
  renderShutdownCountdown();
  els.btnShutdownNow.disabled = false;
  els.shutdownModal.classList.remove('hidden');
  emitEvent('shutdown_after', { action: 'countdown' });
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
  if (state.shutdownTimer) emitEvent('shutdown_after', { action: 'aborted' });
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
  emitEvent('shutdown_after', { action: 'powered_off', followup: !!followupSaved });
  try {
    await invoke('power_off_system');
  } catch (e) {
    if (followupSaved) invoke('clear_pending_followup', { downloadDir: state.downloadDir }).catch(() => {});
    emitEvent('shutdown_after', { action: 'failed' });
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
  emitEvent('followup', { action: 'offered' });
  els.btnFollowupLater.onclick = () => { emitEvent('followup', { action: 'later' }); close(); };
  els.followupModal.querySelector('.modal__backdrop').onclick = close;
  els.btnFollowupDiscard.onclick = () => {
    invoke('clear_pending_followup', { downloadDir: followup.download_dir }).catch(() => {});
    emitEvent('followup', { action: 'discarded' });
    close();
  };
  els.btnFollowupResume.onclick = () => {
    invoke('clear_pending_followup', { downloadDir: followup.download_dir }).catch(() => {});
    emitEvent('followup', { action: 'resumed' });
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
      if (addPendingSource(els.sourcesAddInput.value, els.sourcesAddError)) els.sourcesAddInput.value = '';
      updateSettingsDirty();
    });
  }
  if (els.sourcesList) {
    els.sourcesList.addEventListener('click', async (e) => {
      const btn = e.target.closest('button[data-source-idx]');
      if (!btn) return;
      const idx = parseInt(btn.dataset.sourceIdx, 10);
      if (Number.isInteger(idx)) await removePendingSource(idx);
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

  els.btnSelectAll.addEventListener('click', () => { markDepotSelectionTouched(); selectAll(); });
  els.btnDeselectAll.addEventListener('click', () => { markDepotSelectionTouched(); deselectAll(); });
  els.btnAutoSelect.addEventListener('click', () => {
    markDepotSelectionTouched();
    if (state.depotPicsList) applyRecommendedDepots(state.depotPicsList);
  });
  els.btnAutoStartCancel.addEventListener('click', cancelAutoStart);
  els.btnAutoSelectDlc.addEventListener('click', () => {
    markDepotSelectionTouched();
    state.depotIncludeDlc = !state.depotIncludeDlc;
    if (state.depotPicsList) applyRecommendedDepots(state.depotPicsList);
  });
  els.btnBack.addEventListener('click', () => goToStep(1));
  els.btnDownload.addEventListener('click', startDownload);
  els.btnQueueAdd.addEventListener('click', addToQueue);
  els.btnQueue.addEventListener('click', openQueue);
  els.btnQueueClose.addEventListener('click', closeQueue);
  els.queueModal.querySelector('.modal__backdrop').addEventListener('click', closeQueue);
  els.btnQueueStart.addEventListener('click', () => (state.queueRunning ? stopQueue() : startQueue()));
  els.btnQueueClear.addEventListener('click', async () => {
    try {
      await invoke('queue_clear');
      emitEvent('queue_action', { action: 'cleared', size: countBucket((state.queue || []).length) });
    } catch (e) {
      console.error('queue_clear failed:', e);
    }
    await refreshQueue();
  });
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
  bindProxyControls();
  bindSteamPathControls();
  bindDepotSelectionSettings();
  initSettingsLayout();
  initBugReport();

  if (els.btnCopyBuildInfo) {
    els.btnCopyBuildInfo.addEventListener('click', copyBuildInfo);
  }

  if (els.btnTelemetryAccept) els.btnTelemetryAccept.addEventListener('click', acceptTelemetry);
  if (els.btnTelemetryDecline) els.btnTelemetryDecline.addEventListener('click', declineTelemetry);

  els.btnUpdateNow.addEventListener('click', performUpdate);
  els.btnUpdateLater.addEventListener('click', () => {
    emitEvent('update_dismissed', { action: 'later' });
    hideUpdateModal();
  });
  els.btnUpdateSkip.addEventListener('click', skipUpdateVersion);
  const updateGithubBtn = document.getElementById('btn-update-notes-link');
  if (updateGithubBtn) {
    updateGithubBtn.addEventListener('click', () => {
      if (pendingUpdateInfo && pendingUpdateInfo.releaseUrl) openExternalUrl(pendingUpdateInfo.releaseUrl);
      emitEvent('update_dismissed', { action: 'github' });
    });
  }
  els.updateNotes.addEventListener('click', (e) => {
    const link = e.target.closest('a[data-external]');
    if (!link) return;
    e.preventDefault();
    openExternalUrl(link.getAttribute('href'));
  });
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
    initEmuGameData();
    initEmuNav();
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
    state.steamLibraryError = null;
  } catch (e) {
    state.steamLibrarySupported = false;
    state.steamLibraryUser = null;
    state.steamLibraryError = String(e);
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
        : (state.steamLibraryError ? window.i18n.localizeError(state.steamLibraryError) : window.i18n.t('steamLibrary.notDetected'));
    }
    if (els.btnShortcutSteamChoose) {
      els.btnShortcutSteamChoose.classList.toggle('hidden', state.steamLibrarySupported);
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
          <span class="shortcut-exe-item__name">${escapeHtml(exe.name)}${exePlatformBadge(exe, exes)}${recBadge}</span>
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
    emitEvent('library_added', { from: 'step', ok: true, restarted: !!result.steam_restarted, grid: gridCount > 0 });
    return true;
  } catch (e) {
    console.error('steam_library_add failed:', e);
    if (String(e) === 'STEAM_RUNNING') {
      showSteamRunningPrompt();
      return false;
    }
    setSteamLibraryResult('error', window.i18n.t('steamLibrary.error', { message: window.i18n.localizeError(e) }));
    emitEvent('library_added', { from: 'step', ok: false });
    reportError('steam_library', e);
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
          <span class="shortcut-exe-item__name">${escapeHtml(exe.name)}${exePlatformBadge(exe, exes)}${recBadge}</span>
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
    emitEvent('library_added', { from: 'shortcuts', ok: true, restarted: false, grid: gridCount > 0 });
    if (els.shortcutStatus) {
      const existing = els.shortcutStatus.textContent || '';
      els.shortcutStatus.textContent = existing ? existing + '\n\n' + msg : msg;
    }
  } catch (e) {
    console.error('steam_library_add (windows toggle) failed:', e);
    emitEvent('library_added', { from: 'shortcuts', ok: false });
    reportError('steam_library', e);
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

function shortcutLabel(key) {
  return `${window.i18n.t('shortcuts.ctrl')}+${key}`;
}

function applyShortcutHints() {
  const hints = [
    [els.btnHistory, 'header.history', 'H'],
    [els.btnSettings, 'header.settings', ','],
    [els.dropZone, 'shortcuts.openFile', 'O'],
  ];
  for (const [el, key, combo] of hints) {
    if (el) el.title = `${window.i18n.t(key)} (${shortcutLabel(combo)})`;
  }
}

function focusSearchField() {
  if (state.currentStep === 1) {
    switchTab('search');
    refreshSourcesUI();
    els.searchAppIdInput.focus();
    els.searchAppIdInput.select();
    return true;
  }
  if (state.currentStep === 2 && els.depotSearch) {
    els.depotSearch.focus();
    els.depotSearch.select();
    return true;
  }
  return false;
}

function initShortcuts() {
  applyShortcutHints();
  document.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey || e.repeat) return;
    const key = e.key.toLowerCase();
    const modal = topmostVisibleDialog();
    const historyOpen = modal === els.historyModal;
    let handled = false;
    if (key === 'h') {
      if (historyOpen) {
        closeHistory();
        handled = true;
      } else if (!modal) {
        openHistory();
        handled = true;
      }
    } else if (key === ',') {
      if (!modal) {
        openSettings();
        handled = true;
      }
    } else if (key === 'o') {
      if (!modal && state.currentStep === 1) {
        switchTab('upload');
        openFileDialog();
        handled = true;
      }
    } else if (key === 'f') {
      if (historyOpen) {
        els.historySearch.focus();
        els.historySearch.select();
        handled = true;
      } else if (!modal) {
        handled = focusSearchField();
      }
    }
    if (handled) {
      e.preventDefault();
      emitEvent('shortcut_key', { key: { h: 'history', ',': 'settings', o: 'open', f: 'search' }[key] || 'other' });
    }
  });
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

function updateSettingsSaveLabel() {
  if (!els.btnSettingsSave) return;
  if (state.settingsLanguage && state.settingsLanguage !== i18n.getCurrentLocale()) {
    els.btnSettingsSave.textContent = i18n.t('settings.languageRestartButton');
    els.btnSettingsSave.dataset.languageRestart = '1';
  } else {
    els.btnSettingsSave.textContent = i18n.t('settings.save');
    delete els.btnSettingsSave.dataset.languageRestart;
  }
}

function renderSettingsLanguageCards() {
  const cards = document.getElementById('settings-language-cards');
  if (!cards) return;
  renderLanguageCards(cards, state.settingsLanguage || i18n.getCurrentLocale(), (code) => {
    state.settingsLanguage = code;
    renderSettingsLanguageCards();
    updateSettingsSaveLabel();
    updateSettingsDirty();
  });
}

function resetSettingsLanguage() {
  state.settingsLanguage = i18n.getCurrentLocale();
  renderSettingsLanguageCards();
  updateSettingsSaveLabel();
}

function bindSettingsLanguageCards() {
  resetSettingsLanguage();
}

document.addEventListener('DOMContentLoaded', async () => {
  const { settings: initSettings, hasStored } = await initI18n();

  initTheme();
  initUpload();
  initEvents();
  initHistoryToolbar();
  initShortcuts();
  refreshQueue();
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
