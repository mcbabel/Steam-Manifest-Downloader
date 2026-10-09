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
