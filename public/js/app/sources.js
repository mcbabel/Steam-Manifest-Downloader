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
