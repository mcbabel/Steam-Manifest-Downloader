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
