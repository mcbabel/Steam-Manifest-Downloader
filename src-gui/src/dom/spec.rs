use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use crate::style::NodeInfo;

#[derive(Clone, Debug, Default)]
pub struct El {
    pub node: NodeInfo,
    pub key: Option<u64>,
    pub title: Option<String>,
    pub label: Option<String>,
    pub size: Option<[f32; 2]>,
    pub inline: Vec<(&'static str, String)>,
    pub scroll: bool,
    pub track: Option<&'static str>,
}

pub(crate) fn intern(text: &str) -> &'static str {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = names.get(text) {
        return found;
    }
    let leaked: &'static str = Box::leak(text.to_string().into_boxed_str());
    names.insert(text.to_string(), leaked);
    leaked
}

fn parse(spec: &str) -> NodeInfo {
    let mut node = NodeInfo { tag: "div", ..Default::default() };
    let mut rest = spec.trim();
    let name_end = |s: &str| s.find(['.', '#', '[']).unwrap_or(s.len());
    let end = name_end(rest);
    if end > 0 {
        node.tag = intern(&rest[..end]);
    }
    rest = &rest[end..];
    while !rest.is_empty() {
        match rest.as_bytes()[0] {
            b'.' => {
                let end = name_end(&rest[1..]) + 1;
                node.classes.push(intern(&rest[1..end]));
                rest = &rest[end..];
            }
            b'#' => {
                let end = name_end(&rest[1..]) + 1;
                node.id = Some(intern(&rest[1..end]));
                rest = &rest[end..];
            }
            _ => {
                let end = rest.find(']').unwrap_or(rest.len() - 1);
                let inner = &rest[1..end];
                let (name, value) = inner.split_once('=').unwrap_or((inner, ""));
                node.attrs.push((intern(name.trim()), intern(value.trim().trim_matches('"'))));
                rest = &rest[end + 1..];
            }
        }
    }
    node
}

pub fn el(spec: &'static str) -> El {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, NodeInfo>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    let node = cache.entry(spec).or_insert_with(|| parse(spec)).clone();
    El { node, ..Default::default() }
}

pub fn el_dyn(tag: &str, id: Option<&str>, classes: &str) -> El {
    let mut node = NodeInfo { tag: intern(tag), ..Default::default() };
    node.id = id.filter(|id| !id.is_empty()).map(intern);
    for class in classes.split_whitespace() {
        let class = intern(class);
        if !node.classes.contains(&class) {
            node.classes.push(class);
        }
    }
    El { node, ..Default::default() }
}

impl El {
    pub fn class_dyn(self, class: &str) -> Self {
        let mut out = self;
        for name in class.split_whitespace() {
            let name = intern(name);
            if !out.node.classes.contains(&name) {
                out.node.classes.push(name);
            }
        }
        out
    }

    pub fn attr_dyn(mut self, name: &str, value: &str) -> Self {
        let name = intern(name);
        self.node.attrs.retain(|(n, _)| *n != name);
        self.node.attrs.push((name, intern(value)));
        self
    }

    pub fn css_dyn(mut self, property: &str, value: impl Into<String>) -> Self {
        self.inline.push((intern(property), value.into()));
        self
    }

    pub fn class(mut self, class: &'static str) -> Self {
        for name in class.split_whitespace() {
            if !self.node.classes.contains(&name) {
                self.node.classes.push(name);
            }
        }
        self
    }

    pub fn class_if(self, class: &'static str, on: bool) -> Self {
        if on {
            self.class(class)
        } else {
            self
        }
    }

    pub fn hidden(self, hidden: bool) -> Self {
        self.class_if("hidden", hidden)
    }

    pub fn attr(mut self, name: &'static str, value: &str) -> Self {
        if name == "id" {
            self.node.id = Some(intern(value));
            return self;
        }
        self.node.attrs.retain(|(n, _)| *n != name);
        self.node.attrs.push((name, intern(value)));
        self
    }

    pub fn state(mut self, flag: u16, on: bool) -> Self {
        if on {
            self.node.state |= flag;
        } else {
            self.node.state &= !flag;
        }
        self
    }

    pub fn key(mut self, key: impl Hash) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        self.key = Some(hasher.finish());
        self
    }

    pub fn track(mut self, name: &str) -> Self {
        self.track = Some(intern(name));
        self
    }

    pub fn scroll_into_view(mut self, on: bool) -> Self {
        self.scroll = on;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn title_or(mut self, title: impl Into<String>) -> Self {
        if self.title.is_none() {
            self.title = Some(title.into());
        }
        self
    }

    pub fn css(mut self, property: &'static str, value: impl Into<String>) -> Self {
        self.inline.push((property, value.into()));
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Some([width, height]);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl From<&'static str> for El {
    fn from(spec: &'static str) -> Self {
        el(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tag_classes_id_and_attributes() {
        let e = el("button.btn.btn--primary#btn-search[type=button]").class_if("active", true).class_if("x", false);
        assert_eq!(e.node.tag, "button");
        assert_eq!(e.node.id, Some("btn-search"));
        assert_eq!(e.node.classes, vec!["btn", "btn--primary", "active"]);
        assert_eq!(e.node.attrs, vec![("type", "button")]);
        assert_eq!(el(".card").node.tag, "div");
    }
}
