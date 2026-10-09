function initTheme() {
  const saved = localStorage.getItem('theme') || 'dark';
  document.documentElement.setAttribute('data-theme', saved);
  updateThemeButton(saved);
}

function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme') || 'dark';
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', next);
  localStorage.setItem('theme', next);
  updateThemeButton(next);
  emitEvent('theme_toggled', { to: next });
}

function updateThemeButton(theme) {
  if (els.btnThemeToggle) {
    els.btnThemeToggle.innerHTML = theme === 'dark' ? ICONS.moon : ICONS.sun;
    const themeLabel = window.i18n.t(theme === 'dark' ? 'header.themeToLight' : 'header.themeToDark');
    els.btnThemeToggle.title = themeLabel;
    els.btnThemeToggle.setAttribute('aria-label', themeLabel);
  }
}

const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

const modalFocusReturn = new WeakMap();

function getFocusable(modal) {
  return Array.from(modal.querySelectorAll(FOCUSABLE_SELECTOR))
    .filter((el) => el.offsetParent !== null || el === document.activeElement);
}

function shortcutLabel(key) {
  return `${window.i18n.t('shortcuts.ctrl')}+${key}`;
}

function applyShortcutHints() {
  const hints = [
    [els.btnHistory, 'header.history', 'H'],
    [els.btnSettings, 'header.settings', ','],
    [els.dropZone, 'shortcuts.openFile', 'O'],
  ];
  for (const [el, key, combo] of hints) {
    if (el) el.title = `${window.i18n.t(key)} (${shortcutLabel(combo)})`;
  }
}

function focusSearchField() {
  if (state.currentStep === 1) {
    switchTab('search');
    refreshSourcesUI();
    els.searchAppIdInput.focus();
    els.searchAppIdInput.select();
    return true;
  }
  if (state.currentStep === 2 && els.depotSearch) {
    els.depotSearch.focus();
    els.depotSearch.select();
    return true;
  }
  return false;
}

function initShortcuts() {
  applyShortcutHints();
  document.addEventListener('keydown', (e) => {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey || e.repeat) return;
    const key = e.key.toLowerCase();
    const modal = topmostVisibleDialog();
    const historyOpen = modal === els.historyModal;
    let handled = false;
    if (key === 'h') {
      if (historyOpen) {
        closeHistory();
        handled = true;
      } else if (!modal) {
        openHistory();
        handled = true;
      }
    } else if (key === ',') {
      if (!modal) {
        openSettings();
        handled = true;
      }
    } else if (key === 'o') {
      if (!modal && state.currentStep === 1) {
        switchTab('upload');
        openFileDialog();
        handled = true;
      }
    } else if (key === 'f') {
      if (historyOpen) {
        els.historySearch.focus();
        els.historySearch.select();
        handled = true;
      } else if (!modal) {
        handled = focusSearchField();
      }
    }
    if (handled) {
      e.preventDefault();
      emitEvent('shortcut_key', { key: { h: 'history', ',': 'settings', o: 'open', f: 'search' }[key] || 'other' });
    }
  });
}

function topmostVisibleDialog() {
  const dialogs = document.querySelectorAll('[role="dialog"]');
  for (let i = dialogs.length - 1; i >= 0; i--) {
    if (!dialogs[i].classList.contains('hidden')) return dialogs[i];
  }
  return null;
}

function setupModalA11y() {
  document.querySelectorAll('[role="dialog"]').forEach((modal) => {
    modal.setAttribute('aria-hidden', modal.classList.contains('hidden') ? 'true' : 'false');

    const observer = new MutationObserver(() => {
      const isHidden = modal.classList.contains('hidden');
      modal.setAttribute('aria-hidden', isHidden ? 'true' : 'false');

      if (!isHidden) {
        modalFocusReturn.set(modal, document.activeElement);
        const focusables = getFocusable(modal);
        if (focusables.length > 0) {
          requestAnimationFrame(() => focusables[0].focus());
        }
      } else {
        const returnTo = modalFocusReturn.get(modal);
        if (returnTo && typeof returnTo.focus === 'function') {
          returnTo.focus();
        }
        modalFocusReturn.delete(modal);
      }
    });

    observer.observe(modal, { attributes: true, attributeFilter: ['class'] });
  });

  document.addEventListener('keydown', (e) => {
    const modal = topmostVisibleDialog();
    if (!modal) return;

    if (e.key === 'Escape') {
      const cancelBtn = modal.querySelector('[data-modal-cancel]')
        || modal.querySelector('.btn--outline')
        || modal.querySelector('button');
      if (cancelBtn) {
        e.preventDefault();
        cancelBtn.click();
      }
      return;
    }

    if (e.key === 'Tab') {
      const focusables = getFocusable(modal);
      if (focusables.length === 0) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  });
}

const LANGUAGE_FLAGS = {
  en: `<svg viewBox="0 0 60 30" preserveAspectRatio="xMidYMid slice">
    <rect width="60" height="30" fill="#012169"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#fff" stroke-width="6"/>
    <path d="M0,0 L60,30 M60,0 L0,30" stroke="#C8102E" stroke-width="2.5"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#fff" stroke-width="10"/>
    <path d="M30,0 v30 M0,15 h60" stroke="#C8102E" stroke-width="6"/>
  </svg>`,
  de: `<svg viewBox="0 0 5 3" preserveAspectRatio="none">
    <rect width="5" height="1" y="0" fill="#000"/>
    <rect width="5" height="1" y="1" fill="#DD0000"/>
    <rect width="5" height="1" y="2" fill="#FFCE00"/>
  </svg>`,
};

function renderLanguageCards(container, activeCode, onSelect) {
  if (!container) return;
  container.innerHTML = '';
  for (const locale of i18n.getAvailableLocales()) {
    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'language-card' + (locale.code === activeCode ? ' is-active' : '');
    card.setAttribute('data-lang', locale.code);
    card.setAttribute('aria-label', locale.label);
    card.innerHTML = `
      <span class="language-card__flag" aria-hidden="true">${LANGUAGE_FLAGS[locale.code] || ''}</span>
      <span class="language-card__label">${escapeHtml(locale.label)}</span>
    `;
    card.addEventListener('click', () => onSelect(locale.code));
    container.appendChild(card);
  }
}

async function initI18n() {
  let settings = null;
  try {
    settings = await invoke('get_settings');
  } catch (e) {
    console.error('Failed to read settings during i18n init:', e);
  }
  const stored = settings && settings.language ? settings.language : '';
  const code = stored || i18n.detectBrowserLocale();
  try {
    await i18n.loadLocale(code);
  } catch (e) {
    console.error('Failed to load locale, falling back to en:', e);
    await i18n.loadLocale(i18n.FALLBACK);
  }
  i18n.applyTranslations(document);
  return { settings, hasStored: !!stored };
}

async function showLanguagePickerIfNeeded(initSettings, hasStored) {
  if (hasStored) return;
  const picker = document.getElementById('language-picker');
  const cards = document.getElementById('language-picker-cards');
  if (!picker || !cards) return;

  await new Promise((resolve) => {
    renderLanguageCards(cards, i18n.getCurrentLocale(), async (code) => {
      try {
        const fresh = await invoke('get_settings');
        fresh.language = code;
        await invoke('save_settings', { settings: fresh });
      } catch (e) {
        console.error('Failed to save language choice:', e);
      }
      if (code !== i18n.getCurrentLocale()) {
        try { await i18n.loadLocale(code); } catch {}
        i18n.applyTranslations(document);
      }
      picker.classList.add('hidden');
      resolve();
    });

    picker.classList.remove('hidden');
    const active = cards.querySelector('.language-card.is-active') || cards.querySelector('.language-card');
    if (active) active.focus();
  });
}
