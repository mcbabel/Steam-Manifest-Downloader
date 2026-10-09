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
