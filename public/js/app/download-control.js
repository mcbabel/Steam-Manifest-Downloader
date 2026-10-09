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
