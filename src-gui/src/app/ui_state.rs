use std::collections::{HashMap, HashSet};

use crate::dom::{el, El};
use crate::style::selector::DISABLED;

#[derive(Default)]
pub struct UiState {
    classes: HashMap<&'static str, Vec<&'static str>>,
    text: HashMap<String, String>,
    html: HashMap<String, String>,
    value: HashMap<String, String>,
    checked: HashMap<String, bool>,
    disabled: HashSet<String>,
    attrs: HashMap<(String, &'static str), String>,
    names: HashMap<String, &'static str>,
    inputs: HashMap<&'static str, (&'static str, &'static str)>,
    focus: Option<String>,
    styles: HashMap<String, Vec<(&'static str, String)>>,
    scroll: std::cell::RefCell<Option<String>>,
    flags: HashSet<String>,
}

impl UiState {
    fn name(&mut self, id: &str) -> &'static str {
        if let Some(found) = self.names.get(id) {
            return found;
        }
        let leaked: &'static str = Box::leak(id.to_string().into_boxed_str());
        self.names.insert(id.to_string(), leaked);
        leaked
    }

    pub fn declare(&mut self, id: &'static str, classes: &[&'static str]) {
        self.names.insert(id.to_string(), id);
        self.classes.insert(id, classes.to_vec());
    }

    pub fn declare_input(&mut self, id: &'static str, kind: &'static str, group: &'static str) {
        self.inputs.insert(id, (kind, group));
    }

    pub fn input_kind(&self, id: &str) -> Option<&'static str> {
        self.inputs.get(id).map(|(kind, _)| *kind)
    }

    pub fn radio_group(&self, group: &str) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.inputs.iter().filter(|(_, (kind, name))| *kind == "radio" && *name == group).map(|(id, _)| *id).collect();
        ids.sort_unstable();
        ids
    }

    pub fn radio_group_of(&self, id: &str) -> &'static str {
        self.inputs.get(id).map(|(_, group)| *group).unwrap_or("")
    }

    pub fn request_focus(&mut self, id: &str) {
        self.focus = Some(id.to_string());
    }

    pub fn take_focus(&mut self, id: &str) -> bool {
        if self.focus.as_deref() == Some(id) {
            self.focus = None;
            true
        } else {
            false
        }
    }

    pub fn known(&self, id: &str) -> bool {
        self.names.contains_key(id)
    }

    pub fn el(&self, spec: &'static str) -> El {
        let element = el(spec);
        match element.node.id {
            Some(id) => self.apply(id, element),
            None => element,
        }
    }

    pub fn el_as(&self, key: &'static str, spec: &'static str) -> El {
        self.apply(key, el(spec))
    }

    fn apply(&self, id: &'static str, mut element: El) -> El {
        if let Some(classes) = self.classes.get(id) {
            element.node.classes = classes.clone();
        }
        if self.disabled.contains(id) {
            element.node.state |= DISABLED;
        }
        for ((owner, name), value) in &self.attrs {
            if owner != id {
                continue;
            }
            match *name {
                "aria-expanded" | "data-mode" | "data-state" => element = element.attr(name, value),
                "title" => element = element.title(value.clone()),
                _ => {}
            }
        }
        if self.scroll.borrow().as_deref() == Some(id) {
            self.scroll.borrow_mut().take();
            element = element.scroll_into_view(true);
        }
        if let Some(style) = self.styles.get(id) {
            for (property, value) in style {
                element = element.css(property, value.clone());
            }
        }
        element
    }

    pub fn flag(&self, name: &str) -> bool {
        self.flags.contains(name)
    }

    pub fn set_flag(&mut self, name: &str, on: bool) {
        if on {
            self.flags.insert(name.to_string());
        } else {
            self.flags.remove(name);
        }
    }

    pub fn scroll_into_view(&mut self, id: &str) {
        *self.scroll.borrow_mut() = Some(id.to_string());
    }

    pub fn clear_content(&mut self, id: &str) {
        self.text.remove(id);
        self.html.remove(id);
    }

    pub fn set_style(&mut self, id: &str, property: &'static str, value: impl Into<String>) {
        let list = self.styles.entry(id.to_string()).or_default();
        list.retain(|(name, _)| *name != property);
        list.push((property, value.into()));
    }

    pub fn clear_style(&mut self, id: &str, property: &str) {
        if let Some(list) = self.styles.get_mut(id) {
            list.retain(|(name, _)| *name != property);
        }
    }

    pub fn has_class(&self, id: &str, class: &str) -> bool {
        self.names.get(id).and_then(|name| self.classes.get(name)).is_some_and(|list| list.iter().any(|c| *c == class))
    }

    pub fn set_class(&mut self, id: &str, class: &'static str, on: bool) {
        let name = self.name(id);
        let list = self.classes.entry(name).or_default();
        let present = list.contains(&class);
        if on && !present {
            list.push(class);
        } else if !on && present {
            list.retain(|c| *c != class);
        }
    }

    pub fn add_class(&mut self, id: &str, class: &'static str) {
        self.set_class(id, class, true);
    }

    pub fn remove_class(&mut self, id: &str, class: &'static str) {
        self.set_class(id, class, false);
    }

    pub fn set_classes(&mut self, id: &str, classes: &[&'static str]) {
        let name = self.name(id);
        self.classes.insert(name, classes.to_vec());
    }

    pub fn hidden(&self, id: &str) -> bool {
        self.has_class(id, "hidden")
    }

    pub fn set_hidden(&mut self, id: &str, hidden: bool) {
        self.set_class(id, "hidden", hidden);
    }

    pub fn hide(&mut self, id: &str) {
        self.set_hidden(id, true);
    }

    pub fn show(&mut self, id: &str) {
        self.set_hidden(id, false);
    }

    pub fn text(&self, id: &str) -> Option<&str> {
        self.text.get(id).map(String::as_str)
    }

    pub fn set_text(&mut self, id: &str, text: impl Into<String>) {
        self.html.remove(id);
        self.text.insert(id.to_string(), text.into());
    }

    pub fn clear_text(&mut self, id: &str) {
        self.text.remove(id);
        self.html.remove(id);
    }

    pub fn html(&self, id: &str) -> Option<&str> {
        self.html.get(id).map(String::as_str)
    }

    pub fn set_html(&mut self, id: &str, html: impl Into<String>) {
        self.text.remove(id);
        self.html.insert(id.to_string(), html.into());
    }

    pub fn value(&self, id: &str) -> &str {
        self.value.get(id).map(String::as_str).unwrap_or("")
    }

    pub fn value_mut(&mut self, id: &str) -> &mut String {
        self.value.entry(id.to_string()).or_default()
    }

    pub fn set_value(&mut self, id: &str, value: impl Into<String>) {
        self.value.insert(id.to_string(), value.into());
    }

    pub fn checked(&self, id: &str) -> bool {
        self.checked.get(id).copied().unwrap_or(false)
    }

    pub fn set_checked(&mut self, id: &str, checked: bool) {
        self.checked.insert(id.to_string(), checked);
    }

    pub fn disabled(&self, id: &str) -> bool {
        self.disabled.contains(id)
    }

    pub fn set_disabled(&mut self, id: &str, disabled: bool) {
        if disabled {
            self.disabled.insert(id.to_string());
        } else {
            self.disabled.remove(id);
        }
    }

    pub fn attr(&self, id: &str, name: &'static str) -> Option<&str> {
        self.attrs.get(&(id.to_string(), name)).map(String::as_str)
    }

    pub fn set_attr(&mut self, id: &str, name: &'static str, value: impl Into<String>) {
        self.attrs.insert((id.to_string(), name), value.into());
    }

    pub fn remove_attr(&mut self, id: &str, name: &'static str) {
        self.attrs.remove(&(id.to_string(), name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaves_like_a_class_list() {
        let mut ui = UiState::default();
        ui.declare("upload-error", &["error-message", "hidden"]);
        assert!(ui.hidden("upload-error"));
        ui.show("upload-error");
        assert!(!ui.hidden("upload-error"));
        assert_eq!(ui.el("div.error-message.hidden#upload-error").node.classes, vec!["error-message"]);
        ui.set_class("upload-error", "active", true);
        ui.set_class("upload-error", "active", true);
        assert_eq!(ui.el("div#upload-error").node.classes, vec!["error-message", "active"]);
        ui.set_text("upload-error", "broken");
        assert_eq!(ui.text("upload-error"), Some("broken"));
        ui.set_html("upload-error", "<b>x</b>");
        assert_eq!(ui.text("upload-error"), None);
    }
}
