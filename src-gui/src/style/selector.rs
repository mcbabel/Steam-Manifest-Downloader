use super::parse::split_top;

pub const HOVER: u16 = 1 << 0;
pub const ACTIVE: u16 = 1 << 1;
pub const FOCUS: u16 = 1 << 2;
pub const FOCUS_VISIBLE: u16 = 1 << 3;
pub const FOCUS_WITHIN: u16 = 1 << 4;
pub const DISABLED: u16 = 1 << 5;
pub const CHECKED: u16 = 1 << 6;
pub const EMPTY: u16 = 1 << 7;
pub const FIRST_CHILD: u16 = 1 << 8;
pub const LAST_CHILD: u16 = 1 << 9;
pub const FIRST_OF_TYPE: u16 = 1 << 10;
pub const HAS_CHECKED: u16 = 1 << 11;
pub const ROOT: u16 = 1 << 12;

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodeInfo {
    pub tag: &'static str,
    pub id: Option<&'static str>,
    pub classes: Vec<&'static str>,
    pub attrs: Vec<(&'static str, &'static str)>,
    pub state: u16,
}

impl NodeInfo {
    pub fn has_class(&self, class: &str) -> bool {
        self.classes.iter().any(|c| c == &class)
    }

    pub fn is(&self, flag: u16) -> bool {
        self.state & flag != 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pseudo {
    State(u16),
    Not(Box<Compound>),
    HasChecked,
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    Before,
    After,
    Placeholder,
    Scrollbar,
    ScrollbarThumb,
    ScrollbarTrack,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, Option<String>)>,
    pub pseudos: Vec<Pseudo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    pub parts: Vec<(Combinator, Compound)>,
    pub pseudo_element: Option<PseudoElement>,
    pub specificity: u32,
}

fn ident_end(s: &str) -> usize {
    s.char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '-' || *c == '_'))
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn paren_end(s: &str) -> usize {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    s.len().saturating_sub(1)
}

fn parse_compound(text: &str) -> (Compound, Option<PseudoElement>) {
    let mut compound = Compound::default();
    let mut element = None;
    let mut rest = text.trim();
    while !rest.is_empty() {
        let first = rest.chars().next().unwrap();
        match first {
            '*' => rest = &rest[1..],
            '.' => {
                let end = ident_end(&rest[1..]) + 1;
                compound.classes.push(rest[1..end].to_string());
                rest = &rest[end..];
            }
            '#' => {
                let end = ident_end(&rest[1..]) + 1;
                compound.id = Some(rest[1..end].to_string());
                rest = &rest[end..];
            }
            '[' => {
                let end = rest.find(']').unwrap_or(rest.len() - 1);
                let inner = &rest[1..end];
                let attr = match inner.split_once('=') {
                    Some((name, value)) => (name.trim().to_string(), Some(value.trim().trim_matches(|c| c == '"' || c == '\'').to_string())),
                    None => (inner.trim().to_string(), None),
                };
                compound.attrs.push(attr);
                rest = &rest[end + 1..];
            }
            ':' => {
                let double = rest.starts_with("::");
                let body = &rest[if double { 2 } else { 1 }..];
                let end = ident_end(body);
                let name = &body[..end];
                let mut consumed = (if double { 2 } else { 1 }) + end;
                let mut argument = None;
                if body[end..].starts_with('(') {
                    let close = paren_end(&body[end..]);
                    argument = Some(body[end + 1..end + close].trim().to_string());
                    consumed += close + 1;
                }
                if double || matches!(name, "before" | "after") {
                    element = Some(match name {
                        "before" => PseudoElement::Before,
                        "after" => PseudoElement::After,
                        "placeholder" => PseudoElement::Placeholder,
                        "-webkit-scrollbar" => PseudoElement::Scrollbar,
                        "-webkit-scrollbar-thumb" => PseudoElement::ScrollbarThumb,
                        "-webkit-scrollbar-track" => PseudoElement::ScrollbarTrack,
                        _ => PseudoElement::Other,
                    });
                } else {
                    compound.pseudos.push(match name {
                        "hover" => Pseudo::State(HOVER),
                        "active" => Pseudo::State(ACTIVE),
                        "focus" => Pseudo::State(FOCUS),
                        "focus-visible" => Pseudo::State(FOCUS_VISIBLE),
                        "focus-within" => Pseudo::State(FOCUS_WITHIN),
                        "disabled" => Pseudo::State(DISABLED),
                        "checked" => Pseudo::State(CHECKED),
                        "empty" => Pseudo::State(EMPTY),
                        "first-child" => Pseudo::State(FIRST_CHILD),
                        "last-child" => Pseudo::State(LAST_CHILD),
                        "first-of-type" => Pseudo::State(FIRST_OF_TYPE),
                        "root" => Pseudo::State(ROOT),
                        "not" => Pseudo::Not(Box::new(parse_compound(argument.as_deref().unwrap_or("")).0)),
                        "has" if argument.as_deref().is_some_and(|a| a.contains(":checked")) => Pseudo::HasChecked,
                        _ => Pseudo::Never,
                    });
                }
                rest = &rest[consumed..];
            }
            _ => {
                let end = ident_end(rest).max(1);
                compound.tag = Some(rest[..end].to_ascii_lowercase());
                rest = &rest[end..];
            }
        }
    }
    (compound, element)
}

fn specificity_of(compound: &Compound) -> u32 {
    let mut value = 0;
    if compound.id.is_some() {
        value += 1 << 16;
    }
    value += ((compound.classes.len() + compound.attrs.len()) as u32) << 8;
    for pseudo in &compound.pseudos {
        match pseudo {
            Pseudo::Not(inner) => value += specificity_of(inner),
            _ => value += 1 << 8,
        }
    }
    if compound.tag.is_some() {
        value += 1;
    }
    value
}

pub fn parse_selector(text: &str) -> Selector {
    let mut spaced = String::with_capacity(text.len() + 8);
    let mut depth = 0i32;
    for c in text.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                spaced.push(c);
            }
            ')' | ']' => {
                depth -= 1;
                spaced.push(c);
            }
            '>' | '+' | '~' if depth == 0 => {
                spaced.push(' ');
                spaced.push(c);
                spaced.push(' ');
            }
            _ => spaced.push(c),
        }
    }
    let mut parts = Vec::new();
    let mut pseudo_element = None;
    let mut combinator = Combinator::Descendant;
    for token in split_top(&spaced, ' ') {
        match token.as_str() {
            ">" => combinator = Combinator::Child,
            "+" => combinator = Combinator::Adjacent,
            "~" => combinator = Combinator::Sibling,
            "" => {}
            _ => {
                let (compound, element) = parse_compound(&token);
                if element.is_some() {
                    pseudo_element = element;
                }
                parts.push((combinator, compound));
                combinator = Combinator::Descendant;
            }
        }
    }
    let mut specificity = parts.iter().map(|(_, c)| specificity_of(c)).sum();
    if pseudo_element.is_some() {
        specificity += 1;
    }
    Selector { parts, pseudo_element, specificity }
}

pub fn matches_compound(compound: &Compound, node: &NodeInfo) -> bool {
    if let Some(tag) = &compound.tag {
        if tag != node.tag {
            return false;
        }
    }
    if let Some(id) = &compound.id {
        if node.id != Some(id.as_str()) {
            return false;
        }
    }
    if !compound.classes.iter().all(|c| node.has_class(c)) {
        return false;
    }
    for (name, value) in &compound.attrs {
        let found = node.attrs.iter().find(|(n, _)| n == name);
        match (found, value) {
            (None, _) => return false,
            (Some((_, actual)), Some(expected)) if actual != expected => return false,
            _ => {}
        }
    }
    for pseudo in &compound.pseudos {
        let ok = match pseudo {
            Pseudo::State(flag) => node.is(*flag),
            Pseudo::Not(inner) => !matches_compound(inner, node),
            Pseudo::HasChecked => node.is(HAS_CHECKED),
            Pseudo::Never => false,
        };
        if !ok {
            return false;
        }
    }
    true
}

pub struct Chain<'a> {
    pub nodes: &'a [NodeInfo],
    pub previous: &'a [Option<NodeInfo>],
}

fn matches_from(selector: &Selector, part: usize, chain: &Chain<'_>, at: usize) -> bool {
    let (combinator, compound) = &selector.parts[part];
    if !matches_compound(compound, &chain.nodes[at]) {
        return false;
    }
    if part == 0 {
        return true;
    }
    match combinator {
        Combinator::Child => at > 0 && matches_from(selector, part - 1, chain, at - 1),
        Combinator::Descendant => (0..at).rev().any(|i| matches_from(selector, part - 1, chain, i)),
        Combinator::Adjacent | Combinator::Sibling => match chain.previous.get(at).and_then(|p| p.as_ref()) {
            Some(previous) => {
                let (_, before) = &selector.parts[part - 1];
                if !matches_compound(before, previous) {
                    return false;
                }
                if part == 1 {
                    return true;
                }
                let (outer, _) = &selector.parts[part - 1];
                match outer {
                    Combinator::Child => at > 0 && matches_from(selector, part - 2, chain, at - 1),
                    _ => (0..at).rev().any(|i| matches_from(selector, part - 2, chain, i)),
                }
            }
            None => false,
        },
    }
}

pub fn matches(selector: &Selector, chain: &Chain<'_>) -> bool {
    if selector.parts.is_empty() || chain.nodes.is_empty() {
        return false;
    }
    matches_from(selector, selector.parts.len() - 1, chain, chain.nodes.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(tag: &'static str, classes: &[&'static str], state: u16) -> NodeInfo {
        NodeInfo { tag, id: None, classes: classes.to_vec(), attrs: vec![], state }
    }

    #[test]
    fn matches_descendants_children_states_and_siblings() {
        let root = NodeInfo { tag: "html", attrs: vec![("data-theme", "light")], state: ROOT, ..Default::default() };
        let zone = node("div", &["drop-zone"], HOVER);
        let icon = node("svg", &["drop-zone__icon"], 0);
        let nodes = [root.clone(), zone.clone(), icon.clone()];
        let previous = [None, None, None];
        let chain = Chain { nodes: &nodes, previous: &previous };
        assert!(matches(&parse_selector(".drop-zone:hover .drop-zone__icon"), &chain));
        assert!(matches(&parse_selector("[data-theme=\"light\"] .drop-zone > svg"), &chain));
        assert!(!matches(&parse_selector(".drop-zone.drag-over .drop-zone__icon"), &chain));
        assert!(!matches(&parse_selector("html > .drop-zone__icon"), &chain));
        assert!(matches(&parse_selector(".drop-zone__icon:not(.is-active)"), &chain));

        let input = NodeInfo { tag: "input", attrs: vec![("type", "checkbox")], state: CHECKED, ..Default::default() };
        let slider = node("span", &["toggle-switch__slider"], 0);
        let nodes = [root, node("label", &["toggle-switch"], 0), slider];
        let previous = [None, None, Some(input)];
        let chain = Chain { nodes: &nodes, previous: &previous };
        let sel = parse_selector(".toggle-switch input:checked + .toggle-switch__slider::before");
        assert_eq!(sel.pseudo_element, Some(PseudoElement::Before));
        assert!(matches(&sel, &chain));
        assert!(!matches(&parse_selector(".other input:checked + .toggle-switch__slider"), &chain));
    }

    #[test]
    fn orders_selectors_by_specificity() {
        let a = parse_selector(".btn");
        let b = parse_selector(".btn--primary:hover");
        let c = parse_selector("#x .btn");
        let d = parse_selector("button");
        assert!(d.specificity < a.specificity && a.specificity < b.specificity && b.specificity < c.specificity);
    }
}
