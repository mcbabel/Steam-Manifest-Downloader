const SELECT_CHEVRON = '<svg class="history-sort__chevron" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="6 9 12 15 18 9"/></svg>';

function syncSelectPicker(select) {
  const picker = select && select._picker;
  if (!picker) return;
  const current = select.options[select.selectedIndex] || select.options[0];
  picker.label.textContent = current ? current.textContent : '';
  picker.menu.querySelectorAll('.history-sort__option').forEach((li, i) => {
    li.textContent = select.options[i] ? select.options[i].textContent : '';
    const active = select.options[i] && select.options[i].value === select.value;
    li.classList.toggle('is-active', active);
    li.setAttribute('aria-selected', active ? 'true' : 'false');
  });
}

function refreshSelectPickers(root = document) {
  root.querySelectorAll('select.select-picker__native').forEach(syncSelectPicker);
}

function setSelectPickerOpen(select, open) {
  const p = select._picker;
  p.menu.classList.toggle('hidden', !open);
  p.button.setAttribute('aria-expanded', open ? 'true' : 'false');
  p.wrap.classList.toggle('is-open', open);
  if (open) {
    const active = p.menu.querySelector('.is-active') || p.menu.firstElementChild;
    p.menu.querySelectorAll('.history-sort__option').forEach(o => o.classList.toggle('is-focused', o === active));
    if (active) active.scrollIntoView({ block: 'nearest' });
  }
}

function chooseSelectPicker(select, index) {
  if (select.selectedIndex !== index) {
    select.selectedIndex = index;
    select.dispatchEvent(new Event('change', { bubbles: true }));
  }
  syncSelectPicker(select);
  setSelectPickerOpen(select, false);
  select._picker.button.focus();
}

function fillSelectPickerMenu(select, menu) {
  menu.innerHTML = '';
  Array.from(select.options).forEach((opt, i) => {
    const li = document.createElement('li');
    li.className = 'history-sort__option';
    li.setAttribute('role', 'option');
    li.addEventListener('click', () => chooseSelectPicker(select, i));
    li.addEventListener('mouseenter', () => {
      menu.querySelectorAll('.history-sort__option').forEach(o => o.classList.toggle('is-focused', o === li));
    });
    menu.appendChild(li);
  });
}

function rebuildSelectPicker(select) {
  if (!select) return;
  if (!select._picker) {
    enhanceSelect(select);
    return;
  }
  fillSelectPickerMenu(select, select._picker.menu);
  syncSelectPicker(select);
}

function enhanceSelect(select) {
  if (!select || select._picker) return;
  const wrap = document.createElement('div');
  wrap.className = 'history-sort select-picker';
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'history-sort__button select-picker__button';
  button.setAttribute('aria-haspopup', 'listbox');
  button.setAttribute('aria-expanded', 'false');
  if (select.id) {
    const label = document.querySelector(`label[for="${CSS.escape(select.id)}"]`);
    if (label) {
      label.id = label.id || `${select.id}-label`;
      button.setAttribute('aria-labelledby', label.id);
      label.addEventListener('click', (e) => { e.preventDefault(); button.focus(); });
    }
  }
  const text = document.createElement('span');
  text.className = 'history-sort__label';
  button.appendChild(text);
  button.insertAdjacentHTML('beforeend', SELECT_CHEVRON);
  const menu = document.createElement('ul');
  menu.className = 'history-sort__menu history-sort__menu--scroll select-picker__menu hidden';
  menu.setAttribute('role', 'listbox');
  menu.tabIndex = -1;
  fillSelectPickerMenu(select, menu);
  select.classList.add('select-picker__native');
  select.tabIndex = -1;
  select.setAttribute('aria-hidden', 'true');
  select.parentNode.insertBefore(wrap, select);
  wrap.append(button, menu, select);
  select._picker = { wrap, button, menu, label: text };

  button.addEventListener('click', () => setSelectPickerOpen(select, menu.classList.contains('hidden')));
  wrap.addEventListener('keydown', (e) => {
    const open = !menu.classList.contains('hidden');
    const options = Array.from(menu.querySelectorAll('.history-sort__option'));
    const focused = options.findIndex(o => o.classList.contains('is-focused'));
    if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      setSelectPickerOpen(select, false);
      button.focus();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) {
        setSelectPickerOpen(select, true);
        return;
      }
      const next = (focused + (e.key === 'ArrowDown' ? 1 : options.length - 1)) % options.length;
      options.forEach((o, i) => o.classList.toggle('is-focused', i === next));
      options[next].scrollIntoView({ block: 'nearest' });
    } else if ((e.key === 'Enter' || (e.key === ' ' && !select._typed)) && open && focused >= 0) {
      e.preventDefault();
      chooseSelectPicker(select, focused);
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      clearTimeout(select._typeTimer);
      select._typed = (select._typed || '') + e.key.toLowerCase();
      select._typeTimer = setTimeout(() => { select._typed = ''; }, 800);
      const hit = Array.from(select.options).findIndex(o => o.textContent.toLowerCase().startsWith(select._typed));
      if (hit < 0) return;
      e.preventDefault();
      if (!open) setSelectPickerOpen(select, true);
      options.forEach((o, i) => o.classList.toggle('is-focused', i === hit));
      options[hit].scrollIntoView({ block: 'nearest' });
    }
  });
  document.addEventListener('click', (e) => {
    if (!wrap.contains(e.target) && !menu.classList.contains('hidden')) setSelectPickerOpen(select, false);
  });
  syncSelectPicker(select);
}
