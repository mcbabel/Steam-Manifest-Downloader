const WELCOME_CHOICES = ['upload', 'search', 'patch'];

function welcomeLocationText() {
  return defaultDownloadDir || window.i18n.t('welcome.locationUnset');
}

async function chooseWelcomeLocation() {
  try {
    const { open } = window.__TAURI__.dialog;
    const selected = await open({ directory: true, multiple: false, title: window.i18n.t('select.chooseLocationTitle') });
    if (!selected) return;
    const settings = await invoke('get_settings');
    settings.download_location = selected;
    await invoke('save_settings', { settings });
    defaultDownloadDir = selected;
    if (els.downloadDirInput) els.downloadDirInput.value = selected;
    const path = document.getElementById('welcome-location-path');
    if (path) path.textContent = selected;
    emitEvent('welcome_choice', { choice: 'location' });
  } catch (e) {
    console.error('Failed to choose download location:', e);
  }
}

function applyWelcomeChoice(choice) {
  if (choice === 'search') {
    switchTab('search');
    refreshSourcesUI();
    setTimeout(() => {
      const input = els.searchAppIdInput;
      if (input && !input.closest('.hidden')) input.focus();
    }, 50);
  } else if (choice === 'patch') {
    switchTab('patchOnly');
  } else if (choice === 'upload') {
    switchTab('upload');
  }
}

async function showWelcomeIfNeeded(hasStored) {
  if (hasStored) return;
  const modal = document.getElementById('welcome-modal');
  if (!modal) return;
  const path = document.getElementById('welcome-location-path');
  if (path) path.textContent = welcomeLocationText();
  emitEvent('welcome_shown');
  await new Promise((resolve) => {
    const finish = (choice) => {
      modal.classList.add('hidden');
      emitEvent('welcome_choice', { choice });
      applyWelcomeChoice(choice);
      resolve();
    };
    modal.querySelectorAll('.welcome__choice').forEach((btn) => {
      btn.addEventListener('click', () => finish(WELCOME_CHOICES.includes(btn.dataset.choice) ? btn.dataset.choice : 'closed'), { once: true });
    });
    const close = document.getElementById('btn-welcome-close');
    if (close) close.addEventListener('click', () => finish('closed'), { once: true });
    const backdrop = modal.querySelector('.modal__backdrop');
    if (backdrop) backdrop.addEventListener('click', () => finish('closed'), { once: true });
    const location = document.getElementById('btn-welcome-location');
    if (location) location.addEventListener('click', chooseWelcomeLocation);
    modal.querySelectorAll('a[href^="http"]').forEach((a) => a.addEventListener('click', (e) => {
      e.preventDefault();
      openExternalUrl(a.href);
    }));
    modal.classList.remove('hidden');
  });
}
