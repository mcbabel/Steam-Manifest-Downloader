use egui::{Key, Modifiers};

use crate::app::App;
use crate::dom::{el, Dom};
use crate::icons;

pub type Options = Vec<(String, String)>;

fn options_of(app: &App, id: &str, fixed: &[(&'static str, &'static str, &'static str)]) -> Options {
    if let Some(dynamic) = app.state.select_options.get(id) {
        return dynamic.clone();
    }
    fixed.iter().map(|(value, key, text)| (value.to_string(), if key.is_empty() { text.to_string() } else { app.i18n.t(key).to_string() })).collect()
}

pub fn set_options(app: &mut App, id: &str, options: Options) {
    app.state.select_options.insert(id.to_string(), options);
}

fn is_open(app: &App, id: &str) -> bool {
    app.state.open_select.as_deref() == Some(id)
}

fn set_open(app: &mut App, id: &str, open: bool, active: Option<usize>) {
    if open {
        app.state.open_select = Some(id.to_string());
        app.state.select_focus = active.unwrap_or(0);
    } else if is_open(app, id) {
        app.state.open_select = None;
    }
}

fn choose(app: &mut App, id: &str, options: &Options, index: usize) {
    let Some((value, _)) = options.get(index) else { return };
    let changed = app.ui.value(id) != value;
    app.ui.set_value(id, value.clone());
    set_open(app, id, false, None);
    if changed {
        app.changed(id);
    }
}

pub fn render(app: &mut App, dom: &mut Dom<'_>, id: &'static str, fixed: &[(&'static str, &'static str, &'static str)]) {
    let options = options_of(app, id, fixed);
    let value = app.ui.value(id).to_string();
    let active = options.iter().position(|(v, _)| *v == value);
    let shown = active.or(if options.is_empty() { None } else { Some(0) });
    let label = shown.map(|index| options[index].1.clone()).unwrap_or_default();
    let open = is_open(app, id);
    let focused = app.state.select_focus;
    let disabled = app.ui.disabled(id);
    let mut toggled = false;
    let mut chosen: Option<usize> = None;
    dom.el(el("div.history-sort.select-picker").class_if("is-open", open).track(id), |dom| {
        let button = el("button.history-sort__button.select-picker__button").state(crate::style::selector::DISABLED, disabled);
        let clicked = dom
            .button(button, |dom| {
                dom.el("span.history-sort__label", |dom| dom.text(&label));
                dom.icon(el("svg.history-sort__chevron").size(14.0, 14.0), icons::CHEVRON_DOWN);
            })
            .clicked();
        if clicked && !disabled {
            toggled = true;
        }
        dom.el(el("ul.history-sort__menu.history-sort__menu--scroll.select-picker__menu").class_if("hidden", !open), |dom| {
            for (index, (_, text)) in options.iter().enumerate() {
                let item = el("li.history-sort__option").class_if("is-active", shown == Some(index)).class_if("is-focused", open && focused == index).scroll_into_view(open && focused == index && app.state.select_scroll);
                if dom.button(item, |dom| dom.text(text)).clicked() {
                    chosen = Some(index);
                }
            }
        });
    });
    if open {
        app.state.select_scroll = false;
    }
    if toggled {
        set_open(app, id, !open, shown);
        app.state.select_scroll = true;
    }
    if let Some(index) = chosen {
        choose(app, id, &options, index);
    }
    if open {
        app.state.open_select_options = options;
    }
}

pub fn frame(app: &mut App, ctx: &egui::Context) {
    let Some(id) = app.state.open_select.clone() else { return };
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
            let inside = app.rects.iter().any(|(name, rect)| *name == id && rect.expand2(egui::vec2(0.0, 320.0)).contains(pointer));
            if !inside {
                app.state.open_select = None;
                return;
            }
        }
    }
    let options = app.state.open_select_options.clone();
    if options.is_empty() {
        return;
    }
    let pressed = |key: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    if pressed(Key::Escape) {
        app.state.open_select = None;
        return;
    }
    for (key, step) in [(Key::ArrowDown, 1), (Key::ArrowUp, options.len() - 1)] {
        if pressed(key) {
            app.state.select_focus = (app.state.select_focus + step) % options.len();
            app.state.select_scroll = true;
            return;
        }
    }
    if pressed(Key::Enter) || pressed(Key::Space) {
        let index = app.state.select_focus;
        choose(app, &id, &options, index);
        return;
    }
    let typed: String = ctx.input(|i| i.events.iter().filter_map(|event| if let egui::Event::Text(text) = event { Some(text.to_lowercase()) } else { None }).collect());
    if !typed.is_empty() {
        if let Some(hit) = options.iter().position(|(_, label)| label.to_lowercase().starts_with(&typed)) {
            app.state.select_focus = hit;
            app.state.select_scroll = true;
        }
    }
}
