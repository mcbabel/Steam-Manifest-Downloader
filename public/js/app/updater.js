const SKIPPED_VERSION_KEY = 'skippedUpdateVersion';

async function checkForUpdates() {
  try {
    const enabled = await invoke('get_auto_update_enabled');
    if (!enabled) return;

    const result = await invoke('check_for_updates');

    emitEvent('update_checked', { available: !!result.available, failed: !!result.error });
    if (result.error) reportError('update_check', result.error);

    if (result.error) {
      console.error('[AutoUpdate] Error:', result.error);
    }
    if (!result.available) return;

    // Check if user has skipped this version
    const skipped = localStorage.getItem(SKIPPED_VERSION_KEY);
    if (skipped === result.version) return;

    showUpdateModal(result);
  } catch (e) {
    console.error('[AutoUpdate] Check failed:', e);
  }
}

const RELEASE_REPO = 'MCbabel/Steam-Manifest-Downloader';

function releaseInline(text) {
  const codes = [];
  let out = text.replace(/`([^`]+)`/g, (_, c) => {
    codes.push(c);
    return `\u0000${codes.length - 1}\u0000`;
  });
  const links = [];
  const keep = (html) => {
    links.push(html);
    return `\u0001${links.length - 1}\u0001`;
  };
  const safeUrl = (u) => (/^https?:\/\//i.test(u) ? u : null);
  out = out.replace(/!\[[^\]]*\]\([^)]*\)/g, '');
  out = out.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, label, url) => {
    const u = safeUrl(url);
    return u ? keep(`<a href="${escapeHtml(u)}" data-external>${escapeHtml(label)}</a>`) : label;
  });
  out = out.replace(/https?:\/\/[^\s<>()]+[^\s<>().,:;!?]/g, (u) => keep(`<a href="${escapeHtml(u)}" data-external>${escapeHtml(u.replace(/^https?:\/\//, ''))}</a>`));
  out = out.replace(/\b([A-Za-z0-9-]+\/[A-Za-z0-9._-]+)#(\d+)\b/g, (m, repo, n) => keep(`<a class="update-modal__ref" href="https://github.com/${repo}/issues/${n}" data-external>${escapeHtml(m)}</a>`));
  out = out.replace(/(^|[^\w&/])#(\d+)\b/g, (m, pre, n) => pre + keep(`<a class="update-modal__ref" href="https://github.com/${RELEASE_REPO}/issues/${n}" data-external>#${n}</a>`));
  out = out.replace(/(^|[^\w/])@([A-Za-z0-9](?:[A-Za-z0-9-]{0,38}))\b/g, (m, pre, user) => pre + keep(`<a class="update-modal__ref" href="https://github.com/${user}" data-external>@${escapeHtml(user)}</a>`));
  out = escapeHtml(out);
  out = out.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  out = out.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  out = out.replace(/(^|[^*\w])\*([^*\s][^*]*?)\*(?![*\w])/g, '$1<em>$2</em>');
  out = out.replace(/(^|[^_\w])_([^_\s][^_]*?)_(?![_\w])/g, '$1<em>$2</em>');
  out = out.replace(/\u0001(\d+)\u0001/g, (_, i) => links[Number(i)]);
  out = out.replace(/\u0000(\d+)\u0000/g, (_, i) => `<code>${escapeHtml(codes[Number(i)])}</code>`);
  return out.trim();
}

function releaseCells(line) {
  return line.trim().replace(/^\||\|$/g, '').split('|').map(c => c.trim());
}

function renderReleaseNotes(md) {
  const lines = String(md || '').replace(/\r\n?/g, '\n').split('\n');
  const html = [];
  let para = [];
  let list = null;
  let skipLevel = 0;
  const flushPara = () => {
    if (para.length) html.push(`<p>${releaseInline(para.join(' '))}</p>`);
    para = [];
  };
  const flushList = () => {
    if (list) html.push(`<${list.tag}>${list.items.map(i => `<li>${releaseInline(i)}</li>`).join('')}</${list.tag}>`);
    list = null;
  };
  const flush = () => { flushPara(); flushList(); };
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    const heading = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (heading && skipLevel && heading[1].length <= skipLevel) skipLevel = 0;
    if (skipLevel) continue;
    if (heading && /\bdownloads?\s*$/i.test(heading[2].replace(/[^\w\s]/g, '').trim()) && heading[1].length >= 2) {
      flush();
      skipLevel = heading[1].length;
      continue;
    }
    if (!trimmed) { flush(); continue; }
    if (heading) {
      flush();
      const level = Math.min(Math.max(heading[1].length, 2), 4);
      html.push(`<h${level}>${releaseInline(heading[2])}</h${level}>`);
      continue;
    }
    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      flush();
      if (html.length && !html[html.length - 1].startsWith('<hr')) html.push('<hr>');
      continue;
    }
    if (trimmed.startsWith('|') && i + 1 < lines.length && /^\|?\s*:?-{2,}/.test(lines[i + 1].trim())) {
      flush();
      const head = releaseCells(trimmed);
      i += 1;
      const rows = [];
      while (i + 1 < lines.length && lines[i + 1].trim().startsWith('|')) {
        i += 1;
        rows.push(releaseCells(lines[i]));
      }
      html.push(`<table><thead><tr>${head.map(c => `<th>${releaseInline(c)}</th>`).join('')}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td>${releaseInline(c)}</td>`).join('')}</tr>`).join('')}</tbody></table>`);
      continue;
    }
    const item = trimmed.match(/^([-*+]|\d+[.)])\s+(.+)$/);
    if (item) {
      flushPara();
      const tag = /\d/.test(item[1]) ? 'ol' : 'ul';
      if (!list || list.tag !== tag) { flushList(); list = { tag, items: [] }; }
      list.items.push(item[2]);
      continue;
    }
    if (list && /^\s+/.test(line)) {
      list.items[list.items.length - 1] += ' ' + trimmed;
      continue;
    }
    flushList();
    if (!releaseInline(trimmed)) continue;
    para.push(trimmed);
  }
  flush();
  while (html.length && html[html.length - 1] === '<hr>') html.pop();
  return html.join('\n');
}

function showUpdateModal(info) {
  pendingUpdateInfo = info;
  els.updateVersion.textContent = `v${info.version}`;
  const current = document.getElementById('update-current');
  if (current) current.textContent = info.currentVersion ? `v${String(info.currentVersion).replace(/^v/, '')}` : '';
  if (info.date) {
    try {
      const d = new Date(info.date);
      els.updateDate.textContent = isNaN(d.getTime()) ? info.date : d.toLocaleDateString(window.i18n.getCurrentLocale(), { dateStyle: 'medium' });
    } catch { els.updateDate.textContent = info.date; }
    els.updateDateRow.style.display = '';
  } else {
    els.updateDateRow.style.display = 'none';
  }
  const notes = info.body ? renderReleaseNotes(info.body) : '';
  els.updateNotes.innerHTML = notes
    || `<p class="update-modal__empty">${escapeHtml(window.i18n.t('modals.update.noReleaseNotes'))}</p>`;
  const body = els.updateNotes.closest('.update-modal__body');
  if (body) body.scrollTop = 0;
  els.updateProgressWrap.classList.add('hidden');
  els.updateActions.style.display = '';
  els.btnUpdateNow.disabled = false;

  const externallyManaged = info.installMethod && info.installMethod !== 'self';
  const systemHint = document.getElementById('update-system-hint');
  if (systemHint) {
    systemHint.classList.toggle('hidden', !externallyManaged);
    if (externallyManaged) {
      renderUpdateCommands(info.installMethod);
    }
  }
  els.btnUpdateNow.classList.toggle('hidden', externallyManaged);
  els.btnUpdateSkip.classList.toggle('hidden', externallyManaged);
  const githubBtn = document.getElementById('btn-update-notes-link');
  if (githubBtn) githubBtn.classList.toggle('hidden', !info.releaseUrl);

  els.updateModal.classList.remove('hidden');
  if (body) body.scrollTop = 0;
  const primary = externallyManaged ? els.btnUpdateLater : els.btnUpdateNow;
  setTimeout(() => {
    if (els.updateModal.classList.contains('hidden')) return;
    if (primary) primary.focus({ preventScroll: true });
    if (body) body.scrollTop = 0;
  }, 50);
}

function openExternalUrl(url) {
  if (!/^https?:\/\//i.test(url || '')) return;
  try {
    window.__TAURI__.shell.open(url);
  } catch (e) {
    console.error('open failed', e);
  }
}

function updateCommandsFor(method) {
  switch (method) {
    case 'flatpak':
      return [{ label: '', cmd: 'flatpak update de.mcbabel.SteamManifestDownloader' }];
    case 'snap':
      return [{ label: '', cmd: 'sudo snap refresh steam-manifest-downloader' }];
    case 'system':
    default:
      return [
        { label: i18n.t('modals.update.aurBinLabel'), cmd: 'paru -Syu steam-manifest-downloader-bin' },
        { label: i18n.t('modals.update.aurSourceLabel'), cmd: 'paru -Syu steam-manifest-downloader' },
      ];
  }
}

function renderUpdateCommands(method) {
  const list = document.getElementById('update-system-cmd-list');
  if (!list) return;
  const commands = updateCommandsFor(method);
  const copyLabel = i18n.t('modals.update.copyCmd');
  list.innerHTML = commands.map((c, i) => {
    const labelHtml = c.label
      ? `<div class="update-system-cmd__label">${escapeHtml(c.label)}</div>`
      : '';
    return `
      <div class="update-system-cmd__row">
        ${labelHtml}
        <div class="update-system-cmd__inner">
          <pre><code>${escapeHtml(c.cmd)}</code></pre>
          <button type="button" class="btn btn--outline btn--small update-system-cmd__copy" data-cmd-idx="${i}">${escapeHtml(copyLabel)}</button>
        </div>
      </div>
    `;
  }).join('');

  list.querySelectorAll('.update-system-cmd__copy').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const idx = parseInt(btn.dataset.cmdIdx, 10);
      const cmd = commands[idx]?.cmd;
      if (!cmd) return;
      try {
        await navigator.clipboard.writeText(cmd);
        const original = btn.textContent;
        btn.textContent = window.i18n.t('common.copied');
        setTimeout(() => { btn.textContent = original; }, 1500);
      } catch {}
    });
  });
}

function hideUpdateModal() {
  els.updateModal.classList.add('hidden');
}

let pendingUpdateInfo = null;

async function performUpdate() {
  if (!pendingUpdateInfo || !pendingUpdateInfo.installerUrl) {
    // No direct installer — open release page in browser
    if (pendingUpdateInfo && pendingUpdateInfo.releaseUrl) {
      window.__TAURI__.shell.open(pendingUpdateInfo.releaseUrl);
    }
    hideUpdateModal();
    return;
  }

  els.btnUpdateNow.disabled = true;
  els.btnUpdateLater.style.display = 'none';
  els.btnUpdateSkip.style.display = 'none';
  els.btnUpdateNow.textContent = window.i18n.t('modals.update.downloading');
  els.updateProgressWrap.classList.remove('hidden');
  els.updateProgressText.textContent = window.i18n.t('modals.update.installerDownloading');
  els.updateProgressFill.style.width = '100%';
  els.updateProgressFill.classList.add('progress-bar__fill--indeterminate');

  emitEvent('update_installed');
  try {
    await invoke('install_update', { installerUrl: pendingUpdateInfo.installerUrl });
    // App will exit — this line may not be reached
  } catch (e) {
    console.error('[AutoUpdate] Install failed:', e);
    reportError('update_install', e);
    els.updateProgressText.textContent = window.i18n.t('modals.update.failed', { message: window.i18n.localizeError(e) });
    els.updateProgressFill.classList.remove('progress-bar__fill--indeterminate');
    els.updateProgressFill.style.width = '0%';
    els.btnUpdateNow.textContent = window.i18n.t('common.retry');
    els.btnUpdateNow.disabled = false;
    els.btnUpdateLater.style.display = '';
  }
}

function skipUpdateVersion() {
  const version = els.updateVersion.textContent.replace(/^v/, '');
  localStorage.setItem(SKIPPED_VERSION_KEY, version);
  emitEvent('update_dismissed', { action: 'skip' });
  hideUpdateModal();
}

// DEV: Test function — call window.testUpdateModal() in browser console
window.testUpdateModal = function() {
  showUpdateModal({
    available: true,
    version: '2.0.0',
    currentVersion: '1.1.0',
    date: new Date().toISOString(),
    body: '### What\'s New\n- ✨ Auto-Update feature\n- 🔧 Bug fixes\n- 🚀 Performance improvements\n\nThis is a **test** update dialog.',
    releaseUrl: 'https://github.com/MCbabel/Steam-Manifest-Downloader/releases'
  });
};
