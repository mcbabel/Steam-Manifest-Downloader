use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug)]
pub enum Node {
    Text(String),
    El(Element),
}

#[derive(Clone, Debug, Default)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

const VOID: &[&str] = &["br", "img", "input", "hr", "icon"];
const PHRASING: &[&str] = &["strong", "b", "em", "i", "code", "kbd", "br"];

fn parse_attrs(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' && bytes[i] != b'/' {
            i += 1;
        }
        if start == i {
            break;
        }
        let name = source[start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let from = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                value = crate::html::decode_entities(&source[from..i]);
                i += 1;
            } else {
                let from = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = crate::html::decode_entities(&source[from..i]);
            }
        }
        out.push((name, value));
    }
    out
}

pub fn parse(source: &str) -> Vec<Node> {
    let mut stack: Vec<Element> = vec![Element::default()];
    let mut rest = source;
    while !rest.is_empty() {
        let Some(open) = rest.find('<') else {
            stack.last_mut().expect("root").children.push(Node::Text(rest.to_string()));
            break;
        };
        if open > 0 {
            stack.last_mut().expect("root").children.push(Node::Text(rest[..open].to_string()));
        }
        if rest[open..].starts_with("<!--") {
            rest = rest[open..].find("-->").map(|end| &rest[open + end + 3..]).unwrap_or("");
            continue;
        }
        let Some(close) = rest[open..].find('>') else { break };
        let inner = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            if let Some(position) = stack.iter().rposition(|e| e.tag == name) {
                while stack.len() > position.max(1) {
                    let done = stack.pop().expect("open element");
                    stack.last_mut().expect("root").children.push(Node::El(done));
                    if stack.len() == position {
                        break;
                    }
                }
            }
            continue;
        }
        let name_end = inner.find(|c: char| c.is_whitespace() || c == '/').unwrap_or(inner.len());
        let tag = inner[..name_end].to_ascii_lowercase();
        let element = Element { attrs: parse_attrs(&inner[name_end..]), tag, children: Vec::new() };
        if inner.ends_with('/') || VOID.contains(&element.tag.as_str()) {
            stack.last_mut().expect("root").children.push(Node::El(element));
        } else {
            stack.push(element);
        }
    }
    while stack.len() > 1 {
        let done = stack.pop().expect("open element");
        stack.last_mut().expect("root").children.push(Node::El(done));
    }
    stack.pop().map(|root| root.children).unwrap_or_default()
}

pub fn parsed(source: &str) -> Arc<Vec<Node>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, Arc<Vec<Node>>>>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    let key = hasher.finish();
    let mut cache = CACHE.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = cache.get(&key) {
        return found.clone();
    }
    if cache.len() > 4000 {
        cache.clear();
    }
    let nodes = Arc::new(parse(source));
    cache.insert(key, nodes.clone());
    nodes
}

pub fn is_phrasing(node: &Node) -> bool {
    match node {
        Node::Text(_) => true,
        Node::El(e) if e.tag == "a" => e.attr("id").is_none() && e.attr("data-click").is_none() && !e.attr("class").is_some_and(|class| class.contains("btn")) && e.children.iter().all(is_phrasing),
        Node::El(e) => PHRASING.contains(&e.tag.as_str()) && e.attrs.is_empty() && e.children.iter().all(is_phrasing),
    }
}

pub fn is_blank(node: &Node) -> bool {
    matches!(node, Node::Text(text) if text.trim().is_empty())
}

pub fn serialize(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::El(e) => {
                out.push('<');
                out.push_str(&e.tag);
                for name in ["href", "class", "data-click"] {
                    if let Some(value) = e.attr(name) {
                        out.push_str(&format!(" {}=\"{}\"", name, escape(value)));
                    }
                }
                out.push('>');
                if !VOID.contains(&e.tag.as_str()) {
                    serialize(&e.children, out);
                    out.push_str("</");
                    out.push_str(&e.tag);
                    out.push('>');
                }
            }
        }
    }
}

pub fn plain(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(&crate::html::decode_entities(text)),
            Node::El(e) => out.push_str(&plain(&e.children)),
        }
    }
    out
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nested_markup_with_attributes() {
        let nodes = parse(r#"<span class="a b" data-click="go:1">Hi <strong>you</strong></span><icon name="check"/> tail"#);
        assert_eq!(nodes.len(), 3);
        let Node::El(span) = &nodes[0] else { panic!("span expected") };
        assert_eq!(span.attr("class"), Some("a b"));
        assert_eq!(span.attr("data-click"), Some("go:1"));
        assert_eq!(span.children.len(), 2);
        assert!(span.children.iter().all(is_phrasing));
        let Node::El(icon) = &nodes[1] else { panic!("icon expected") };
        assert_eq!(icon.attr("name"), Some("check"));
        assert_eq!(plain(&nodes), "Hi you tail");
        assert_eq!(escape("<a&b>"), "&lt;a&amp;b&gt;");
    }
}
