use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use super::computed::{apply_tag_defaults, ApplyCx, Computed};
use super::parse::{parse, RawDecl};
use super::selector::{matches, parse_selector, Chain, Combinator, NodeInfo, Pseudo, PseudoElement, Selector, HOVER};
use super::value::resolve_vars;

struct Rule {
    selector: Selector,
    decls: Arc<[RawDecl]>,
    media_max_width: Option<f32>,
    order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    pub light: bool,
    pub viewport: [f32; 2],
}

pub struct Sheet {
    rules: Vec<Rule>,
    by_class: HashMap<String, Vec<u32>>,
    by_tag: HashMap<String, Vec<u32>>,
    by_id: HashMap<String, Vec<u32>>,
    universal: Vec<u32>,
    vars_dark: HashMap<String, String>,
    vars_light: HashMap<String, String>,
    hover_classes: HashSet<String>,
    hover_tags: HashSet<String>,
    pseudo_classes: HashMap<PseudoElement, HashSet<String>>,
    cache: Mutex<HashMap<u64, Arc<Computed>>>,
    cache_env: Mutex<Option<Env>>,
    unsupported: Mutex<BTreeSet<String>>,
}

fn is_theme_root(selector: &Selector, light: bool) -> bool {
    if selector.parts.len() != 1 || selector.pseudo_element.is_some() {
        return false;
    }
    let compound = &selector.parts[0].1;
    if !compound.classes.is_empty() || compound.tag.is_some() || compound.id.is_some() {
        return false;
    }
    if light {
        compound.attrs.iter().any(|(name, value)| name == "data-theme" && value.as_deref() == Some("light"))
    } else {
        compound.attrs.is_empty() && compound.pseudos.iter().any(|p| matches!(p, Pseudo::State(flag) if *flag == super::selector::ROOT))
    }
}

impl Sheet {
    pub fn new(css: &str) -> Self {
        let mut sheet = Self {
            rules: Vec::new(),
            by_class: HashMap::new(),
            by_tag: HashMap::new(),
            by_id: HashMap::new(),
            universal: Vec::new(),
            vars_dark: HashMap::new(),
            vars_light: HashMap::new(),
            hover_classes: HashSet::new(),
            hover_tags: HashSet::new(),
            pseudo_classes: HashMap::new(),
            cache: Mutex::new(HashMap::new()),
            cache_env: Mutex::new(None),
            unsupported: Mutex::new(BTreeSet::new()),
        };
        let mut order = 0u32;
        for raw in parse(css) {
            let decls: Arc<[RawDecl]> = raw.decls.into();
            for text in &raw.selectors {
                let selector = parse_selector(text);
                if selector.parts.is_empty() {
                    continue;
                }
                for light in [false, true] {
                    if is_theme_root(&selector, light) {
                        for decl in decls.iter().filter(|d| d.prop.starts_with("--")) {
                            if !light {
                                sheet.vars_dark.insert(decl.prop.clone(), decl.value.clone());
                            }
                            sheet.vars_light.insert(decl.prop.clone(), decl.value.clone());
                        }
                    }
                }
                for (_, compound) in &selector.parts {
                    if compound.pseudos.iter().any(|p| matches!(p, Pseudo::State(flag) if *flag == HOVER)) {
                        sheet.hover_classes.extend(compound.classes.iter().cloned());
                        if compound.classes.is_empty() {
                            if let Some(tag) = &compound.tag {
                                sheet.hover_tags.insert(tag.clone());
                            }
                        }
                    }
                }
                let index = sheet.rules.len() as u32;
                let last = &selector.parts[selector.parts.len() - 1].1;
                if let Some(element) = selector.pseudo_element {
                    let entry = sheet.pseudo_classes.entry(element).or_default();
                    entry.extend(last.classes.iter().cloned());
                    if let Some(tag) = &last.tag {
                        entry.insert(tag.clone());
                    }
                    let scrollbar = matches!(element, PseudoElement::Scrollbar | PseudoElement::ScrollbarThumb | PseudoElement::ScrollbarTrack);
                    if scrollbar && last.classes.is_empty() && last.tag.is_none() && last.id.is_none() {
                        entry.insert("*".to_string());
                    }
                }
                if let Some(id) = &last.id {
                    sheet.by_id.entry(id.clone()).or_default().push(index);
                } else if let Some(class) = last.classes.first() {
                    sheet.by_class.entry(class.clone()).or_default().push(index);
                } else if let Some(tag) = &last.tag {
                    sheet.by_tag.entry(tag.clone()).or_default().push(index);
                } else {
                    sheet.universal.push(index);
                }
                sheet.rules.push(Rule { selector, decls: decls.clone(), media_max_width: raw.media_max_width, order });
                order += 1;
            }
        }
        sheet
    }

    pub fn var(&self, name: &str, light: bool) -> Option<String> {
        let vars = if light { &self.vars_light } else { &self.vars_dark };
        vars.get(name).cloned()
    }

    pub fn hover_sensitive(&self, node: &NodeInfo) -> bool {
        self.hover_tags.contains(node.tag) || node.classes.iter().any(|c| self.hover_classes.contains(*c))
    }

    pub fn may_have(&self, element: PseudoElement, node: &NodeInfo) -> bool {
        self.pseudo_classes
            .get(&element)
            .is_some_and(|names| names.contains("*") || names.contains(node.tag) || node.classes.iter().any(|c| names.contains(*c)))
    }

    pub fn unsupported(&self) -> Vec<String> {
        self.unsupported.lock().unwrap_or_else(|e| e.into_inner()).iter().cloned().collect()
    }

    fn candidates(&self, node: &NodeInfo) -> Vec<u32> {
        let mut out: Vec<u32> = self.universal.clone();
        if let Some(list) = self.by_tag.get(node.tag) {
            out.extend(list);
        }
        if let Some(list) = node.id.and_then(|id| self.by_id.get(id)) {
            out.extend(list);
        }
        for class in &node.classes {
            if let Some(list) = self.by_class.get(*class) {
                out.extend(list);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn compute(
        &self,
        chain: &Chain<'_>,
        pseudo: Option<PseudoElement>,
        parent: &Computed,
        signature: u64,
        env: Env,
    ) -> Arc<Computed> {
        {
            let mut cached_env = self.cache_env.lock().unwrap_or_else(|e| e.into_inner());
            if *cached_env != Some(env) {
                self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
                *cached_env = Some(env);
            }
        }
        let key = signature ^ (pseudo.map(|p| p as u64 + 1).unwrap_or(0)).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        if let Some(found) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return found.clone();
        }
        let computed = Arc::new(self.cascade(chain, pseudo, parent, env));
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(key, computed.clone());
        computed
    }

    fn cascade(&self, chain: &Chain<'_>, pseudo: Option<PseudoElement>, parent: &Computed, env: Env) -> Computed {
        let node = &chain.nodes[chain.nodes.len() - 1];
        let mut matched: Vec<&Rule> = self
            .candidates(node)
            .into_iter()
            .map(|i| &self.rules[i as usize])
            .filter(|rule| rule.selector.pseudo_element == pseudo)
            .filter(|rule| rule.media_max_width.is_none_or(|max| env.viewport[0] <= max))
            .filter(|rule| matches(&rule.selector, chain))
            .collect();
        matched.sort_by_key(|rule| (rule.selector.specificity, rule.order));

        let mut style = Computed::child_of(parent);
        if pseudo.is_none() {
            apply_tag_defaults(node.tag, &mut style);
            if node.tag == "input" {
                if let Some((_, kind)) = node.attrs.iter().find(|(name, _)| *name == "type") {
                    super::computed::apply_input_type_defaults(kind, &mut style);
                }
            }
        } else {
            style.display = super::computed::Display::Inline;
        }
        let lookup = |name: &str| self.var(name, env.light);
        let cx = ApplyCx { parent, viewport: env.viewport };
        let mut declarations: Vec<(&RawDecl, bool)> = Vec::new();
        for rule in &matched {
            declarations.extend(rule.decls.iter().filter(|d| !d.important).map(|d| (d, false)));
        }
        for rule in &matched {
            declarations.extend(rule.decls.iter().filter(|d| d.important).map(|d| (d, true)));
        }
        let first_pass = |prop: &str| prop == "font" || prop == "font-size";
        for early in [true, false] {
            for (decl, _) in &declarations {
                if first_pass(&decl.prop) != early {
                    continue;
                }
                let value = if decl.value.contains("var(") { resolve_vars(&decl.value, &lookup) } else { decl.value.clone() };
                if !style.apply(&decl.prop, &value, &cx) {
                    self.unsupported.lock().unwrap_or_else(|e| e.into_inner()).insert(format!("{}: {}", decl.prop, decl.value));
                }
            }
        }
        style
    }

    pub fn uses_sibling(&self) -> bool {
        self.rules.iter().any(|r| r.selector.parts.iter().any(|(c, _)| matches!(c, Combinator::Adjacent | Combinator::Sibling)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::computed::Display;
    use crate::style::selector::ROOT;
    use crate::style::value::Len;
    use egui::Color32;

    const CSS: &str = r#"
        :root { --accent: #58a6ff; --radius: 12px; --border: rgba(88, 166, 255, 0.1); }
        [data-theme="light"] { --accent: #0969da; }
        * { margin: 0; padding: 0; }
        body { color: #e6edf3; line-height: 1.6; }
        .card { border: 1px solid var(--border); border-radius: var(--radius); padding: 2rem; }
        .btn { padding: 0.6rem 1.25rem; font-size: 0.9rem; color: var(--accent); display: inline-flex; }
        .btn:hover { color: white; }
        .card .btn--small { font-size: 0.8rem; padding: 0.4rem 0.75rem; }
        .hidden { display: none !important; }
        @media (max-width: 600px) { .card { padding: 1rem; } }
    "#;

    fn node(tag: &'static str, classes: &[&'static str], state: u16) -> NodeInfo {
        NodeInfo { tag, id: None, classes: classes.to_vec(), attrs: vec![], state }
    }

    fn compute(sheet: &Sheet, nodes: &[NodeInfo], light: bool, width: f32) -> Vec<Arc<Computed>> {
        let env = Env { light, viewport: [width, 800.0] };
        let previous: Vec<Option<NodeInfo>> = nodes.iter().map(|_| None).collect();
        let mut parent = Arc::new(Computed::root());
        let mut out = Vec::new();
        for i in 0..nodes.len() {
            let chain = Chain { nodes: &nodes[..=i], previous: &previous[..=i] };
            let signature = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                nodes[..=i].hash(&mut h);
                h.finish()
            };
            parent = sheet.compute(&chain, None, &parent, signature, env);
            out.push(parent.clone());
        }
        out
    }

    #[test]
    fn cascades_inherits_and_switches_theme() {
        let sheet = Sheet::new(CSS);
        let html = NodeInfo { tag: "html", state: ROOT, ..Default::default() };
        let nodes = [html.clone(), node("body", &[], 0), node("div", &["card"], 0), node("button", &["btn", "btn--small"], 0)];
        let styles = compute(&sheet, &nodes, false, 1280.0);
        let card = &styles[2];
        assert_eq!(card.padding[0], Len::Px(32.0));
        assert_eq!(card.radius[0], Len::Px(12.0));
        assert_eq!(card.border(0), 1.0);
        assert_eq!(card.color, Color32::from_rgb(0xe6, 0xed, 0xf3));
        let button = &styles[3];
        assert_eq!(button.font_size, 12.8);
        assert_eq!(button.padding[1], Len::Px(12.0));
        assert_eq!(button.color, Color32::from_rgb(0x58, 0xa6, 0xff));
        assert_eq!(button.display, Display::InlineFlex);

        let mut hovered = nodes.clone();
        hovered[3].state = HOVER;
        assert_eq!(compute(&sheet, &hovered, false, 1280.0)[3].color, Color32::WHITE);

        let light_html = NodeInfo { attrs: vec![("data-theme", "light")], ..html };
        let light = [light_html, nodes[1].clone(), nodes[2].clone(), nodes[3].clone()];
        assert_eq!(compute(&sheet, &light, true, 1280.0)[3].color, Color32::from_rgb(0x09, 0x69, 0xda));

        let mut hidden = nodes.clone();
        hidden[3].classes.push("hidden");
        assert_eq!(compute(&sheet, &hidden, false, 1280.0)[3].display, Display::None);
        assert_eq!(compute(&sheet, &nodes, false, 500.0)[2].padding[0], Len::Px(16.0));
        assert!(sheet.unsupported().is_empty(), "{:?}", sheet.unsupported());
    }

    #[test]
    fn understands_the_whole_application_stylesheet() {
        let css = include_str!("../../assets/style.css");
        let sheet = Sheet::new(css);
        assert!(sheet.rules.len() > 900);
        assert_eq!(sheet.var("--accent", false).as_deref(), Some("#58a6ff"));
        assert_eq!(sheet.var("--accent", true).as_deref(), Some("#0969da"));
        let parent = Computed::root();
        let cx = ApplyCx { parent: &parent, viewport: [1280.0, 860.0] };
        let mut failed = BTreeSet::new();
        for light in [false, true] {
            let lookup = |name: &str| sheet.var(name, light);
            for rule in &sheet.rules {
                for decl in rule.decls.iter() {
                    let mut style = Computed::child_of(&parent);
                    let value = resolve_vars(&decl.value, &lookup);
                    if !style.apply(&decl.prop, &value, &cx) {
                        failed.insert(format!("{}: {}", decl.prop, decl.value));
                    }
                }
            }
        }
        let unexpected: Vec<&String> = failed.iter().collect();
        assert!(unexpected.is_empty(), "declarations the engine does not understand yet:\n{}", unexpected.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n"));
    }
}
