async function performSearch() {
  hideAutocomplete(els.searchAutocomplete);
  const appIdStr = els.searchAppIdInput.value.trim();
  if (!appIdStr) return;

  const appId = parseInt(appIdStr, 10);
  if (isNaN(appId) || appId <= 0) {
    showSearchError(window.i18n.t('search.invalidAppId'));
    return;
  }

  els.searchError.classList.add('hidden');
  els.searchResults.classList.add('hidden');
  els.searchNextRow.classList.add('hidden');
  els.searchGameBanner.classList.add('hidden');
  state.selectedRepo = null;
  state.searchRepos = [];
  state.searchAppId = appId;

  els.searchLoading.classList.remove('hidden');
  els.btnSearch.disabled = true;

  fetchSearchGameInfo(appId);

  try {
    const raw = await invoke('search_repos', {
      appId: String(appId),
    });

    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;

    const repos = (raw.repos || []).map(r => ({
      name: r.repo,
      date: r.date,
      sha: r.sha,
      type: r.type || 'unknown',
      source: r.source || r.type || 'unknown'
    }));
    state.searchRepos = repos;
    emitSearchOutcome(repos.length, raw.sourceProbe);

    if (repos.length === 0) {
      showSearchError(window.i18n.t('search.noResults'));
      reportError('search', null, 'search.noResults');
      return;
    }

    renderRepoList(repos);
    els.searchResults.classList.remove('hidden');
  } catch (error) {
    els.searchLoading.classList.add('hidden');
    els.btnSearch.disabled = false;
    emitSearchOutcome(0, 'error');
    showSearchError(window.i18n.localizeError(error));
    reportError('search', error);
  }
}

function emitSearchOutcome(repoCount, probe) {
  const parts = String(probe || '').split(':');
  emitEvent('search_performed', {
    found: repoCount > 0,
    repo_count: repoCount,
    source_probe: parts[0] || null,
    probe_class: parts[1] || null,
  });
}

function showSearchError(message) {
  els.searchError.textContent = message;
  els.searchError.classList.remove('hidden');
}

async function fetchSearchGameInfo(appId) {
  els.searchGameBanner.classList.add('hidden');

  try {
    const info = await invoke('get_steam_app_info', { appId: String(appId) });

    if (info) {
      const { name, headerImage, shortDescription } = info;

      if (headerImage) {
        els.searchGameImage.src = headerImage;
        els.searchGameImage.alt = name || window.i18n.t('common.gameCover');
        state.headerImage = headerImage;
      }

      if (name) {
        els.searchGameName.textContent = name;
        state.gameName = name;
      }

      if (shortDescription) {
        els.searchGameDescription.textContent = shortDescription;
      }

      els.searchGameBanner.classList.remove('hidden');
    }
  } catch (e) {
  }
}

function renderRepoList(repos) {
  els.repoList.innerHTML = '';

  repos.forEach((repo, index) => {
    const card = document.createElement('div');
    card.className = 'repo-card';
    card.dataset.repoIndex = index;

    const displayName = repoDisplayName(repo);
    const badgeText = repoBadgeText(repo);
    const dateHtml = repo.date
      ? `<div class="repo-card__date">${window.i18n.t('search.repoUpdated')}: ${formatRepoDate(repo.date)}</div>`
      : '';

    card.innerHTML = `
      <div class="repo-card__radio"></div>
      <div class="repo-card__info">
        <div class="repo-card__name">${escapeHtml(displayName)}</div>
        ${dateHtml}
      </div>
      <span class="repo-card__badge repo-card__badge--archive">${escapeHtml(badgeText)}</span>
    `;
    card.addEventListener('click', () => selectRepo(index));
    els.repoList.appendChild(card);
  });

  if (repos.length === 1) {
    selectRepo(0);
  }

  if (autoRedownloadPending && repos.length > 0) {
    autoRedownloadPending = false;
    selectRepo(0);
    proceedFromSearch();
  }
}

function repoDisplayName(repo) {
  switch (repo.type) {
    case 'hubcap':
      return window.i18n.t('search.repoNameHubcap');
    case 'remote':
      return window.i18n.t('search.repoNameRemote');
    default:
      return repo.name || repo.type || 'Unknown';
  }
}

function repoBadgeText(repo) {
  switch (repo.type) {
    case 'hubcap':
      return window.i18n.t('search.repoBadgeHubcap');
    case 'remote':
      return window.i18n.t('search.repoBadgeRemote');
    default:
      return repo.source || repo.type || window.i18n.t('search.sourceFallback');
  }
}

function formatRepoDate(dateStr) {
  try {
    const d = new Date(dateStr);
    if (isNaN(d.getTime())) return dateStr;
    return d.toLocaleString(window.i18n.getCurrentLocale(), { dateStyle: 'medium', timeStyle: 'short' });
  } catch {
    return dateStr;
  }
}

function selectRepo(index) {
  $$('.repo-card').forEach(c => c.classList.remove('selected'));

  state.selectedRepo = state.searchRepos[index];

  const card = els.repoList.querySelector(`[data-repo-index="${index}"]`);
  if (card) card.classList.add('selected');

  els.searchNextRow.classList.remove('hidden');
}

async function proceedFromSearch() {
  if (!state.selectedRepo) return;

  const repo = state.selectedRepo;
  const appId = state.searchAppId;

  els.searchNextRow.classList.add('hidden');
  els.manifestLoading.classList.remove('hidden');
  els.searchError.classList.add('hidden');

  try {
    const mRaw = await invoke('get_repo_manifests', {
      appId: String(appId),
      repo: repo.name,
      sha: repo.sha || null,
    });

    const depots = (mRaw.manifests || []).map(m => ({
      depotId: String(m.depot_id),
      manifestId: m.manifest_id || 'N/A',
      depotKey: m.depot_key || null,
      sizeBytes: m.size_bytes || null
    }));

    state.searchRepo = repo.name;
    state.searchSha = repo.sha;
    state.searchKeyVdfKeys = mRaw.depot_keys || null;

    els.manifestLoading.classList.add('hidden');

    if (depots.length === 0) {
      showSearchError(window.i18n.t('errors.noManifests'));
      reportError('manifests', null, 'errors.noManifests');
      els.searchNextRow.classList.remove('hidden');
      return;
    }

    state.parsedData = {
      mainAppId: appId,
      depots: depots
    };

    showSelectionStep();
  } catch (error) {
    els.manifestLoading.classList.add('hidden');
    showSearchError(window.i18n.localizeError(error));
    reportError('manifests', error);
    els.searchNextRow.classList.remove('hidden');
  }
}
