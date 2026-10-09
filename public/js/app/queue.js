function updateQueueBadge(queue) {
  if (Array.isArray(queue)) state.queue = queue;
  const n = (state.queue || []).length;
  const badge = document.getElementById('queue-badge');
  if (badge) {
    badge.textContent = String(n);
    badge.classList.toggle('hidden', n === 0);
  }
  if (els.btnQueue) {
    const label = window.i18n.t('queue.title');
    els.btnQueue.title = n ? `${label} (${n})` : label;
  }
}

async function refreshQueue() {
  try {
    updateQueueBadge(await invoke('queue_list'));
  } catch (e) {
    console.error('queue_list failed:', e);
  }
  if (els.queueModal && !els.queueModal.classList.contains('hidden')) renderQueue();
}

async function addToQueue() {
  const data = state.parsedData;
  if (!data) return;
  const selectedDepots = data.depots.filter(d => state.selectedDepots.has(d.depotId));
  if (selectedDepots.length === 0) return;
  if (!(await confirmFreeSpace(selectedDepots, activeUpdateDir(data.mainAppId)))) return;
  const mhApiKey = rememberDownloadInputs();
  const repairManifests = activeRepair(data.mainAppId);
  const depots = collectDepotConfigs(selectedDepots, repairManifests);
  const size = selectedDownloadBytes(selectedDepots);
  const item = {
    id: '',
    app_id: String(data.mainAppId),
    game_name: state.gameName || null,
    header_image: state.headerImage || null,
    depot_count: depots.length,
    size_bytes: size > 0 ? size : null,
    depots,
    config: buildDownloadConfig(data, depots, mhApiKey, repairManifests),
    added_at: new Date().toISOString(),
  };
  try {
    updateQueueBadge(await invoke('queue_add', { item }));
    emitEvent('queue_action', { action: 'added', size: countBucket((state.queue || []).length) });
  } catch (e) {
    alert(window.i18n.localizeError(e));
    return;
  }
  const name = item.game_name || `App ${item.app_id}`;
  showToast(window.i18n.t('queue.added', { name, count: state.queue.length }), 'success');
  resetApp();
}

function renderQueue() {
  const list = document.getElementById('queue-list');
  const running = document.getElementById('queue-running');
  const queue = state.queue || [];
  const current = state.queueRunning && state.queueCurrent;
  running.classList.toggle('hidden', !current);
  if (current) {
    running.textContent = window.i18n.t('queue.running', { name: current.game_name || `App ${current.app_id}` });
  }
  if (queue.length === 0) {
    list.innerHTML = `<div class="history-empty">${escapeHtml(window.i18n.t('queue.empty'))}</div>`;
  } else {
    list.innerHTML = queue.map((item, i) => `
      <div class="queue-item" data-id="${escapeHtml(item.id)}">
        <span class="queue-item__pos">${i + 1}</span>
        <div class="queue-item__info">
          <div class="queue-item__name">${escapeHtml(item.game_name || `App ${item.app_id}`)}</div>
          <div class="queue-item__meta">App ${escapeHtml(item.app_id)} · ${escapeHtml(window.i18n.t('queue.depots', { count: item.depot_count }))}${item.size_bytes ? ` · ${escapeHtml(formatBytes(item.size_bytes) || '')}` : ''}${item.config && item.config.updateDir ? ` · ${escapeHtml(window.i18n.t(item.config.repair ? 'queue.kindRepair' : 'queue.kindUpdate'))}` : ''}</div>
        </div>
        <div class="queue-item__actions">
          <button class="btn btn--small btn--outline queue-up" title="${escapeHtml(window.i18n.t('queue.up'))}" aria-label="${escapeHtml(window.i18n.t('queue.up'))}"${i === 0 ? ' disabled' : ''}>↑</button>
          <button class="btn btn--small btn--outline queue-down" title="${escapeHtml(window.i18n.t('queue.down'))}" aria-label="${escapeHtml(window.i18n.t('queue.down'))}"${i === queue.length - 1 ? ' disabled' : ''}>↓</button>
          <button class="btn btn--small btn--outline queue-remove" title="${escapeHtml(window.i18n.t('queue.remove'))}" aria-label="${escapeHtml(window.i18n.t('queue.remove'))}">${ICONS.trash}</button>
        </div>
      </div>`).join('');
  }
  list.querySelectorAll('.queue-item').forEach((row) => {
    const id = row.dataset.id;
    const act = async (cmd, args) => {
      try {
        updateQueueBadge(await invoke(cmd, args));
      } catch (e) {
        console.error(`${cmd} failed:`, e);
      }
      renderQueue();
    };
    row.querySelector('.queue-up').addEventListener('click', () => act('queue_move', { id, offset: -1 }));
    row.querySelector('.queue-down').addEventListener('click', () => act('queue_move', { id, offset: 1 }));
    row.querySelector('.queue-remove').addEventListener('click', () => act('queue_remove', { id }));
  });
  const busy = !!state.jobId || state.queueRunning;
  els.btnQueueStart.disabled = queue.length === 0 || busy;
  els.btnQueueStart.textContent = window.i18n.t(state.queueRunning ? 'queue.stop' : 'queue.start');
  if (state.queueRunning) els.btnQueueStart.disabled = false;
  els.btnQueueClear.disabled = queue.length === 0;
}

async function openQueue() {
  await refreshQueue();
  renderQueue();
  els.queueModal.classList.remove('hidden');
}

function closeQueue() {
  els.queueModal.classList.add('hidden');
}

async function startQueue() {
  if (state.jobId || state.queueRunning) return;
  state.queueRunning = true;
  emitEvent('queue_action', { action: 'started', size: countBucket((state.queue || []).length) });
  closeQueue();
  await runNextQueued();
}

function stopQueue() {
  if (state.queueRunning) emitEvent('queue_action', { action: 'stopped', size: countBucket((state.queue || []).length) });
  state.queueRunning = false;
  clearTimeout(state.queueTimer);
  state.queueTimer = null;
  state.queueCurrent = null;
  showToast(window.i18n.t('queue.stopped'));
  renderQueue();
}

async function runNextQueued() {
  state.queueTimer = null;
  if (!state.queueRunning) return;
  if (state.jobId) {
    state.queueTimer = setTimeout(runNextQueued, 2000);
    return;
  }
  let next = null;
  try {
    next = await invoke('queue_take_next');
  } catch (e) {
    console.error('queue_take_next failed:', e);
  }
  await refreshQueue();
  if (!next) {
    state.queueRunning = false;
    state.queueCurrent = null;
    return;
  }
  state.queueCurrent = next;
  closeHistory();
  state.emuStandalone = false;
  setEmuEditMode(false);
  cleanupProgressListener();
  state.jobId = null;
  state.parsedData = { mainAppId: next.app_id, depots: next.depots || [] };
  state.selectedDepots = new Set((next.depots || []).map((d) => String(d.depotId)));
  state.gameName = next.game_name || null;
  state.headerImage = next.header_image || null;
  state.updateDir = next.config.updateDir || null;
  state.updateAppId = next.config.updateDir ? String(next.app_id) : null;
  state.repairManifests = null;
  state.repairRunning = !!next.config.repair;
  await runDownload(next.config, next.depots || [], true);
}

function queueAfterDownload(outcome) {
  if (!state.queueRunning) return;
  if (outcome === 'cancelled') {
    stopQueue();
    return;
  }
  (async () => {
    await refreshQueue();
    const remaining = (state.queue || []).length;
    if (remaining === 0) {
      state.queueRunning = false;
      state.queueCurrent = null;
      showToast(window.i18n.t('queue.finished'), 'success', 6000);
      maybeScheduleShutdown();
      return;
    }
    await commitPendingHistory();
    if (outcome !== 'failed') await savePendingFollowup();
    appendTerminalLine(window.i18n.t('queue.nextIn', { seconds: QUEUE_NEXT_DELAY_SECONDS }), 'info');
    state.queueTimer = setTimeout(runNextQueued, QUEUE_NEXT_DELAY_SECONDS * 1000);
  })();
}
