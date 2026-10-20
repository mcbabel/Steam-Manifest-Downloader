// Steam account (QR sign-in only) and branches.
// The refresh token never reaches this page: the backend only hands out the
// account name, where the sign-in is kept, QR codes and branch lists.

const accountUi = {
  info: null,
  pollTimer: null,
  polling: false,
  interval: 5,
  branches: null,
  branch: 'public',
};

function accountEl(id) {
  return document.getElementById(id);
}

function accountStorageText(info) {
  if (!info) return '';
  if (info.storage === 'keychain') return window.i18n.t('account.storedKeychain');
  if (info.storage === 'file') return window.i18n.t('account.storedFile');
  return window.i18n.t('account.storedSession');
}

async function loadAccountStatus() {
  try {
    accountUi.info = await invoke('account_status');
  } catch (e) {
    console.warn('account_status failed:', e);
    accountUi.info = null;
  }
  renderAccountSettings();
  return accountUi.info;
}

function renderAccountSettings() {
  const status = accountEl('account-status');
  const detail = accountEl('account-detail');
  const login = accountEl('btn-account-login');
  const logout = accountEl('btn-account-logout');
  if (!status || !detail || !login || !logout) return;
  const info = accountUi.info;
  if (info) {
    status.textContent = window.i18n.t('account.signedInAs', { name: info.accountName });
    detail.textContent = accountStorageText(info);
    login.classList.add('hidden');
    logout.classList.remove('hidden');
  } else {
    status.textContent = window.i18n.t('account.signedOut');
    detail.textContent = window.i18n.t('account.signedOutHint');
    login.classList.remove('hidden');
    logout.classList.add('hidden');
  }
}

// ---- QR sign-in -------------------------------------------------------------

function setAccountQr(qr) {
  const box = accountEl('account-qr');
  if (!box) return;
  // The SVG comes from the backend's QR encoder and only holds the code.
  box.innerHTML = qr && qr.svg ? qr.svg : '';
  box.classList.toggle('account-qr--empty', !(qr && qr.svg));
}

function setAccountModalStatus(key, tone) {
  const el = accountEl('account-modal-status');
  if (!el) return;
  el.textContent = key ? window.i18n.t(key) : '';
  el.className = 'account-modal__status' + (tone ? ` account-modal__status--${tone}` : '');
}

function stopAccountPolling() {
  if (accountUi.pollTimer) clearTimeout(accountUi.pollTimer);
  accountUi.pollTimer = null;
  accountUi.polling = false;
}

function scheduleAccountPoll(seconds) {
  if (accountUi.pollTimer) clearTimeout(accountUi.pollTimer);
  accountUi.pollTimer = setTimeout(pollAccountLogin, Math.max(1, seconds || 5) * 1000);
}

async function startAccountLogin() {
  const modal = accountEl('account-modal');
  if (!modal) return;
  stopAccountPolling();
  setAccountQr(null);
  accountEl('btn-account-retry').classList.add('hidden');
  setAccountModalStatus('account.qrLoading');
  modal.classList.remove('hidden');
  emitEvent('account_login', { step: 'start' });
  try {
    const qr = await invoke('account_login_start');
    setAccountQr(qr);
    setAccountModalStatus('account.qrWaiting');
    accountUi.polling = true;
    accountUi.interval = qr.intervalSecs || 5;
    scheduleAccountPoll(accountUi.interval);
  } catch (e) {
    setAccountModalStatus(null);
    const el = accountEl('account-modal-status');
    el.textContent = window.i18n.localizeError(String(e));
    el.className = 'account-modal__status account-modal__status--error';
    accountEl('btn-account-retry').classList.remove('hidden');
    emitEvent('account_login', { step: 'failed', reason: 'start' });
  }
}

async function pollAccountLogin() {
  if (!accountUi.polling) return;
  let result;
  try {
    result = await invoke('account_login_poll');
  } catch (e) {
    stopAccountPolling();
    const el = accountEl('account-modal-status');
    el.textContent = window.i18n.localizeError(String(e));
    el.className = 'account-modal__status account-modal__status--error';
    accountEl('btn-account-retry').classList.remove('hidden');
    emitEvent('account_login', { step: 'failed', reason: 'poll' });
    return;
  }
  if (!accountUi.polling) return;
  switch (result.state) {
    case 'waiting':
      if (result.qr) {
        setAccountQr(result.qr);
        accountUi.interval = result.qr.intervalSecs || accountUi.interval;
      }
      setAccountModalStatus(result.scanned ? 'account.qrScanned' : 'account.qrWaiting');
      scheduleAccountPoll(accountUi.interval);
      break;
    case 'done':
      stopAccountPolling();
      accountUi.info = result.account;
      renderAccountSettings();
      closeAccountModal(false);
      showToast(window.i18n.t('account.signedInToast', { name: result.account.accountName }), 'success', 6000);
      emitEvent('account_login', { step: 'done', storage: result.account.storage });
      refreshBranchSection();
      break;
    case 'expired':
      stopAccountPolling();
      setAccountQr(null);
      setAccountModalStatus('account.qrExpired', 'warning');
      accountEl('btn-account-retry').classList.remove('hidden');
      emitEvent('account_login', { step: 'expired' });
      break;
    default:
      stopAccountPolling();
  }
}

function closeAccountModal(cancel = true) {
  const modal = accountEl('account-modal');
  if (modal) modal.classList.add('hidden');
  if (cancel && accountUi.polling) {
    stopAccountPolling();
    invoke('account_login_cancel').catch(() => {});
    emitEvent('account_login', { step: 'cancelled' });
  }
  stopAccountPolling();
}

async function accountLogout() {
  const name = accountUi.info && accountUi.info.accountName;
  try {
    await invoke('account_logout');
  } catch (e) {
    showToast(window.i18n.localizeError(String(e)), 'error');
    return;
  }
  accountUi.info = null;
  renderAccountSettings();
  refreshBranchSection();
  emitEvent('account_logout', {});
  showToast(window.i18n.t('account.signedOutToast', { name: name || '' }), 'success');
}

// ---- branches in the depot selection --------------------------------------------

function selectedBranch() {
  return accountUi.branch || 'public';
}

function selectedBranchInfo() {
  const list = (accountUi.branches && accountUi.branches.branches) || [];
  return list.find(b => b.name === selectedBranch()) || null;
}

function isPublicBranch(name) {
  return !name || name.toLowerCase() === 'public';
}

// Fields the download config needs for branches and the account.
function accountDownloadFields() {
  const branch = selectedBranch();
  const toggle = accountEl('use-account-toggle');
  const useAccount = !!(state.accountMode || !isPublicBranch(branch) || (accountUi.info && toggle && toggle.checked));
  const fields = { useAccount };
  if (!isPublicBranch(branch)) {
    fields.branch = branch;
    const pw = accountEl('branch-password');
    if (pw && pw.value.trim()) fields.branchPassword = pw.value.trim();
  }
  return fields;
}

// Blocks the start when the chosen branch cannot work as configured.
function branchStartProblem() {
  const branch = selectedBranch();
  if (isPublicBranch(branch)) return null;
  if (!accountUi.info) return 'branch.needsLogin';
  const info = selectedBranchInfo();
  const pw = accountEl('branch-password');
  if (info && info.passwordRequired && !info.available && !(pw && pw.value.trim())) return 'branch.needsPassword';
  return null;
}

function branchTag(b) {
  if (b.passwordRequired) return window.i18n.t('branch.passwordTag');
  if (!b.available) return window.i18n.t('branch.lockedTag');
  return '';
}

function branchItemHtml(b) {
  const tag = branchTag(b);
  return `<span class="branch-picker__name">${escapeHtml(b.name)}</span>`
    + (b.description ? `<span class="branch-picker__desc">${escapeHtml(b.description)}</span>` : '')
    + (tag ? `<span class="depot-tag branch-picker__tag">${escapeHtml(tag)}</span>` : '');
}

function branchList() {
  const list = (accountUi.branches && accountUi.branches.branches) || [];
  return list.length ? list : [{ name: 'public', description: '', passwordRequired: false, available: true }];
}

function branchOptions() {
  const menu = accountEl('branch-menu');
  return menu ? Array.from(menu.querySelectorAll('.history-sort__option')) : [];
}

function renderBranchSection() {
  const menu = accountEl('branch-menu');
  if (!menu) return;
  const names = branchList();
  if (!names.some(b => b.name === accountUi.branch)) accountUi.branch = 'public';
  menu.innerHTML = names.map(b =>
    `<li class="history-sort__option" role="option" data-value="${escapeHtml(b.name)}">${branchItemHtml(b)}</li>`,
  ).join('');
  branchOptions().forEach(opt => {
    opt.addEventListener('click', () => chooseBranch(opt.dataset.value));
    opt.addEventListener('mouseenter', () => {
      branchOptions().forEach(o => o.classList.toggle('is-focused', o === opt));
    });
  });
  setBranch(accountUi.branch);
}

function setBranch(name) {
  accountUi.branch = name || 'public';
  const label = accountEl('branch-button-label');
  const current = branchList().find(b => b.name === accountUi.branch);
  if (label && current) label.innerHTML = branchItemHtml(current);
  branchOptions().forEach(opt => {
    const active = opt.dataset.value === accountUi.branch;
    opt.classList.toggle('is-active', active);
    opt.setAttribute('aria-selected', active ? 'true' : 'false');
  });
  updateBranchControls();
}

function setBranchMenuOpen(open) {
  const picker = accountEl('branch-picker');
  const menu = accountEl('branch-menu');
  const button = accountEl('branch-button');
  if (!picker || !menu || !button) return;
  menu.classList.toggle('hidden', !open);
  button.setAttribute('aria-expanded', open ? 'true' : 'false');
  picker.classList.toggle('is-open', open);
  if (open) {
    const active = menu.querySelector('.is-active') || menu.firstElementChild;
    branchOptions().forEach(o => o.classList.toggle('is-focused', o === active));
    if (active) active.scrollIntoView({ block: 'nearest' });
  }
}

function chooseBranch(name) {
  setBranch(name);
  setBranchMenuOpen(false);
  const button = accountEl('branch-button');
  if (button) button.focus();
}

function bindBranchPicker() {
  const picker = accountEl('branch-picker');
  const menu = accountEl('branch-menu');
  const button = accountEl('branch-button');
  if (!picker || !menu || !button) return;
  button.addEventListener('click', () => setBranchMenuOpen(menu.classList.contains('hidden')));
  picker.addEventListener('keydown', (e) => {
    const open = !menu.classList.contains('hidden');
    const options = branchOptions();
    const focused = options.findIndex(o => o.classList.contains('is-focused'));
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      setBranchMenuOpen(false);
      button.focus();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) {
        setBranchMenuOpen(true);
        return;
      }
      const next = (focused + (e.key === 'ArrowDown' ? 1 : options.length - 1)) % options.length;
      options.forEach((o, i) => o.classList.toggle('is-focused', i === next));
      options[next].scrollIntoView({ block: 'nearest' });
    } else if ((e.key === 'Enter' || e.key === ' ') && open && focused >= 0) {
      e.preventDefault();
      chooseBranch(options[focused].dataset.value);
    } else if (e.key === 'Tab' && open) {
      setBranchMenuOpen(false);
    }
  }, true);
  document.addEventListener('click', (e) => {
    if (!picker.contains(e.target)) setBranchMenuOpen(false);
  });
}

function updateBranchControls() {
  const branch = selectedBranch();
  const info = selectedBranchInfo();
  const pw = accountEl('branch-password');
  const hint = accountEl('branch-hint');
  const accountRow = accountEl('use-account-row');
  const toggle = accountEl('use-account-toggle');
  const section = accountEl('branch-section');
  const label = accountEl('branch-label');
  const row = accountEl('branch-row');
  const loginRow = accountEl('branch-login-row');
  const more = accountEl('branch-more');
  const signedIn = !!accountUi.info;
  const nonPublic = !isPublicBranch(branch);
  const others = branchList().length - 1;

  if (section) section.classList.toggle('hidden', !signedIn && others < 1);
  if (label) label.classList.toggle('hidden', others < 1 && !nonPublic);
  if (row) row.classList.toggle('hidden', !signedIn || (others < 1 && !nonPublic));
  if (loginRow) loginRow.classList.toggle('hidden', signedIn || others < 1);
  if (more) more.textContent = window.i18n.t('branch.moreBranches', { count: others });
  if (pw) pw.classList.toggle('hidden', !(nonPublic && info && info.passwordRequired && !info.available));
  if (accountRow) accountRow.classList.toggle('hidden', !signedIn);
  if (toggle) {
    toggle.disabled = nonPublic || !!state.accountMode;
    if (nonPublic || state.accountMode) toggle.checked = true;
  }
  if (hint) {
    hint.textContent = signedIn && nonPublic ? window.i18n.t('branch.hintBranch') : '';
    hint.classList.toggle('hidden', !hint.textContent);
  }
}

async function refreshBranchSection() {
  const data = state.parsedData;
  const section = accountEl('branch-section');
  if (!section || !data || !data.mainAppId) return;
  const appId = String(data.mainAppId);
  section.dataset.appId = appId;
  if (!accountUi.info) await loadAccountStatus();
  try {
    const list = await invoke('list_branches', { appId });
    if (section.dataset.appId !== appId) return;
    accountUi.branches = list;
  } catch (e) {
    console.warn('list_branches failed:', e);
    accountUi.branches = null;
  }
  renderBranchSection();
}

function initBranchSection() {
  accountUi.branches = null;
  accountUi.branch = 'public';
  setBranchMenuOpen(false);
  const pw = accountEl('branch-password');
  if (pw) pw.value = '';
  const toggle = accountEl('use-account-toggle');
  if (toggle) toggle.checked = !!state.accountMode;
  renderBranchSection();
  refreshBranchSection();
}

// ---- search: load straight from the account ---------------------------------------

async function loadFromAccount(appId) {
  if (!accountUi.info) {
    openSettings('steam');
    return;
  }
  els.searchError.classList.add('hidden');
  els.manifestLoading.classList.remove('hidden');
  try {
    const depots = await invoke('account_app_depots', { appId: String(appId), branch: null, branchPassword: null });
    els.manifestLoading.classList.add('hidden');
    if (!depots.length) {
      showSearchError(window.i18n.t('account.noDepots'));
      return;
    }
    state.accountMode = true;
    state.searchRepo = null;
    state.searchSha = null;
    state.searchKeyVdfKeys = null;
    state.selectedRepo = { type: 'account' };
    state.parsedData = {
      mainAppId: appId,
      depots: depots.map(d => ({ depotId: String(d.depotId), manifestId: d.manifestId, depotKey: null, sizeBytes: null })),
    };
    emitEvent('account_depots', { count: depots.length });
    showSelectionStep();
  } catch (e) {
    els.manifestLoading.classList.add('hidden');
    showSearchError(window.i18n.localizeError(String(e)));
  }
}

function initAccount() {
  const login = accountEl('btn-account-login');
  if (login) login.addEventListener('click', startAccountLogin);
  const logout = accountEl('btn-account-logout');
  if (logout) logout.addEventListener('click', accountLogout);
  const retry = accountEl('btn-account-retry');
  if (retry) retry.addEventListener('click', startAccountLogin);
  const cancel = accountEl('btn-account-cancel');
  if (cancel) cancel.addEventListener('click', () => closeAccountModal(true));
  const modal = accountEl('account-modal');
  if (modal) modal.querySelector('.modal__backdrop').addEventListener('click', () => closeAccountModal(true));
  bindBranchPicker();
  const branchLogin = accountEl('btn-branch-login');
  if (branchLogin) branchLogin.addEventListener('click', () => openSettings('steam'));
  const fromAccount = accountEl('btn-search-account');
  if (fromAccount) fromAccount.addEventListener('click', () => loadFromAccount(state.searchAppId));
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal && !modal.classList.contains('hidden')) closeAccountModal(true);
  });
  loadAccountStatus();
}
