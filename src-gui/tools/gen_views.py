import glob
import hashlib
import os
import re
import sys
from html.parser import HTMLParser

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
HTML = os.path.join(ROOT, 'src-gui', 'tools', 'index.html')
ICONS = os.path.join(ROOT, 'src-gui', 'src', 'icon_defs.rs')
OUT = os.path.join(ROOT, 'src-gui', 'src', 'app', 'view')

VOID = {'input', 'img', 'br', 'meta', 'link', 'hr'}
SVG_CHILDREN = {'path', 'line', 'polyline', 'circle', 'rect', 'polygon', 'g', 'ellipse'}
SPLIT_CLASSES = {'step', 'modal', 'settings__panel', 'emu-panel', 'tab-content', 'header'}
SPLIT_IDS = {'title-bar', 'steps-indicator', 'dotnet-warning'}
SPLIT_TAGS = set()
DRAG_IDS = {'title-bar'}
DYNAMIC_IDS = {'btn-theme-toggle'}
EXTRA_CLICK_IDS = {'drop-zone'}
DELEGATED_IDS = {'search-autocomplete', 'patch-only-autocomplete', 'sources-list', 'update-notes', 'emu-download-modal'}
CSS_ATTRS = {'data-section', 'type', 'for', 'readonly'}
CLICK_ROLES = {'button', 'tab', 'radio', 'option'}
CSS = os.path.join(ROOT, 'src-gui', 'assets', 'style.css')
EMPTY_CLASSES = set(re.findall(r'\.([\w-]+):empty', open(CSS, encoding='utf-8').read()))


class Node:
    def __init__(self, tag, attrs, parent):
        self.tag = tag
        self.attrs = attrs
        self.parent = parent
        self.children = []

    @property
    def id(self):
        return self.attrs.get('id')

    @property
    def classes(self):
        return self.attrs.get('class', '').split()

    def elements(self):
        return [c for c in self.children if isinstance(c, Node)]

    def find(self, pred):
        for c in self.elements():
            if pred(c):
                yield c
            yield from c.find(pred)

    def raw_svg(self):
        parts = []
        for c in self.children:
            if isinstance(c, Node):
                attrs = ' '.join('%s="%s"' % (k, v) for k, v in c.attrs.items())
                inner = c.raw_svg()
                if inner:
                    parts.append('<%s %s>%s</%s>' % (c.tag, attrs, inner, c.tag))
                else:
                    parts.append('<%s %s/>' % (c.tag, attrs))
        return ''.join(parts)


class Parser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = Node('root', {}, None)
        self.cur = self.root
        self.skip = 0

    def handle_starttag(self, tag, attrs):
        if tag in ('script', 'head'):
            self.skip += 1
            return
        if self.skip:
            return
        node = Node(tag, {k: (v if v is not None else '') for k, v in attrs}, self.cur)
        self.cur.children.append(node)
        if tag not in VOID:
            self.cur = node

    def handle_startendtag(self, tag, attrs):
        if self.skip:
            return
        node = Node(tag, {k: (v if v is not None else '') for k, v in attrs}, self.cur)
        self.cur.children.append(node)

    def handle_endtag(self, tag):
        if tag in ('script', 'head'):
            self.skip -= 1
            return
        if self.skip or tag in VOID:
            return
        node = self.cur
        while node is not self.root and node.tag != tag:
            node = node.parent
        if node is not self.root:
            self.cur = node.parent

    def handle_data(self, data):
        if self.skip:
            return
        self.cur.children.append(data)


def norm(text):
    return re.sub(r'\s+', ' ', text).strip()


def icon_key(body):
    body = re.sub(r'\s+', ' ', body)
    body = re.sub(r'>\s+<', '><', body).strip()
    shapes = re.findall(r'<(\w+)\s([^>]*?)/?>', body)
    canon = []
    for tag, attrs in shapes:
        pairs = sorted(re.findall(r'([\w-]+)="([^"]*)"', attrs))
        pairs = [(k, v.replace('currentColor', '#fff').replace('white', '#fff')) for k, v in pairs]
        canon.append((tag, tuple(pairs)))
    return hashlib.md5(repr(canon).encode()).hexdigest()


def load_icons():
    table = {}
    for m in re.finditer(r'pub const (\w+): IconDef = IconDef \{ name: "[^"]*", view: \[[^\]]*\], stroked: (\w+), body: r##"(.*?)"## \};', open(ICONS).read()):
        table[icon_key(m.group(3))] = (m.group(1), m.group(2) == 'true')
    return table


IDS = os.path.join(ROOT, 'src-gui', 'tools', 'ids.json')


def saved_ids(kind):
    import json
    return set(json.load(open(IDS, encoding='utf-8'))[kind])


def clickable_ids():
    return (saved_ids('clickable') | EXTRA_CLICK_IDS) - DELEGATED_IDS


def content_ids():
    return saved_ids('content')


def inner_html(node):
    parts = []
    for child in node.children:
        if isinstance(child, str):
            parts.append(re.sub(r'\s+', ' ', child).replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;'))
        else:
            attrs = ''.join(' %s="%s"' % (k, v.replace('"', '&quot;')) for k, v in child.attrs.items() if k in ('href', 'class'))
            if child.tag == 'br':
                parts.append('<br>')
            else:
                parts.append('<%s%s>%s</%s>' % (child.tag, attrs, inner_html(child), child.tag))
    return ''.join(parts).strip()


def owner_id(node):
    owner = node.parent
    while owner is not None and not owner.id:
        owner = owner.parent
    return owner.id if owner is not None else None


def panel_section(node):
    owner = node.parent
    while owner is not None:
        if 'emu-panel' in owner.classes:
            return owner.attrs.get('data-section')
        owner = owner.parent
    return None


CONTENT_KEYS = {
    ('emu-drm-section', 'emu-drm-title'): 'emu-drm-title',
    ('emu-drm-section', 'emu-drm-description'): 'emu-drm-description',
    ('emu-bypass-section', 'emu-bypass-row__hint'): 'emu-bypass-hint',
    ('history-remove-modal', 'modal__text'): 'history-remove-body',
    ('history-clear-modal', 'modal__text'): 'history-clear-body',
    ('step-emulator', 'emu-header'): 'emu-header',
    ('step-emulator', 'emu-description'): 'emu-description',
    ('shortcut-steam-row', 'shortcut-option__hint'): 'shortcut-steam-hint',
    ('btn-toggle-detected', 'settings-advanced__arrow'): 'shortcut-detected-arrow',
    ('btn-steam-toggle-detected', 'settings-advanced__arrow'): 'steam-detected-arrow',
}


def content_key(node):
    if node.id:
        return None
    owner = node.parent
    while owner is not None and not owner.id:
        owner = owner.parent
    if owner is None:
        return None
    for cls in node.classes:
        name = CONTENT_KEYS.get((owner.id, cls))
        if name:
            return name
    if 'emu-nav__badge' in node.classes and 'data-badge' in node.attrs:
        return 'emu-badge-' + node.attrs['data-badge']
    if 'emu-asset-row__value' in node.classes:
        row = node.parent
        while row is not None and 'data-asset' not in row.attrs:
            row = row.parent
        if row is not None:
            return 'emu-asset-value-' + row.attrs['data-asset']
    return None


def rust_str(text):
    return '"' + text.replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n') + '"'


def snake(name):
    return re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')


def i18n_attrs(node):
    out = {}
    spec = node.attrs.get('data-i18n-attr', '')
    for pair in spec.split(','):
        if '=' in pair:
            k, v = pair.split('=', 1)
            out[k.strip()] = v.strip()
    return out


class Gen:
    def __init__(self):
        self.icons = load_icons()
        self.click_ids = clickable_ids()
        self.functions = {}
        self.order = []
        self.initial = []
        self.unknown_icons = 0
        self.anon = 0
        self.modals = []
        self.modal_cancel = []
        self.keys = set()
        self.settings_serial = {}
        self.settings_rows = []
        self.settings_inputs = []
        self.content_ids = content_ids()
        self.i18n_ids = []
        self.select_ids = set()

    def spec(self, node, tag=None):
        s = tag or node.tag
        for c in node.classes:
            s += '.' + c
        if node.id:
            s += '#' + node.id
        elif node.tag == 'label' and node.attrs.get('for') in self.select_ids:
            s += '#%s-label' % node.attrs['for']
        for k in sorted(node.attrs):
            if k in CSS_ATTRS and not (k == 'type' and node.tag == 'button'):
                value = node.attrs[k]
                s += '[%s=%s]' % (k, value) if value else '[%s]' % k
        return s

    def state_key(self, node):
        if node.id:
            return None
        if 'steps__item' in node.classes and 'data-step' in node.attrs:
            return 'steps-item-' + node.attrs['data-step']
        if node.classes == ['app']:
            return 'app'
        if 'segmented__option' in node.classes and 'data-value' in node.attrs:
            owner = owner_id(node)
            if owner:
                return '%s-%s' % (owner, node.attrs['data-value'])
        if 'emu-nav__badge' in node.classes and 'data-badge' in node.attrs:
            return 'emu-badge-' + node.attrs['data-badge']
        if 'emu-extra__needs' in node.classes and panel_section(node):
            return 'emu-needs-' + panel_section(node)
        if 'data-asset-clear' in node.attrs:
            return 'emu-asset-clear-' + node.attrs['data-asset-clear']
        if 'history-sort__option' in node.classes and 'data-value' in node.attrs and owner_id(node) == 'history-sort-menu':
            return 'history-sort-' + node.attrs['data-value']
        if 'settings__tab' in node.classes and 'data-section' in node.attrs:
            return 'settings-tab-' + node.attrs['data-section']
        if 'settings__panel' in node.classes and 'data-section' in node.attrs:
            return 'settings-panel-' + node.attrs['data-section']
        if 'speed-limit__unit' in node.classes and 'data-unit' in node.attrs:
            return 'speed-unit-' + snake(node.attrs['data-unit'].split('/')[0])
        if 'settings-row' in node.classes or 'settings-group' in node.classes or 'settings__group-title' in node.classes:
            return self.settings_key(node)
        return None

    def settings_key(self, node):
        if node.id:
            return node.id
        if not hasattr(node, 'settings_key'):
            kind = 'row' if 'settings-row' in node.classes else 'group' if 'settings-group' in node.classes else 'title'
            self.settings_serial[kind] = self.settings_serial.get(kind, 0) + 1
            node.settings_key = 'settings-%s-%d' % (kind, self.settings_serial[kind])
        return node.settings_key

    def note_settings(self, node):
        if 'settings-row' in node.classes:
            group = node.parent
            while group is not None and 'settings-group' not in group.classes:
                group = group.parent
            panel = node.parent
            while panel is not None and 'settings__panel' not in panel.classes:
                panel = panel.parent
            if group is None or panel is None:
                return
            siblings = group.parent.elements()
            index = siblings.index(group)
            title = siblings[index - 1] if index > 0 and 'settings__group-title' in siblings[index - 1].classes else None
            keys = []
            for inner in [node] + list(node.find(lambda n: True)):
                for name in ('data-i18n', 'data-i18n-html'):
                    if inner.attrs.get(name):
                        keys.append(inner.attrs[name])
            title_keys = []
            if title is not None:
                for inner in [title] + list(title.find(lambda n: True)):
                    if inner.attrs.get('data-i18n'):
                        title_keys.append(inner.attrs['data-i18n'])
            self.settings_rows.append((self.settings_key(node), self.settings_key(group), self.settings_key(title) if title is not None else '', panel.attrs.get('data-section', ''), keys, title_keys))
        if node.tag in ('input', 'select') and node.id:
            owner = node.parent
            while owner is not None and 'settings__panels' not in owner.classes:
                owner = owner.parent
            if owner is not None:
                self.settings_inputs.append((node.id, node.attrs.get('type', 'select' if node.tag == 'select' else 'text')))

    def el(self, node, tag=None, extra=''):
        spec = rust_str(self.spec(node, tag))
        key = self.state_key(node)
        if key:
            expr = 'self.ui.el_as(%s, %s)' % (rust_str(key), spec)
            if key not in self.keys:
                self.keys.add(key)
                self.initial.append('    ui.declare(%s, &[%s]);' % (rust_str(key), ', '.join(rust_str(c) for c in node.classes)))
        else:
            expr = 'self.ui.el(%s)' % spec if node.id else 'el(%s)' % spec
        if 'emu-section' in node.classes and not node.id and panel_section(node) in ('saves', 'profile', 'network', 'mods'):
            expr += '.class_if("hidden", self.ui.flag(%s))' % rust_str('emu-extra-hide-' + panel_section(node))
        if node.id and EMPTY_CLASSES.intersection(node.classes):
            expr += '.state(crate::style::selector::EMPTY, self.is_empty(%s))' % rust_str(node.id)
        title = i18n_attrs(node).get('title')
        if title:
            expr += '.title_or(self.t(%s))' % rust_str(title)
        elif node.attrs.get('title'):
            expr += '.title_or(%s)' % rust_str(node.attrs['title'])
        return expr + extra

    def record_initial(self, node):
        if not node.id:
            return
        classes = ', '.join(rust_str(c) for c in node.classes)
        self.initial.append('    ui.declare(%s, &[%s]);' % (rust_str(node.id), classes))
        if 'disabled' in node.attrs:
            self.initial.append('    ui.set_disabled(%s, true);' % rust_str(node.id))
        if node.tag == 'input':
            kind = node.attrs.get('type', 'text')
            self.initial.append('    ui.declare_input(%s, %s, %s);' % (rust_str(node.id), rust_str(kind), rust_str(node.attrs.get('name', ''))))
            if kind in ('checkbox', 'radio'):
                if 'checked' in node.attrs:
                    self.initial.append('    ui.set_checked(%s, true);' % rust_str(node.id))
            elif node.attrs.get('value'):
                self.initial.append('    ui.set_value(%s, %s);' % (rust_str(node.id), rust_str(node.attrs['value'])))
        for name in ('aria-expanded', 'src', 'href'):
            if name in node.attrs and node.attrs[name]:
                self.initial.append('    ui.set_attr(%s, %s, %s);' % (rust_str(node.id), rust_str(name), rust_str(node.attrs[name])))

    def function_name(self, node):
        if node.id:
            return 'view_' + snake(node.id)
        if 'data-section' in node.attrs:
            cls = [c for c in node.classes if c in SPLIT_CLASSES]
            return 'view_%s_%s' % (snake(cls[0] if cls else node.tag), snake(node.attrs['data-section']))
        return 'view_' + snake(node.tag if node.tag in SPLIT_TAGS else (node.classes[0] if node.classes else node.tag))

    def is_split(self, node):
        return bool(set(node.classes) & SPLIT_CLASSES) or node.id in SPLIT_IDS or node.tag in SPLIT_TAGS

    def checkbox_in(self, node):
        found = [c for c in node.find(lambda n: n.tag == 'input' and n.attrs.get('type') in ('checkbox', 'radio'))]
        return found[0] if len(found) == 1 else None

    def click_key(self, node):
        if node.id:
            return node.id
        if 'data-asset-pick' in node.attrs:
            return 'asset-pick:' + node.attrs['data-asset-pick']
        if 'data-asset-clear' in node.attrs:
            return 'asset-clear:' + node.attrs['data-asset-clear']
        if 'segmented__option' in node.classes and 'data-value' in node.attrs and owner_id(node):
            return '%s:%s' % (owner_id(node), node.attrs['data-value'])
        if 'history-sort__option' in node.classes and 'data-value' in node.attrs and owner_id(node) == 'history-sort-menu':
            return 'history-sort:' + node.attrs['data-value']
        if 'settings__tab' in node.classes and 'data-section' in node.attrs:
            return 'settings-tab:' + node.attrs['data-section']
        if 'speed-limit__unit' in node.classes and 'data-unit' in node.attrs:
            return 'speed-unit:' + node.attrs['data-unit']
        for name in ('data-section', 'data-value', 'data-tab', 'data-feedback', 'data-choice', 'data-asset', 'data-step', 'data-badge'):
            if name in node.attrs:
                return '%s[%s=%s]' % (node.classes[0] if node.classes else node.tag, name, node.attrs[name])
        if 'data-modal-cancel' in node.attrs:
            return 'modal-cancel'
        owner = node.parent
        while owner is not None and not owner.id:
            owner = owner.parent
        base = owner.id if owner is not None and owner.id else 'root'
        return '%s>%s' % (base, node.classes[0] if node.classes else node.tag)

    def is_clickable(self, node):
        if node.tag in ('button', 'a'):
            return True
        if node.attrs.get('role') in CLICK_ROLES:
            return True
        if node.id and node.id in self.click_ids:
            return True
        return 'modal__backdrop' in node.classes

    def emit_children(self, node, out, ind):
        pad = '    ' * ind
        key = node.attrs.get('data-i18n')
        html = node.attrs.get('data-i18n-html')
        elements = node.elements()
        texts = [norm(c) for c in node.children if isinstance(c, str) and norm(c)]
        named = content_key(node)
        if named:
            if key:
                self.i18n_ids.append(named)
                out.append('%sself.label(%s, %s, dom);' % (pad, rust_str(named), rust_str(key)))
            elif html:
                self.i18n_ids.append(named)
                out.append('%sself.label_html(%s, %s, dom);' % (pad, rust_str(named), rust_str(html)))
            else:
                if texts:
                    self.initial.append('    ui.set_text(%s, %s);' % (rust_str(named), rust_str(' '.join(texts))))
                out.append('%sself.fill(%s, dom);' % (pad, rust_str(named)))
            return
        if node.id and (key or html):
            self.i18n_ids.append(node.id)
        if key:
            if node.id:
                out.append('%sself.label(%s, %s, dom);' % (pad, rust_str(node.id), rust_str(key)))
            else:
                out.append('%sdom.text(self.t(%s));' % (pad, rust_str(key)))
            return
        if html:
            if node.id:
                out.append('%sself.label_html(%s, %s, dom);' % (pad, rust_str(node.id), rust_str(html)))
            else:
                out.append('%sself.html_key(%s, dom);' % (pad, rust_str(html)))
            return
        if node.id in DYNAMIC_IDS:
            out.append('%sself.fill(%s, dom);' % (pad, rust_str(node.id)))
            return
        if 'steps__number' in node.classes and node.parent is not None and 'data-step' in node.parent.attrs:
            name = 'steps-number-' + node.parent.attrs['data-step']
            self.initial.append('    ui.set_text(%s, %s);' % (rust_str(name), rust_str(' '.join(texts))))
            out.append('%sself.fill(%s, dom);' % (pad, rust_str(name)))
            return
        if node.id and not elements:
            if texts:
                self.initial.append('    ui.set_text(%s, %s);' % (rust_str(node.id), rust_str(' '.join(texts))))
            out.append('%sself.fill(%s, dom);' % (pad, rust_str(node.id)))
            return
        inline_tags = {'strong', 'b', 'em', 'i', 'code', 'kbd', 'br', 'a'}
        if texts and elements and all(c.tag in inline_tags and not c.id and 'btn' not in c.attrs.get('class', '') for c in elements) and any(c.tag == 'a' for c in elements):
            out.append('%sself.markup(%s, dom);' % (pad, rust_str(inner_html(node))))
            return
        guarded = bool(node.id) and node.tag in ('button', 'a', 'span', 'p', 'div', 'h2', 'h3', 'label') and node.id in self.content_ids
        if guarded:
            out.append('%sif !self.override_content(%s, dom) {' % (pad, rust_str(node.id)))
            ind += 1
            pad = '    ' * ind
        for child in node.children:
            if isinstance(child, str):
                text = norm(child)
                if text:
                    out.append('%sdom.text(%s);' % (pad, rust_str(text)))
            else:
                self.emit(child, out, ind)
        if not node.children:
            out.append('%slet _ = &dom;' % pad)
        if guarded:
            out.append('%s}' % ('    ' * (ind - 1)))
        if 'settings-section' in node.classes and any(True for _ in node.find(lambda n: n.id == 'mh-apikey')):
            out.append('%sself.fill("mh-apikey-required", dom);' % ('    ' * (ind - 1 if guarded else ind)))

    def emit_svg(self, node, out, ind):
        pad = '    ' * ind
        found = self.icons.get(icon_key(node.raw_svg()))
        if not found:
            self.unknown_icons += 1
            name, stroked = 'ALERT_TRIANGLE', True
        else:
            name, stroked = found
        size = ''
        if node.attrs.get('width') and node.attrs.get('height'):
            size = '.size(%s, %s)' % (float(node.attrs['width']), float(node.attrs['height']))
        element = self.el(node, 'svg', size)
        if stroked:
            width = float(node.attrs.get('stroke-width', '2'))
            round_caps = 'true' if node.attrs.get('stroke-linecap') == 'round' else 'false'
            out.append('%sdom.icon_with(%s, icons::%s, %s, %s);' % (pad, element, name, width, round_caps))
        else:
            out.append('%sdom.icon(%s, icons::%s);' % (pad, element, name))

    def input_id(self, node):
        if 'data-emu-key' in node.attrs:
            return 'emu-set-' + node.attrs['data-emu-key']
        if node.attrs.get('name') and node.attrs.get('value'):
            return '%s-%s' % (node.attrs['name'], node.attrs['value'])
        self.anon += 1
        return 'anon-input-%d' % self.anon

    def emit_input(self, node, out, ind):
        pad = '    ' * ind
        kind = node.attrs.get('type', 'text')
        synthetic = getattr(node, 'synthetic', False)
        if not node.id:
            node.attrs['id'] = self.input_id(node)
            node.synthetic = synthetic = True
            self.record_initial(node)
        ident = rust_str(node.id)
        if kind in ('checkbox', 'radio'):
            name = node.attrs.get('name', '')
            call = 'self.toggle(%s)' % ident if kind == 'checkbox' else 'self.radio(%s, %s)' % (rust_str(name), ident)
            if synthetic:
                bare = self.spec(node).replace('#' + node.id, '')
                target = 'self.ui.el_as(%s, %s)' % (ident, rust_str(bare))
            else:
                target = 'self.ui.el(%s)' % rust_str(self.spec(node))
            if getattr(node, 'in_label', False):
                out.append('%sdom.empty(%s.state(CHECKED, self.ui.checked(%s)));' % (pad, target, ident))
            else:
                out.append('%sif dom.button(%s.state(CHECKED, self.ui.checked(%s)), |_| ()).clicked() {' % (pad, target, ident))
                out.append('%s    %s;' % (pad, call))
                out.append('%s}' % pad)
            return
        placeholder = i18n_attrs(node).get('placeholder')
        literal = node.attrs.get('placeholder')
        out.append('%s{' % pad)
        out.append('%s    let element = %s;' % (pad, self.el(node)))
        if placeholder:
            out.append('%s    let placeholder = self.ui.attr(%s, "placeholder").map(str::to_string).unwrap_or_else(|| self.t(%s).to_string());' % (pad, ident, rust_str(placeholder)))
        else:
            out.append('%s    let placeholder = self.ui.attr(%s, "placeholder").map(str::to_string).unwrap_or_else(|| String::from(%s));' % (pad, ident, rust_str(literal or '')))
        password = 'true' if kind == 'password' else 'false'
        out.append('%s    let response = dom.input_with(element, self.ui.value_mut(%s), &placeholder, %s);' % (pad, ident, password))
        out.append('%s    self.input_events(%s, &response);' % (pad, ident))
        out.append('%s}' % pad)

    def emit(self, node, out, ind, force_inline=False):
        pad = '    ' * ind
        self.record_initial(node)
        if not force_inline:
            self.note_settings(node)
        if node.tag == 'svg':
            self.emit_svg(node, out, ind)
            return
        if node.tag == 'input':
            self.emit_input(node, out, ind)
            return
        if node.tag == 'br':
            return
        if node.tag == 'select':
            options = []
            for option in node.find(lambda n: n.tag == 'option'):
                label = option.attrs.get('data-i18n')
                text = norm(''.join(c for c in option.children if isinstance(c, str)))
                value = option.attrs.get('value', text)
                options.append('(%s, %s, %s)' % (rust_str(value), rust_str(label or ''), rust_str(text)))
                if 'selected' in option.attrs and node.id:
                    self.initial.append('    ui.set_value(%s, %s);' % (rust_str(node.id), rust_str(value)))
            out.append('%sself.select(dom, %s, %s, &[%s]);' % (pad, self.el(node), rust_str(node.id or ''), ', '.join(options)))
            return
        if node.tag == 'textarea':
            if not node.id:
                self.anon += 1
                node.attrs['id'] = 'anon-textarea-%d' % self.anon
                self.record_initial(node)
            placeholder = i18n_attrs(node).get('placeholder')
            out.append('%s{' % pad)
            out.append('%s    let element = %s;' % (pad, self.el(node)))
            out.append('%s    let placeholder = %s;' % (pad, 'self.t(%s).to_string()' % rust_str(placeholder) if placeholder else 'String::new()'))
            out.append('%s    let response = dom.textarea(element, self.ui.value_mut(%s), &placeholder, %s);' % (pad, rust_str(node.id), int(node.attrs.get('rows', '3'))))
            out.append('%s    self.input_events(%s, &response);' % (pad, rust_str(node.id)))
            out.append('%s}' % pad)
            return
        if node.tag == 'img':
            size = ''
            if node.attrs.get('width') and node.attrs.get('height'):
                size = '.size(%s, %s)' % (float(node.attrs['width']), float(node.attrs['height']))
            out.append('%sself.image(dom, %s, %s);' % (pad, self.el(node, extra=size), rust_str(node.id or node.attrs.get('src', ''))))
            return
        if self.is_split(node) and not force_inline:
            name = self.function_name(node)
            base = name
            n = 2
            while name in self.functions:
                name = '%s_%d' % (base, n)
                n += 1
            body = []
            self.functions[name] = body
            self.order.append(name)
            self.emit(node, body, 2, force_inline=True)
            if 'modal' in node.classes and node.id:
                inside = False
                above = node.parent
                while above is not None:
                    inside = inside or 'app-shell' in above.classes
                    above = above.parent
                self.modals.append((node.id, name, inside))
                cancel = next(node.find(lambda n: 'data-modal-cancel' in n.attrs), None)
                if cancel is None:
                    cancel = next(node.find(lambda n: 'btn--outline' in n.classes), None)
                if cancel is None:
                    cancel = next(node.find(lambda n: n.tag == 'button'), None)
                if cancel is not None:
                    self.modal_cancel.append((node.id, self.click_key(cancel)))
            else:
                out.append('%sself.%s(dom);' % (pad, name))
            return

        checkbox = self.checkbox_in(node) if node.tag == 'label' else None
        extra = ''
        action = None
        if checkbox is not None:
            checkbox.in_label = True
            if not checkbox.id:
                checkbox.attrs['id'] = self.input_id(checkbox)
                checkbox.synthetic = True
            ident = rust_str(checkbox.id)
            extra = '.state(HAS_CHECKED, self.ui.checked(%s))' % ident
            if checkbox.attrs.get('type') == 'radio':
                action = 'self.radio(%s, %s)' % (rust_str(checkbox.attrs.get('name', '')), ident)
            else:
                action = 'self.toggle(%s)' % ident
        elif node.tag == 'label' and node.attrs.get('for'):
            action = 'self.label_click(%s)' % rust_str(node.attrs['for'])
        elif node.tag == 'a' and node.attrs.get('href') and not node.id:
            action = 'self.open_url(%s)' % rust_str(node.attrs['href'])
        elif self.is_clickable(node):
            action = 'self.click(%s)' % rust_str(self.click_key(node))

        element = self.el(node, extra=extra)
        if node.id in DRAG_IDS:
            out.append('%slet dragged = dom.hit(%s, egui::Sense::click_and_drag(), |dom| {' % (pad, element))
            self.emit_children(node, out, ind + 1)
            out.append('%s});' % pad)
            out.append('%sself.dragged(%s, dragged.and_then(|out| out.response));' % (pad, rust_str(node.id)))
            return
        elements = node.elements()
        has_text = any(isinstance(c, str) and norm(c) for c in node.children)
        simple = not elements and not has_text and not node.id and not node.attrs.get('data-i18n') and not node.attrs.get('data-i18n-html')
        if node.id in DELEGATED_IDS or (node.tag in ('ul', 'ol') and node.id and not elements) or content_key(node):
            simple = False
        if action is None and simple:
            out.append('%sdom.empty(%s);' % (pad, element))
            return
        if action is None and 'modal__content' in node.classes:
            out.append('%sdom.hit(%s, egui::Sense::CLICK, |dom| {' % (pad, element))
            self.emit_children(node, out, ind + 1)
            out.append('%s});' % pad)
        elif action is None:
            out.append('%sdom.el(%s, |dom| {' % (pad, element))
            self.emit_children(node, out, ind + 1)
            out.append('%s});' % pad)
        else:
            out.append('%sif dom.button(%s, |dom| {' % (pad, element))
            self.emit_children(node, out, ind + 1)
            out.append('%s}).clicked() {' % pad)
            out.append('%s    %s;' % (pad, action))
            out.append('%s}' % pad)


def main():
    parser = Parser()
    parser.feed(open(HTML, encoding='utf-8').read())
    body = next(parser.root.find(lambda n: n.tag == 'body'))
    gen = Gen()
    gen.select_ids = {n.id for n in parser.root.find(lambda n: n.tag == 'select') if n.id}
    shell = []
    gen.functions['view_body'] = shell
    gen.order.append('view_body')
    for child in body.elements():
        gen.emit(child, shell, 2)
    os.makedirs(OUT, exist_ok=True)
    for old in glob.glob(os.path.join(OUT, '*.rs')):
        os.remove(old)
    groups = {}
    for name in gen.order:
        group = 'shell'
        if name.startswith('view_step_'):
            group = name[len('view_'):]
        elif name.startswith('view_emu_panel'):
            group = 'step_emulator'
        elif name.startswith('view_tab_content'):
            group = 'step_upload'
        elif name.startswith('view_settings'):
            group = 'settings'
        elif name.endswith('_modal') or name in ('view_language_picker',):
            group = 'modals'
        groups.setdefault(group, []).append(name)
    header = 'use crate::app::App;\nuse crate::dom::{el, Dom};\nuse crate::icons;\nuse crate::style::selector::{CHECKED, HAS_CHECKED};\n\n'
    for group, names in groups.items():
        lines = ['#![allow(unused_imports, clippy::all)]', '', header.rstrip(), '', 'impl App {']
        for name in names:
            lines.append('    pub(crate) fn %s(&mut self, dom: &mut Dom<\'_>) {' % name)
            lines.extend(gen.functions[name])
            lines.append('    }')
            lines.append('')
        if lines[-1] == '':
            lines.pop()
        lines.append('}')
        open(os.path.join(OUT, group + '.rs'), 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
    mod = ['#![allow(clippy::all)]', '']
    for group in sorted(groups):
        mod.append('mod %s;' % group)
    mod += ['', 'use crate::app::ui_state::UiState;', '', 'pub const MODALS: &[(&str, fn(&mut crate::app::App, &mut crate::dom::Dom<\'_>), bool)] = &[']
    for ident, name, inside in gen.modals:
        mod.append('    (%s, crate::app::App::%s, %s),' % (rust_str(ident), name, 'true' if inside else 'false'))
    mod += ['];', '', 'pub const MODAL_CANCEL: &[(&str, &str)] = &[']
    for ident, key in gen.modal_cancel:
        mod.append('    (%s, %s),' % (rust_str(ident), rust_str(key)))
    mod += ['];', '', 'pub const SETTINGS_ROWS: &[(&str, &str, &str, &str, &[&str], &[&str])] = &[']
    for row, group, title, panel, keys, title_keys in gen.settings_rows:
        mod.append('    (%s, %s, %s, %s, &[%s], &[%s]),' % (rust_str(row), rust_str(group), rust_str(title), rust_str(panel), ', '.join(rust_str(k) for k in keys), ', '.join(rust_str(k) for k in title_keys)))
    mod += ['];', '', 'pub const SETTINGS_INPUTS: &[(&str, &str)] = &[']
    for ident, kind in gen.settings_inputs:
        mod.append('    (%s, %s),' % (rust_str(ident), rust_str(kind)))
    mod += ['];', '', 'pub const I18N_IDS: &[&str] = &[']
    for ident in gen.i18n_ids:
        mod.append('    %s,' % rust_str(ident))
    mod += ['];', '', 'pub fn declare(ui: &mut UiState) {']
    mod += gen.initial
    mod += ['}', '']
    open(os.path.join(OUT, 'mod.rs'), 'w', encoding='utf-8').write('\n'.join(mod))
    print('functions', len(gen.order), 'modals', len(gen.modals), 'ids', sum(1 for l in gen.initial if 'declare(' in l), 'unknown icons', gen.unknown_icons, 'clickable ids', len(gen.click_ids))
    for group, names in groups.items():
        print(' ', group, len(names))


main()
