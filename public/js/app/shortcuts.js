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
