const REMOTE_DEFAULT_LINKS = {
  achievement_watcher: 'https://github.com/Shirowwww/Achievement-Watcher-Next',
  issues: 'https://github.com/MCbabel/Steam-Manifest-Downloader/issues',
  releases: 'https://github.com/MCbabel/Steam-Manifest-Downloader/releases',
};
const REMOTE_DISMISSED_KEY = 'dismissedNotices_v1';
const OFFLINE_RECHECK_MS = 20000;

function applyRemoteLinks(links) {
  if (!links) return;
  Object.entries(REMOTE_DEFAULT_LINKS).forEach(([key, url]) => {
    const next = links[key];
    if (!next || next === url) return;
    document.querySelectorAll(`a[href="${CSS.escape(url)}"]`).forEach(a => { a.href = next; });
  });
}

function dismissedNotices() {
  try {
    return new Set(JSON.parse(localStorage.getItem(REMOTE_DISMISSED_KEY) || '[]'));
  } catch (_) {
    return new Set();
  }
}

function noticeText(notice) {
  const lang = (window.i18n.getCurrentLocale && window.i18n.getCurrentLocale()) || 'en';
  const text = notice.text || {};
  return text[lang] || text.en || Object.values(text)[0] || '';
}

function renderRemoteNotices(notices) {
  const box = document.getElementById('remote-notices');
  if (!box) return;
  box.innerHTML = '';
  const hidden = dismissedNotices();
  (notices || []).filter(n => !hidden.has(n.id)).forEach(notice => {
    const text = noticeText(notice);
    const props = { id: notice.id, level: notice.level === 'warning' ? 'warning' : 'info' };
    state.remoteShown = state.remoteShown || new Set();
    if (text && !state.remoteShown.has(notice.id)) {
      state.remoteShown.add(notice.id);
      emitEvent('remote_notice', Object.assign({ action: 'shown' }, props));
    }
    if (!text) return;
    const el = document.createElement('div');
    el.className = `app-banner app-banner--${notice.level === 'warning' ? 'warning' : 'info'}`;
    const span = document.createElement('span');
    span.textContent = text;
    el.appendChild(span);
    if (notice.url) {
      const link = document.createElement('a');
      link.href = notice.url;
      link.target = '_blank';
      link.rel = 'noopener';
      link.className = 'link app-banner__link';
      link.textContent = window.i18n.t('remote.more');
      link.addEventListener('click', () => emitEvent('remote_notice', Object.assign({ action: 'opened' }, props)));
      el.appendChild(link);
    }
    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'app-banner__close';
    close.setAttribute('aria-label', window.i18n.t('remote.dismiss'));
    close.textContent = '×';
    close.addEventListener('click', () => {
      const set = dismissedNotices();
      set.add(notice.id);
      try { localStorage.setItem(REMOTE_DISMISSED_KEY, JSON.stringify([...set])); } catch (_) {}
      emitEvent('remote_notice', Object.assign({ action: 'dismissed' }, props));
      el.remove();
    });
    el.appendChild(close);
    box.appendChild(el);
  });
}

async function refreshRemote() {
  try {
    const view = await invoke('remote_refresh');
    state.remoteView = view;
    applyRemoteLinks(view.links);
    renderRemoteNotices(view.notices);
  } catch (e) {
    console.warn('remote_refresh failed:', e);
  }
}

async function checkConnectivity() {
  let online = navigator.onLine !== false;
  if (online) {
    try { online = await invoke('connectivity_check'); } catch (_) { online = true; }
  }
  const banner = document.getElementById('offline-banner');
  const wasOffline = state.offline === true;
  state.offline = !online;
  if (banner) banner.classList.toggle('hidden', online);
  if (!online && !state.offlineReported) {
    state.offlineReported = true;
    emitEvent('offline_notice', {});
  }
  clearTimeout(state.offlineTimer);
  if (!online) state.offlineTimer = setTimeout(checkConnectivity, OFFLINE_RECHECK_MS);
  else if (wasOffline) refreshRemote();
}

async function initRemote() {
  window.addEventListener('online', checkConnectivity);
  window.addEventListener('offline', checkConnectivity);
  await checkConnectivity();
  if (!state.offline) await refreshRemote();
}
