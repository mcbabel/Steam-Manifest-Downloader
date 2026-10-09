// Lightweight i18n for the Tauri webview. Loads a single JSON locale file,
// resolves dotted keys (e.g. "modals.cancel.title"), and walks the DOM to
// apply translations via data-i18n* attributes.

const AVAILABLE_LOCALES = [
  { code: 'en', label: 'English' },
  { code: 'de', label: 'Deutsch' },
];

const FALLBACK = 'en';

let current = null;
let fallback = null;
let currentCode = FALLBACK;

function detectBrowserLocale() {
  const langs = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || ''];
  for (const raw of langs) {
    const two = (raw || '').slice(0, 2).toLowerCase();
    if (AVAILABLE_LOCALES.some(l => l.code === two)) return two;
  }
  return FALLBACK;
}

async function fetchLocale(code) {
  const res = await fetch(`locales/${code}.json`, { cache: 'no-cache' });
  if (!res.ok) throw new Error(`Failed to load locale '${code}': ${res.status}`);
  return res.json();
}

async function loadLocale(code) {
  const target = AVAILABLE_LOCALES.some(l => l.code === code) ? code : FALLBACK;
  current = await fetchLocale(target);
  currentCode = target;
  if (target !== FALLBACK) {
    try { fallback = await fetchLocale(FALLBACK); } catch { fallback = null; }
  } else {
    fallback = null;
  }
  backendPatterns = null;
  document.documentElement.setAttribute('lang', target);
}

function lookup(tree, key) {
  if (!tree) return undefined;
  return key.split('.').reduce((acc, part) => (acc && typeof acc === 'object') ? acc[part] : undefined, tree);
}

function t(key, params) {
  let value = lookup(current, key);
  if (value === undefined) value = lookup(fallback, key);
  if (value === undefined) return key;
  if (typeof value !== 'string') return key;
  if (params) {
    value = value.replace(/\{(\w+)\}/g, (_, name) => (params[name] !== undefined ? String(params[name]) : `{${name}}`));
  }
  return value;
}

let backendPatterns = null;

function buildBackendPatterns() {
  const source = fallback && fallback.backend;
  if (!source) return [];
  return Object.entries(source)
    .filter(([, text]) => typeof text === 'string')
    .map(([key, text]) => {
      const parts = text.split(/\{\d+\}/);
      const body = parts.map(p => p.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('([\\s\\S]*?)');
      return { key, re: new RegExp(`^${body}$`), weight: parts.join('').length };
    })
    .sort((a, b) => b.weight - a.weight);
}

function localizeError(text, depth = 0) {
  const s = String(text ?? '');
  if (!s || currentCode === FALLBACK || depth > 4) return s;
  if (!backendPatterns) backendPatterns = buildBackendPatterns();
  for (const p of backendPatterns) {
    const m = p.re.exec(s);
    if (!m) continue;
    const target = lookup(current, `backend.${p.key}`);
    if (typeof target !== 'string') return s;
    return target.replace(/\{(\d+)\}/g, (_, i) => localizeError(m[Number(i) + 1] ?? '', depth + 1));
  }
  return s;
}

function errorKey(text) {
  const s = String(text ?? '');
  if (!s) return null;
  if (!backendPatterns) backendPatterns = buildBackendPatterns();
  const hit = backendPatterns.find(p => p.re.test(s));
  return hit ? `backend.${hit.key}` : null;
}

function applyTranslations(root) {
  const scope = root || document;

  scope.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.getAttribute('data-i18n');
    if (!key) return;
    el.textContent = t(key);
  });

  scope.querySelectorAll('[data-i18n-html]').forEach(el => {
    const key = el.getAttribute('data-i18n-html');
    if (!key) return;
    el.innerHTML = t(key);
  });

  scope.querySelectorAll('[data-i18n-attr]').forEach(el => {
    const spec = el.getAttribute('data-i18n-attr');
    if (!spec) return;
    for (const pair of spec.split(',')) {
      const [attr, key] = pair.split('=').map(s => s.trim());
      if (!attr || !key) continue;
      el.setAttribute(attr, t(key));
    }
  });
}

function getAvailableLocales() {
  return AVAILABLE_LOCALES.slice();
}

function getCurrentLocale() {
  return currentCode;
}

window.i18n = { loadLocale, t, localizeError, errorKey, applyTranslations, detectBrowserLocale, getAvailableLocales, getCurrentLocale, FALLBACK };
