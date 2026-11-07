const EMU_EXTRA_TABS = ['saves', 'profile', 'network', 'mods'];

function emuExtraTargets() {
  const list = state.emuEditMode ? (state.emuEditTargets || []) : getSelectedEmuTargets();
  return list.map(t => t.path).filter(Boolean);
}

function setEmuExtraStatus(id, kind, text) {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = text || '';
  el.classList.toggle('hidden', !text);
  el.classList.toggle('dd-path__hint--error', kind === 'error');
  el.classList.toggle('dd-path__hint--success', kind === 'success');
}

function emuExtraError(e) {
  return window.i18n.localizeError(String((e && e.message) || e || ''));
}

function armConfirm(btn, confirmKey, action) {
  const label = btn.textContent;
  let armed = false;
  let timer = null;
  btn.addEventListener('click', async () => {
    if (!armed) {
      armed = true;
      btn.textContent = window.i18n.t(confirmKey);
      btn.classList.add('btn--danger');
      timer = setTimeout(() => {
        armed = false;
        btn.textContent = label;
        btn.classList.remove('btn--danger');
      }, 4000);
      return;
    }
    clearTimeout(timer);
    armed = false;
    btn.textContent = window.i18n.t(btn.dataset.i18n);
    btn.classList.remove('btn--danger');
    await action();
  });
}

function emuGamePatched() {
  return !!(state.emuEditMode || state.emuApplyComplete || (state.emulatorScan || []).some(f => f.is_patched));
}

function syncEmuAfterPatch() {
  const patched = emuGamePatched();
  const savesTab = document.getElementById('emu-tab-saves');
  if (savesTab) {
    savesTab.classList.toggle('hidden', !patched);
    if (!patched && savesTab.classList.contains('is-active')) selectEmuTab('files');
  }
  const lobby = document.getElementById('emu-lobby-section');
  if (lobby && !patched) lobby.classList.add('hidden');
  const later = document.getElementById('emu-lobby-later');
  if (later) later.classList.toggle('hidden', patched || !emuExtraTargets().length);
}

function emuExtraPanelState(section) {
  const panel = document.getElementById(`emu-panel-${section}`);
  if (!panel) return false;
  const has = emuExtraTargets().length > 0;
  const needs = panel.querySelector('.emu-extra__needs');
  if (needs) needs.classList.toggle('hidden', has);
  panel.querySelectorAll('.emu-section').forEach(s => s.classList.toggle('hidden', !has));
  syncEmuAfterPatch();
  return has;
}

function refreshEmuExtraTab(section) {
  if (!EMU_EXTRA_TABS.includes(section)) return;
  if (!emuExtraPanelState(section)) return;
  if (section === 'saves') {
    refreshEmuSaves();
    refreshEmuAchievements();
  } else if (section === 'profile') {
    refreshEmuProfile();
  } else if (section === 'network') {
    refreshEmuNetwork();
  } else if (section === 'mods') {
    refreshEmuMods();
  }
}

function emuPlaceLabel(place) {
  return window.i18n.t(`emulator.saves.place.${place}`) || place;
}

async function refreshEmuSaves() {
  const list = document.getElementById('emu-saves-list');
  const target = emuExtraTargets()[0];
  const appId = currentAppIdForEmu();
  if (!list || !target) return;
  let folders = [];
  try { folders = await invoke('emu_save_folders', { target, appId: String(appId || '') }) || []; } catch (_) {}
  const keep = state.emuSaveFolder && folders.find(f => f.path === state.emuSaveFolder.path);
  state.emuSaveFolder = keep || folders.find(f => f.exists) || folders[0] || null;
  list.innerHTML = '';
  if (!folders.length) {
    list.innerHTML = `<p class="dd-path__hint">${escapeHtml(window.i18n.t('emulator.saves.none'))}</p>`;
  }
  folders.forEach(f => {
    const row = document.createElement('button');
    row.type = 'button';
    row.className = 'emu-extra-row' + (state.emuSaveFolder && state.emuSaveFolder.path === f.path ? ' is-active' : '');
    const meta = f.exists
      ? window.i18n.t('emulator.saves.meta', {
        files: f.files,
        size: formatBytes(f.size_bytes) || '0 B',
        date: f.modified ? new Date(f.modified * 1000).toLocaleString(window.i18n.getCurrentLocale()) : '–',
      })
      : window.i18n.t('emulator.saves.empty');
    row.innerHTML = `
      <span class="emu-extra-row__main">
        <span class="emu-extra-row__title" title="${escapeHtml(f.path)}">${escapeHtml(f.path)}</span>
        <span class="emu-extra-row__meta">${escapeHtml(meta)}</span>
      </span>
      <span class="emu-file-tag">${escapeHtml(emuPlaceLabel(f.portable ? 'portable' : f.place))}</span>`;
    row.addEventListener('click', () => {
      state.emuSaveFolder = f;
      refreshEmuSaves();
    });
    list.appendChild(row);
  });
  const exists = !!(state.emuSaveFolder && state.emuSaveFolder.exists);
  const open = document.getElementById('btn-emu-saves-open');
  const backup = document.getElementById('btn-emu-saves-backup');
  const restore = document.getElementById('btn-emu-saves-restore');
  if (open) open.disabled = !exists;
  if (backup) backup.disabled = !exists;
  if (restore) restore.disabled = !state.emuSaveFolder;
  setEmuBadge('saves', exists ? String(state.emuSaveFolder.files) : '', null);
}

function emuBackupName() {
  const base = String(state.gameName || currentAppIdForEmu() || 'game').replace(/[^\w.-]+/g, '_').slice(0, 60);
  const day = new Date().toISOString().slice(0, 10);
  return `${base}-saves-${day}.zip`;
}

async function backupEmuSaves() {
  const folder = state.emuSaveFolder;
  if (!folder || !folder.exists) return;
  const dest = await window.__TAURI__.dialog.save({
    defaultPath: emuBackupName(),
    filters: [{ name: 'ZIP', extensions: ['zip'] }],
  });
  if (!dest) return;
  setEmuExtraStatus('emu-saves-status', 'busy', window.i18n.t('emulator.saves.working'));
  try {
    const count = await invoke('emu_backup_saves', { folder: folder.path, dest });
    setEmuExtraStatus('emu-saves-status', 'success', window.i18n.t('emulator.saves.backedUp', { count }));
    emitEvent('emu_saves', { action: 'backup', ok: true });
  } catch (e) {
    setEmuExtraStatus('emu-saves-status', 'error', emuExtraError(e));
    emitEvent('emu_saves', { action: 'backup', ok: false });
  }
}

async function restoreEmuSaves() {
  const folder = state.emuSaveFolder;
  if (!folder) return;
  const archive = await window.__TAURI__.dialog.open({
    multiple: false,
    filters: [{ name: 'ZIP', extensions: ['zip'] }],
  });
  if (!archive) return;
  setEmuExtraStatus('emu-saves-status', 'busy', window.i18n.t('emulator.saves.working'));
  try {
    const count = await invoke('emu_restore_saves', { folder: folder.path, archive });
    setEmuExtraStatus('emu-saves-status', 'success', window.i18n.t('emulator.saves.restored', { count }));
    emitEvent('emu_saves', { action: 'restore', ok: true });
  } catch (e) {
    setEmuExtraStatus('emu-saves-status', 'error', emuExtraError(e));
    emitEvent('emu_saves', { action: 'restore', ok: false });
  }
  refreshEmuSaves();
  refreshEmuAchievements();
}

function emuGameLanguage() {
  const settings = gatherEmuSettings() || {};
  return settings.language || 'english';
}

async function refreshEmuAchievements() {
  const list = document.getElementById('emu-ach-list');
  const empty = document.getElementById('emu-ach-empty');
  const count = document.getElementById('emu-ach-count');
  const target = emuExtraTargets()[0];
  if (!list || !target) return;
  let view = { rows: [], stats_files: 0 };
  try {
    view = await invoke('emu_achievements', { target, appId: String(currentAppIdForEmu() || ''), lang: emuGameLanguage() }) || view;
  } catch (_) {}
  const rows = view.rows || [];
  const earned = rows.filter(r => r.earned).length;
  if (count) count.textContent = rows.length ? window.i18n.t('emulator.achievements.count', { earned, total: rows.length }) : '';
  if (empty) {
    empty.textContent = window.i18n.t('emulator.achievements.none');
    empty.classList.toggle('hidden', rows.length > 0);
  }
  const resetAll = document.getElementById('btn-emu-ach-reset');
  const resetStats = document.getElementById('btn-emu-stats-reset');
  if (resetAll) resetAll.disabled = earned === 0;
  if (resetStats) resetStats.disabled = !view.stats_files;
  list.innerHTML = '';
  rows.forEach(r => {
    const row = document.createElement('div');
    row.className = 'emu-ach' + (r.earned ? ' emu-ach--earned' : '');
    const when = r.earned && r.earned_time
      ? new Date(r.earned_time * 1000).toLocaleDateString(window.i18n.getCurrentLocale())
      : '';
    const desc = r.hidden && !r.earned ? window.i18n.t('emulator.achievements.hidden') : r.description;
    row.innerHTML = `
      <span class="emu-ach__icon" aria-hidden="true"></span>
      <span class="emu-ach__text">
        <span class="emu-ach__name">${escapeHtml(r.display_name)}</span>
        <span class="emu-ach__desc">${escapeHtml(desc || '')}</span>
      </span>
      <span class="emu-ach__state">${escapeHtml(r.earned
        ? (when ? window.i18n.t('emulator.achievements.earnedOn', { date: when }) : window.i18n.t('emulator.achievements.earned'))
        : window.i18n.t('emulator.achievements.locked'))}</span>`;
    if (r.earned) {
      const lock = document.createElement('button');
      lock.type = 'button';
      lock.className = 'btn btn--small btn--outline';
      lock.textContent = window.i18n.t('emulator.achievements.lock');
      lock.addEventListener('click', async () => {
        lock.disabled = true;
        try {
          await invoke('emu_reset_achievements', { target, appId: String(currentAppIdForEmu() || ''), names: [r.name] });
          emitEvent('emu_achievements', { action: 'lock_one' });
        } catch (e) {
          setEmuExtraStatus('emu-ach-status', 'error', emuExtraError(e));
        }
        refreshEmuAchievements();
      });
      row.appendChild(lock);
    }
    list.appendChild(row);
    if (r.icon) {
      invoke('emu_image', { path: r.icon }).then(url => {
        const icon = row.querySelector('.emu-ach__icon');
        if (url && icon) icon.style.backgroundImage = `url("${url}")`;
      }).catch(() => {});
    }
  });
}

async function resetAllEmuAchievements() {
  const target = emuExtraTargets()[0];
  if (!target) return;
  try {
    const n = await invoke('emu_reset_achievements', { target, appId: String(currentAppIdForEmu() || ''), names: null });
    setEmuExtraStatus('emu-ach-status', 'success', window.i18n.t('emulator.achievements.lockedCount', { count: n }));
    emitEvent('emu_achievements', { action: 'lock_all' });
  } catch (e) {
    setEmuExtraStatus('emu-ach-status', 'error', emuExtraError(e));
  }
  refreshEmuAchievements();
}

async function resetEmuStats() {
  const target = emuExtraTargets()[0];
  if (!target) return;
  try {
    const n = await invoke('emu_reset_stats', { target, appId: String(currentAppIdForEmu() || '') });
    setEmuExtraStatus('emu-ach-status', 'success', window.i18n.t('emulator.achievements.statsCleared', { count: n }));
    emitEvent('emu_achievements', { action: 'clear_stats' });
  } catch (e) {
    setEmuExtraStatus('emu-ach-status', 'error', emuExtraError(e));
  }
  refreshEmuAchievements();
}

const EMU_ASSET_FILTERS = {
  avatar: [{ name: 'PNG / JPG', extensions: ['png', 'jpg', 'jpeg'] }],
  font: [{ name: 'TrueType / OpenType', extensions: ['ttf', 'otf', 'ttc'] }],
  achievement: [{ name: 'WAV', extensions: ['wav'] }],
  friend: [{ name: 'WAV', extensions: ['wav'] }],
};

async function refreshEmuProfile() {
  const target = emuExtraTargets()[0];
  if (!target) return;
  let assets = {};
  try { assets = await invoke('emu_overlay_assets', { target }) || {}; } catch (_) {}
  state.emuOverlayAssets = assets;
  const preview = document.getElementById('emu-avatar-preview');
  const clear = document.getElementById('btn-emu-avatar-clear');
  if (clear) clear.classList.toggle('hidden', !assets.avatar);
  if (preview) {
    preview.style.backgroundImage = '';
    if (assets.avatar) {
      invoke('emu_image', { path: assets.avatar }).then(url => {
        if (url) preview.style.backgroundImage = `url("${url}")`;
      }).catch(() => {});
    }
  }
  const values = {
    font: assets.font,
    achievement: assets.achievement_sound ? window.i18n.t('emulator.profile.ownSound') : null,
    friend: assets.friend_sound ? window.i18n.t('emulator.profile.ownSound') : null,
  };
  document.querySelectorAll('#emu-panel-profile .emu-asset-row').forEach(row => {
    const kind = row.dataset.asset;
    const value = row.querySelector('.emu-asset-row__value');
    if (value) {
      value.textContent = values[kind]
        || window.i18n.t(kind === 'font' ? 'emulator.profile.defaultFont' : 'emulator.profile.noSound');
    }
    const rm = row.querySelector('[data-asset-clear]');
    if (rm) rm.classList.toggle('hidden', !values[kind]);
  });
  const count = [assets.avatar, assets.font, assets.achievement_sound, assets.friend_sound].filter(Boolean).length;
  setEmuBadge('profile', count ? String(count) : '', count ? 'on' : null);
  fillEmuSystemFonts();
}

function emuFontFileName(path) {
  const base = String(path || '').split(/[\\/]/).pop() || '';
  return base.replace(/[^A-Za-z0-9._-]/g, '_').replace(/^\.+/, '');
}

async function fillEmuSystemFonts() {
  const select = document.getElementById('emu-font-system');
  if (!select) return;
  if (!state.emuSystemFonts) {
    if (!state.emuSystemFontsLoading) {
      state.emuSystemFontsLoading = invoke('emu_installed_fonts')
        .then(list => { state.emuSystemFonts = Array.isArray(list) ? list : []; })
        .catch(() => { state.emuSystemFonts = []; })
        .finally(() => { state.emuSystemFontsLoading = null; });
    }
    if (!select._picker) enhanceSelect(select);
    await state.emuSystemFontsLoading;
  }
  const fonts = state.emuSystemFonts || [];
  const current = (state.emuOverlayAssets || {}).font || '';
  select.innerHTML = '';
  const head = document.createElement('option');
  head.value = '';
  head.textContent = window.i18n.t(fonts.length ? 'emulator.profile.systemFontsPick' : 'emulator.profile.systemFontsNone');
  select.appendChild(head);
  let selected = '';
  fonts.forEach(f => {
    const opt = document.createElement('option');
    opt.value = f.path;
    opt.textContent = f.name;
    if (!selected && current && emuFontFileName(f.path) === current) selected = f.path;
    select.appendChild(opt);
  });
  select.value = selected;
  select.disabled = !fonts.length;
  rebuildSelectPicker(select);
  if (select._picker) select._picker.button.disabled = !fonts.length;
}

async function setEmuSystemFont(path) {
  const targets = emuExtraTargets();
  if (!targets.length || !path) return;
  try {
    await invoke('emu_set_font', { targets, src: path });
    setEmuExtraStatus('emu-profile-status', 'success', window.i18n.t('emulator.profile.saved'));
    emitEvent('emu_profile', { asset: 'font', action: 'set', source: 'system', ok: true });
  } catch (e) {
    setEmuExtraStatus('emu-profile-status', 'error', emuExtraError(e));
    emitEvent('emu_profile', { asset: 'font', action: 'set', source: 'system', ok: false });
  }
  refreshEmuProfile();
}

async function pickEmuAsset(kind) {
  const targets = emuExtraTargets();
  if (!targets.length) return;
  const src = await window.__TAURI__.dialog.open({ multiple: false, filters: EMU_ASSET_FILTERS[kind] });
  if (!src) return;
  try {
    if (kind === 'avatar') {
      await invoke('emu_set_avatar', { targets, src });
      const box = document.querySelector('#step-emulator [data-emu-key="enable_account_avatar"]');
      if (box) box.checked = true;
    } else if (kind === 'font') {
      await invoke('emu_set_font', { targets, src });
    } else {
      await invoke('emu_set_sound', { targets, kind, src });
    }
    setEmuExtraStatus('emu-profile-status', 'success', window.i18n.t('emulator.profile.saved'));
    emitEvent('emu_profile', { asset: kind, action: 'set', ok: true });
  } catch (e) {
    setEmuExtraStatus('emu-profile-status', 'error', emuExtraError(e));
    emitEvent('emu_profile', { asset: kind, action: 'set', ok: false });
  }
  refreshEmuProfile();
}

async function clearEmuAsset(kind) {
  const targets = emuExtraTargets();
  if (!targets.length) return;
  try {
    if (kind === 'avatar') {
      await invoke('emu_remove_avatar', { targets });
      const box = document.querySelector('#step-emulator [data-emu-key="enable_account_avatar"]');
      if (box) box.checked = false;
    } else if (kind === 'font') {
      await invoke('emu_remove_font', { targets });
    } else {
      await invoke('emu_remove_sound', { targets, kind });
    }
    setEmuExtraStatus('emu-profile-status', null, '');
    emitEvent('emu_profile', { asset: kind, action: 'remove', ok: true });
  } catch (e) {
    setEmuExtraStatus('emu-profile-status', 'error', emuExtraError(e));
  }
  refreshEmuProfile();
}

async function refreshEmuNetwork() {
  const target = emuExtraTargets()[0];
  const box = document.getElementById('emu-broadcasts');
  if (!target || !box) return;
  let list = [];
  try { list = await invoke('emu_broadcasts', { target }) || []; } catch (_) {}
  box.value = list.join('\n');
  setEmuBadge('network', list.length ? String(list.length) : '', list.length ? 'on' : null);
}

async function saveEmuBroadcasts() {
  const targets = emuExtraTargets();
  const box = document.getElementById('emu-broadcasts');
  if (!targets.length || !box) return;
  try {
    const bad = await invoke('emu_set_broadcasts', { targets, list: box.value.split('\n') }) || [];
    setEmuExtraStatus('emu-broadcasts-status', bad.length ? 'error' : 'success', bad.length
      ? window.i18n.t('emulator.network.rejected', { list: bad.join(', ') })
      : window.i18n.t('emulator.network.saved'));
    const count = box.value.split(/[\s,;]+/).filter(Boolean).length - bad.length;
    emitEvent('emu_network', { action: 'broadcasts', count: countBucket(Math.max(0, count)) });
  } catch (e) {
    setEmuExtraStatus('emu-broadcasts-status', 'error', emuExtraError(e));
  }
  refreshEmuNetwork();
}

async function startEmuLobbyConnect(allowDownload = false) {
  const target = emuExtraTargets()[0];
  if (!target || !state.downloadDir) return;
  setEmuExtraStatus('emu-lobby-status', 'busy', window.i18n.t('emulator.network.lobbyPreparing'));
  let path;
  try {
    path = await invoke('emu_lobby_connect', { gameDir: state.downloadDir, target, allowDownload });
  } catch (e) {
    const confirmInfo = parseEmuDownloadConfirm(e);
    if (confirmInfo) {
      state.emuPendingDownload = null;
      state.emuDownloadRetry = () => startEmuLobbyConnect(true);
      showEmuDownloadConfirm(confirmInfo);
      setEmuExtraStatus('emu-lobby-status', null, '');
      return;
    }
    setEmuExtraStatus('emu-lobby-status', 'error', String(e) === 'GBE_TOOLS_MISSING'
      ? window.i18n.t('emulator.network.lobbyMissing')
      : emuExtraError(e));
    emitEvent('emu_network', { action: 'lobby', ok: false });
    return;
  }
  try {
    await invoke('emu_launch_console', { path });
    setEmuExtraStatus('emu-lobby-status', 'success', window.i18n.t('emulator.network.lobbyStarted'));
    emitEvent('emu_network', { action: 'lobby', ok: true });
  } catch (e) {
    const text = String(e) === 'WINDOWS_TOOL_ON_LINUX'
      ? window.i18n.t('emulator.network.lobbyProton', { path: relativizeEmuPath(path) })
      : emuExtraError(e);
    setEmuExtraStatus('emu-lobby-status', String(e) === 'WINDOWS_TOOL_ON_LINUX' ? 'success' : 'error', text);
    emitEvent('emu_network', { action: 'lobby', ok: String(e) === 'WINDOWS_TOOL_ON_LINUX' });
  }
}

async function refreshEmuMods() {
  const target = emuExtraTargets()[0];
  const list = document.getElementById('emu-mods-list');
  const empty = document.getElementById('emu-mods-empty');
  if (!target || !list) return;
  let mods = [];
  try { mods = await invoke('emu_mods', { target }) || []; } catch (_) {}
  list.innerHTML = '';
  if (empty) empty.classList.toggle('hidden', mods.length > 0);
  mods.forEach(m => {
    const row = document.createElement('div');
    row.className = 'emu-extra-row emu-mod';
    row.innerHTML = `
      <span class="emu-mod__img" aria-hidden="true"></span>
      <span class="emu-extra-row__main">
        <span class="emu-extra-row__title">${escapeHtml(m.title)}</span>
        <span class="emu-extra-row__meta">${escapeHtml(window.i18n.t('emulator.mods.meta', { id: m.id, files: m.files, size: formatBytes(m.size_bytes) || '0 B' }))}</span>
      </span>`;
    const rm = document.createElement('button');
    rm.type = 'button';
    rm.className = 'btn btn--small btn--outline';
    rm.textContent = window.i18n.t('emulator.mods.remove');
    rm.addEventListener('click', async () => {
      rm.disabled = true;
      try {
        await invoke('emu_remove_mod', { targets: emuExtraTargets(), id: m.id });
        emitEvent('emu_mod', { action: 'remove', ok: true });
      } catch (e) {
        setEmuExtraStatus('emu-mods-status', 'error', emuExtraError(e));
      }
      refreshEmuMods();
    });
    row.appendChild(rm);
    list.appendChild(row);
    if (m.preview) {
      invoke('emu_image', { path: m.preview }).then(url => {
        const img = row.querySelector('.emu-mod__img');
        if (url && img) img.style.backgroundImage = `url("${url}")`;
      }).catch(() => {});
    }
  });
  setEmuBadge('mods', mods.length ? String(mods.length) : '', mods.length ? 'on' : null);
}

async function addEmuMod(directory) {
  const targets = emuExtraTargets();
  const input = document.getElementById('emu-mod-id');
  const id = input ? input.value.trim() : '';
  if (!targets.length) return;
  if (!id) {
    setEmuExtraStatus('emu-mods-status', 'error', window.i18n.t('emulator.mods.needId'));
    if (input) input.focus();
    return;
  }
  const src = await window.__TAURI__.dialog.open({ multiple: false, directory: !!directory });
  if (!src) return;
  setEmuExtraStatus('emu-mods-status', 'busy', window.i18n.t('emulator.mods.adding'));
  try {
    const mod = await invoke('emu_add_mod', { targets, appId: String(currentAppIdForEmu() || ''), id, src });
    const note = mod.other_app ? '\n' + window.i18n.t('emulator.mods.otherApp', { app: mod.other_app }) : '';
    setEmuExtraStatus('emu-mods-status', mod.other_app ? 'error' : 'success', window.i18n.t('emulator.mods.added', { title: mod.title }) + note);
    if (input) input.value = '';
    emitEvent('emu_mod', { action: 'add', ok: true, details: mod.title !== mod.id, other_app: !!mod.other_app });
  } catch (e) {
    setEmuExtraStatus('emu-mods-status', 'error', emuExtraError(e));
    emitEvent('emu_mod', { action: 'add', ok: false });
  }
  refreshEmuMods();
}

function emuColdClientExe() {
  if (state.emuEditMode) return null;
  const chosen = document.querySelector('input[name="emu-method"]:checked');
  if (!chosen || chosen.value !== 'coldclient') return null;
  const select = document.getElementById('emu-cold-exe');
  return select && select.value ? select.value : null;
}

async function refreshEmuMethod() {
  const section = document.getElementById('emu-method-section');
  if (!section) return;
  const windows = (state.emulatorScan || []).some(f => f.platform === 'windows' && !f.loader);
  const show = !state.emuEditMode && windows;
  section.classList.toggle('hidden', !show);
  if (!show) return;
  const select = document.getElementById('emu-cold-exe');
  if (!select || !state.downloadDir || select.dataset.dir === state.downloadDir) return;
  select.dataset.dir = state.downloadDir;
  let exes = [];
  try {
    const r = await invoke('detect_executables', { downloadDir: state.downloadDir });
    exes = ((r && r.executables) || [])
      .filter(e => e.platform === 'windows' && !/^steamclient_loader/i.test(e.name || ''));
  } catch (_) {}
  select.innerHTML = '';
  exes.forEach(e => {
    const opt = document.createElement('option');
    opt.value = e.path;
    opt.textContent = relativizeEmuPath(e.path);
    if (e.recommended) opt.selected = true;
    select.appendChild(opt);
  });
  rebuildSelectPicker(select);
  const cold = section.querySelector('input[value="coldclient"]');
  if (cold) cold.disabled = exes.length === 0;
}

function syncEmuMethodUi() {
  const cold = !!document.querySelector('input[name="emu-method"][value="coldclient"]:checked');
  const options = document.getElementById('emu-cold-options');
  if (options) options.classList.toggle('hidden', !cold);
  const variant = document.getElementById('emu-variant-section');
  if (variant) variant.classList.toggle('emu-section--muted', cold);
  updateEmuOverview();
}

function noteEmuColdClient(results, fresh) {
  const box = document.getElementById('emu-cold-steam');
  const running = results.some(r => r.shortcut === 'steam_running');
  if (box) box.classList.toggle('hidden', !running);
  const ok = results.some(r => r.success && (fresh || r.shortcut));
  const links = results.reduce((n, r) => n + (r.links || 0), 0);
  if (links) showToast(window.i18n.t('emulator.method.linksDone', { count: links }), 'success');
  if (fresh) {
    emitEvent('coldclient_used', {
      action: 'setup',
      ok,
      shortcut: results.map(r => r.shortcut).find(Boolean) || 'none',
      links: links > 0,
    });
  }
}

async function closeSteamForColdClient() {
  const btn = document.getElementById('btn-emu-cold-steam');
  if (btn) btn.disabled = true;
  try {
    const states = await invoke('emu_coldclient_shortcut', { gameDir: state.downloadDir, back: false, closeSteam: true }) || [];
    const ok = states.includes('updated');
    showToast(window.i18n.t(ok ? 'emulator.method.shortcutDone' : 'emulator.method.shortcutFailed'), ok ? 'success' : 'error');
    emitEvent('coldclient_used', { action: 'shortcut', ok });
    if (ok) document.getElementById('emu-cold-steam').classList.add('hidden');
  } catch (e) {
    showToast(emuExtraError(e), 'error');
  }
  if (btn) btn.disabled = false;
}

function resetEmuExtrasView() {
  state.emuSaveFolder = null;
  EMU_EXTRA_TABS.forEach(t => setEmuBadge(t, '', null));
  ['emu-saves-status', 'emu-ach-status', 'emu-profile-status', 'emu-broadcasts-status', 'emu-lobby-status', 'emu-mods-status']
    .forEach(id => setEmuExtraStatus(id, null, ''));
  const steamBox = document.getElementById('emu-cold-steam');
  if (steamBox) steamBox.classList.add('hidden');
  const replace = document.querySelector('input[name="emu-method"][value="replace"]');
  if (replace) replace.checked = true;
  const select = document.getElementById('emu-cold-exe');
  if (select) delete select.dataset.dir;
  syncEmuMethodUi();
  syncEmuAfterPatch();
}

function initEmuExtras() {
  const step = document.getElementById('step-emulator');
  if (!step) return;
  const baseSelect = selectEmuTab;
  selectEmuTab = function (section) {
    baseSelect(section);
    refreshEmuExtraTab(section);
  };
  const on = (id, fn) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('click', fn);
  };
  on('btn-emu-saves-open', () => {
    if (state.emuSaveFolder) invoke('open_folder', { path: state.emuSaveFolder.path }).catch(e => showToast(emuExtraError(e), 'error'));
  });
  on('btn-emu-saves-backup', backupEmuSaves);
  on('btn-emu-saves-restore', restoreEmuSaves);
  const resetAll = document.getElementById('btn-emu-ach-reset');
  if (resetAll) armConfirm(resetAll, 'emulator.achievements.confirm', resetAllEmuAchievements);
  const resetStats = document.getElementById('btn-emu-stats-reset');
  if (resetStats) armConfirm(resetStats, 'emulator.achievements.confirm', resetEmuStats);
  on('btn-emu-avatar', () => pickEmuAsset('avatar'));
  on('btn-emu-avatar-clear', () => clearEmuAsset('avatar'));
  step.querySelectorAll('[data-asset-pick]').forEach(b => b.addEventListener('click', () => pickEmuAsset(b.dataset.assetPick)));
  step.querySelectorAll('[data-asset-clear]').forEach(b => b.addEventListener('click', () => clearEmuAsset(b.dataset.assetClear)));
  const fontSelect = document.getElementById('emu-font-system');
  if (fontSelect) fontSelect.addEventListener('change', () => setEmuSystemFont(fontSelect.value));
  on('btn-emu-broadcasts', saveEmuBroadcasts);
  on('btn-emu-lobby', () => startEmuLobbyConnect(false));
  on('btn-emu-mod-folder', () => addEmuMod(true));
  on('btn-emu-mod-file', () => addEmuMod(false));
  on('btn-emu-cold-steam', closeSteamForColdClient);
  step.querySelectorAll('input[name="emu-method"]').forEach(r => r.addEventListener('change', syncEmuMethodUi));
}
