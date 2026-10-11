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
  initFeedbackPrompt();

  bindSettingsLanguageCards();
  await showLanguagePickerIfNeeded(initSettings, hasStored);
  await initTelemetryConsent();
  await showWelcomeIfNeeded(hasStored);
  await showWhatsNewIfNeeded(hasStored);
  await showPendingFollowupIfAny();
  setTimeout(checkForUpdates, 1500);
});
