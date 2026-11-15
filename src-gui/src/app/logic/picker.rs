use egui::{Key, Modifiers};

use crate::app::App;

#[derive(Clone, Copy)]
pub struct Picker {
    pub picker: &'static str,
    pub button: &'static str,
    pub menu: &'static str,
}

pub enum Outcome {
    None,
    Choose(usize),
}

pub fn is_open(app: &App, picker: Picker) -> bool {
    !app.ui.hidden(picker.menu)
}

pub fn set_open(app: &mut App, picker: Picker, open: bool, active: Option<usize>) {
    app.ui.set_hidden(picker.menu, !open);
    app.ui.set_attr(picker.button, "aria-expanded", if open { "true" } else { "false" });
    app.ui.set_class(picker.picker, "is-open", open);
    if open {
        app.state.picker_focus.insert(picker.menu, active.unwrap_or(0));
    }
}

pub fn focused(app: &App, picker: Picker) -> Option<usize> {
    is_open(app, picker).then(|| app.state.picker_focus.get(picker.menu).copied()).flatten()
}

pub fn frame(app: &mut App, ctx: &egui::Context, picker: Picker, count: usize, active: Option<usize>) -> Outcome {
    let open = is_open(app, picker);
    if open && ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
            let inside = [picker.picker, picker.menu].iter().any(|id| app.rects.get(id).is_some_and(|rect| rect.contains(pointer)));
            if !inside {
                set_open(app, picker, false, None);
                return Outcome::None;
            }
        }
    }
    let has_focus = open || app.focused == Some(picker.button);
    if !has_focus || count == 0 {
        return Outcome::None;
    }
    let pressed = |key: Key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
    if open && pressed(Key::Escape) {
        set_open(app, picker, false, None);
        app.ui.request_focus(picker.button);
        return Outcome::None;
    }
    for (key, step) in [(Key::ArrowDown, 1), (Key::ArrowUp, count - 1)] {
        if pressed(key) {
            if !open {
                set_open(app, picker, true, active);
            } else {
                let current = app.state.picker_focus.get(picker.menu).copied().unwrap_or(0);
                app.state.picker_focus.insert(picker.menu, (current + step) % count);
            }
            return Outcome::None;
        }
    }
    if open {
        if let Some(index) = app.state.picker_focus.get(picker.menu).copied() {
            if pressed(Key::Enter) || pressed(Key::Space) {
                return Outcome::Choose(index);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Tab)) {
            set_open(app, picker, false, None);
        }
    }
    Outcome::None
}
