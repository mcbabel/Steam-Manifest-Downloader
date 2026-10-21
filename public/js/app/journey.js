const journey = {
  active: false,
  bound: false,
  screen: null,
  visits: {},
  lastClick: { name: null, at: 0, count: 0 },
  hoverTimer: null,
  timer: null,
};

const INTERACTIVE = 'button, a[href], input, select, textarea, label, summary, [role="button"], [role="tab"], [role="option"], [role="switch"], [role="checkbox"], [data-track], .depot-item, .history-entry, .language-card, .segmented__option, .repo-item, .history-sort__option';
const STEP_ORDER = { source: 1, select: 2, download: 3, shortcut: 4, emulator: 5, library: 6 };

function journeyLabel(text) {
  return String(text || '')
    .replace(/\d{3,}/g, 'n')
    .replace(/[^A-Za-z0-9_.-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 64) || 'unknown';
}

function journeyName(el) {
  if (!el || el === document.body) return 'body';
  if (el.dataset && el.dataset.track) return journeyLabel(el.dataset.track);
  if (el.id) return journeyLabel(el.id);
  const cls = Array.from(el.classList || []).find(c => c.includes('__') || c.includes('--')) || (el.classList && el.classList[0]) || el.tagName.toLowerCase();
  const owner = el.parentElement && el.parentElement.closest('[id]');
  return journeyLabel(owner ? `${owner.id}.${cls}` : cls);
}

function journeyScreenName() {
  const dialog = typeof topmostVisibleDialog === 'function' ? topmostVisibleDialog() : null;
  if (dialog) {
    const base = (dialog.id || 'dialog').replace(/-modal$/, '');
    if (dialog === els.settingsModal) {
      const tab = dialog.querySelector('.settings__tab.is-active');
      return `settings.${tab ? tab.dataset.section : 'general'}`;
    }
    return base;
  }
  switch (state.currentStep) {
    case 1: return `source.${state.mode || 'upload'}`;
    case 2: return 'select';
    case 3: return 'download';
    case 4: return 'shortcut';
    case 5: {
      const tab = document.querySelector('#step-emulator .emu-nav__tab.is-active');
      return `emulator.${tab ? tab.dataset.section : 'files'}`;
    }
    case 6: return 'library';
    default: return 'start';
  }
}

function journeyNewScreen(name) {
  journey.visits[name] = (journey.visits[name] || 0) + 1;
  const now = Date.now();
  return { name, visit: journey.visits[name], enteredAt: now, lastAt: now, firstAt: null, longest: 0, clicks: {}, dead: {}, rage: {}, hovers: {} };
}

function journeyBump(map, name) {
  if (Object.keys(map).length >= 40 && !map[name]) return;
  map[name] = (map[name] || 0) + 1;
}

function journeyAction() {
  const s = journey.screen;
  if (!s) return;
  const now = Date.now();
  s.longest = Math.max(s.longest, now - s.lastAt);
  if (s.firstAt === null) s.firstAt = now;
  s.lastAt = now;
}

function journeyFinish(next) {
  const s = journey.screen;
  if (!s) return;
  const now = Date.now();
  const dwell = Math.round((now - s.enteredAt) / 1000);
  const touched = Object.keys(s.clicks).length + Object.keys(s.dead).length;
  if (dwell < 1 && !touched) return;
  const stepOf = n => STEP_ORDER[String(n).split('.')[0]] || 0;
  const back = stepOf(next) && stepOf(s.name) && stepOf(next) < stepOf(s.name) ? 1 : 0;
  emitEvent('ui_screen', {
    screen: s.name,
    next,
    visit: s.visit,
    dwell_s: dwell,
    first_action_s: s.firstAt === null ? null : Math.round((s.firstAt - s.enteredAt) / 1000),
    longest_pause_s: Math.round(Math.max(s.longest, now - s.lastAt) / 1000),
    back,
    clicks: s.clicks,
    dead: s.dead,
    rage: s.rage,
    hovers: s.hovers,
  });
}

function journeyCheckScreen() {
  if (!journey.active) return;
  const name = journeyScreenName();
  if (journey.screen && journey.screen.name === name) return;
  journeyFinish(name);
  journey.screen = journeyNewScreen(name);
}

function journeyClick(e) {
  if (!journey.active || !journey.screen) return;
  journeyCheckScreen();
  const target = e.target instanceof Element ? e.target : null;
  if (!target) return;
  const hit = target.closest(INTERACTIVE);
  const name = journeyName(hit || target);
  if (hit) {
    journeyBump(journey.screen.clicks, name);
  } else {
    journeyBump(journey.screen.dead, journeyName(target.closest('[id], [class]') || target));
  }
  const now = Date.now();
  const last = journey.lastClick;
  if (last.name === name && now - last.at < 800) {
    last.count += 1;
    if (last.count === 3) journeyBump(journey.screen.rage, name);
  } else {
    last.name = name;
    last.count = 1;
  }
  last.at = now;
  journeyAction();
}

function journeyHover(e) {
  if (!journey.active) return;
  clearTimeout(journey.hoverTimer);
  const el = e.target instanceof Element ? e.target.closest('[title], [data-tooltip]') : null;
  if (!el) return;
  journey.hoverTimer = setTimeout(() => {
    if (journey.screen) journeyBump(journey.screen.hovers, journeyName(el));
  }, 900);
}

function journeyNote(name) {
  if (journey.active && journey.screen) journeyBump(journey.screen.clicks, journeyLabel(name));
}

function startJourney() {
  if (journey.active) return;
  journey.active = true;
  journey.screen = journeyNewScreen(journeyScreenName());
  if (journey.bound) return;
  journey.bound = true;
  document.addEventListener('click', journeyClick, true);
  document.addEventListener('keydown', () => journey.active && journeyAction(), true);
  document.addEventListener('mouseover', journeyHover, true);
  document.addEventListener('mouseout', () => clearTimeout(journey.hoverTimer), true);
  new MutationObserver(() => {
    clearTimeout(journey.timer);
    journey.timer = setTimeout(journeyCheckScreen, 120);
  }).observe(document.body, { attributes: true, attributeFilter: ['class'], subtree: true });
  window.addEventListener('pagehide', () => journey.active && journeyFinish('exit'));
  document.addEventListener('visibilitychange', () => {
    if (!journey.active) return;
    if (document.visibilityState === 'hidden') {
      journeyFinish('background');
      journey.screen = null;
    } else {
      journey.screen = journeyNewScreen(journeyScreenName());
    }
  });
}

function stopJourney() {
  if (!journey.active) return;
  journeyFinish('off');
  journey.active = false;
  journey.screen = null;
}
