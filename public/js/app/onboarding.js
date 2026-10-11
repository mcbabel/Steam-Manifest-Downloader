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

async function showWhatsNewIfNeeded(hasStored) {
  let info = null;
  try {
    info = await invoke('get_whats_new', { firstStart: !hasStored });
  } catch (e) {
    console.warn('get_whats_new failed:', e);
  }
  if (!info || !info.body) return;
  const notes = renderReleaseNotes(info.body);
  if (!notes) return;
  const modal = document.getElementById('whats-new-modal');
  if (!modal) return;
  document.getElementById('whats-new-title').textContent = window.i18n.t('whatsNew.title', { version: `v${info.version}` });
  const date = document.getElementById('whats-new-date');
  date.textContent = '';
  if (info.date) {
    const d = new Date(info.date);
    if (!isNaN(d.getTime())) date.textContent = d.toLocaleDateString(window.i18n.getCurrentLocale(), { dateStyle: 'medium' });
  }
  const box = document.getElementById('whats-new-notes');
  box.innerHTML = notes;
  emitEvent('whats_new_shown');
  await new Promise((resolve) => {
    const close = () => {
      modal.classList.add('hidden');
      resolve();
    };
    document.getElementById('btn-whats-new-close').addEventListener('click', close, { once: true });
    modal.querySelector('.modal__backdrop').addEventListener('click', close, { once: true });
    const github = document.getElementById('btn-whats-new-github');
    github.classList.toggle('hidden', !info.releaseUrl);
    github.onclick = () => openExternalUrl(info.releaseUrl);
    box.addEventListener('click', (e) => {
      const link = e.target.closest('a[data-external]');
      if (!link) return;
      e.preventDefault();
      openExternalUrl(link.getAttribute('href'));
    });
    modal.classList.remove('hidden');
    const body = box.closest('.update-modal__body');
    setTimeout(() => {
      if (body) body.scrollTop = 0;
      document.getElementById('btn-whats-new-close').focus({ preventScroll: true });
    }, 50);
  });
}
