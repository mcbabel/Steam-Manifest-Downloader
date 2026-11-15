use egui::{Key, Response};

use super::{logic, App};
use crate::dom::{Dom, El};
use crate::icons;

impl App {
    pub(crate) fn click(&mut self, id: &str) {
        if self.ui.disabled(id) {
            return;
        }
        logic::journey::click(self, id);
        if !logic::click(self, id) {
            self.unhandled.insert(format!("click {id}"));
        }
    }

    pub(crate) fn toggle(&mut self, id: &'static str) {
        if self.ui.disabled(id) {
            return;
        }
        let next = !self.ui.checked(id);
        self.ui.set_checked(id, next);
        logic::journey::click(self, id);
        self.changed(id);
    }

    pub(crate) fn radio(&mut self, group: &str, id: &'static str) {
        if self.ui.disabled(id) || self.ui.checked(id) {
            return;
        }
        for other in self.ui.radio_group(group) {
            self.ui.set_checked(other, other == id);
        }
        logic::journey::click(self, id);
        self.changed(id);
    }

    pub(crate) fn label_click(&mut self, target: &'static str) {
        match self.ui.input_kind(target) {
            Some("checkbox") => self.toggle(target),
            Some("radio") => {
                let group = self.ui.radio_group_of(target);
                self.radio(group, target);
            }
            _ => self.ui.request_focus(target),
        }
    }

    pub(crate) fn changed(&mut self, id: &str) {
        if !logic::changed(self, id) {
            self.unhandled.insert(format!("change {id}"));
        }
    }

    pub(crate) fn input_changed(&mut self, id: &str) {
        if !logic::input(self, id) {
            self.unhandled.insert(format!("input {id}"));
        }
    }

    pub(crate) fn key(&mut self, id: &str, key: Key) -> bool {
        logic::key(self, id, key)
    }

    pub(crate) fn input_events(&mut self, id: &str, response: &Response) {
        if self.ui.take_focus(id) {
            response.request_focus();
        }
        if response.changed() {
            self.input_changed(id);
        }
        if response.lost_focus() && response.ctx.input(|i| i.key_pressed(Key::Enter)) {
            self.key(id, Key::Enter);
        }
        if response.has_focus() {
            for key in [Key::ArrowDown, Key::ArrowUp] {
                if response.ctx.input(|i| i.key_pressed(key)) {
                    self.key(id, key);
                }
            }
        }
        if response.lost_focus() && response.ctx.input(|i| i.key_pressed(Key::Escape)) && self.key(id, Key::Escape) {
            response.ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
            response.request_focus();
        }
    }

    pub(crate) fn dragged(&mut self, id: &str, response: Option<Response>) {
        let Some(response) = response else { return };
        if id == "title-bar" {
            let below_edge = response.interact_pointer_pos().is_some_and(|p| p.y > 5.0);
            if response.double_clicked() {
                self.toggle_maximize();
            } else if response.drag_started_by(egui::PointerButton::Primary) && below_edge {
                self.start_window_drag();
            }
        }
    }

    pub(crate) fn markup(&mut self, markup: &str, dom: &mut Dom<'_>) {
        if let Some(action) = dom.markup(markup) {
            self.click(&action);
        }
    }

    pub(crate) fn html_key(&mut self, key: &str, dom: &mut Dom<'_>) {
        let markup = self.t(key).to_string();
        self.markup(&markup, dom);
    }

    pub(crate) fn override_content(&mut self, id: &str, dom: &mut Dom<'_>) -> bool {
        if let Some(markup) = self.ui.html(id).map(str::to_string) {
            self.markup(&markup, dom);
            true
        } else if let Some(text) = self.ui.text(id) {
            dom.text(text);
            true
        } else {
            false
        }
    }

    pub(crate) fn is_empty(&self, id: &str) -> bool {
        logic::is_empty(self, id).unwrap_or_else(|| self.ui.html(id).is_none_or(str::is_empty) && self.ui.text(id).is_none_or(str::is_empty))
    }

    pub(crate) fn label(&mut self, id: &str, key: &str, dom: &mut Dom<'_>) {
        if !self.override_content(id, dom) {
            dom.text(self.t(key));
        }
    }

    pub(crate) fn label_html(&mut self, id: &str, key: &str, dom: &mut Dom<'_>) {
        if !self.override_content(id, dom) {
            self.html_key(key, dom);
        }
    }

    pub(crate) fn fill(&mut self, id: &'static str, dom: &mut Dom<'_>) {
        if !logic::fill(self, id, dom) {
            self.override_content(id, dom);
        }
    }

    pub(crate) fn select(&mut self, dom: &mut Dom<'_>, element: El, id: &'static str, options: &[(&'static str, &'static str, &'static str)]) {
        logic::select(self, dom, element, id, options);
    }

    pub(crate) fn image(&mut self, dom: &mut Dom<'_>, element: El, key: &'static str) {
        if key == "/assets/icon-small.svg" {
            dom.icon(element, icons::APP_SMALL);
            return;
        }
        logic::image(self, dom, element, key);
    }

    pub(crate) fn open_url(&mut self, url: &str) {
        let url = logic::resolve_url(self, url);
        self.platform.open_url(&url);
    }
}
