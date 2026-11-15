#[derive(Clone, Debug, PartialEq)]
pub struct RawDecl {
    pub prop: String,
    pub value: String,
    pub important: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawRule {
    pub selectors: Vec<String>,
    pub decls: Vec<RawDecl>,
    pub media_max_width: Option<f32>,
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn split_top(text: &str, separator: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut current = String::new();
    for c in text.chars() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
                current.push(c);
            }
            None => match c {
                '"' | '\'' => {
                    quote = Some(c);
                    current.push(c);
                }
                '(' | '[' => {
                    depth += 1;
                    current.push(c);
                }
                ')' | ']' => {
                    depth -= 1;
                    current.push(c);
                }
                c if c == separator && depth == 0 => {
                    out.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(c),
            },
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

fn block_end(text: &str, open: usize) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut quote: Option<u8> = None;
    let mut i = open;
    while i < bytes.len() {
        let c = bytes[i];
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                b'"' | b'\'' => quote = Some(c),
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return i;
                    }
                }
                _ => {}
            },
        }
        i += 1;
    }
    text.len().saturating_sub(1)
}

fn parse_decls(body: &str) -> Vec<RawDecl> {
    split_top(body, ';')
        .into_iter()
        .filter_map(|decl| {
            let colon = decl.find(':')?;
            let prop = decl[..colon].trim().to_ascii_lowercase();
            let mut value = decl[colon + 1..].trim().to_string();
            let important = value.ends_with("!important");
            if important {
                value = value[..value.len() - "!important".len()].trim().to_string();
            }
            if prop.is_empty() || value.is_empty() {
                return None;
            }
            let prop = if prop.starts_with("--") { decl[..colon].trim().to_string() } else { prop };
            Some(RawDecl { prop, value: value.split_whitespace().collect::<Vec<_>>().join(" "), important })
        })
        .collect()
}

fn media_max_width(prelude: &str) -> Option<f32> {
    let start = prelude.find("max-width")?;
    let rest = &prelude[start..];
    let colon = rest.find(':')?;
    let end = rest.find("px")?;
    rest[colon + 1..end].trim().parse().ok()
}

fn parse_block(text: &str, media: Option<f32>, out: &mut Vec<RawRule>) {
    let mut pos = 0;
    while let Some(rel) = text[pos..].find('{') {
        let open = pos + rel;
        let prelude = text[pos..open].trim();
        let close = block_end(text, open);
        let body = &text[open + 1..close.max(open + 1)];
        if prelude.starts_with("@media") {
            parse_block(body, media_max_width(prelude).or(Some(f32::INFINITY)), out);
        } else if prelude.starts_with('@') {
        } else if !prelude.is_empty() {
            let selectors = split_top(&prelude.split_whitespace().collect::<Vec<_>>().join(" "), ',');
            out.push(RawRule { selectors, decls: parse_decls(body), media_max_width: media });
        }
        pos = close + 1;
        if pos >= text.len() {
            break;
        }
    }
}

pub fn parse(css: &str) -> Vec<RawRule> {
    let clean = strip_comments(css);
    let mut out = Vec::new();
    parse_block(&clean, None, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rules_media_and_tricky_values() {
        let css = r#"
            /* note */
            .a, .b:hover > .c { color: red; padding: 1px 2px !important; }
            @keyframes spin { to { transform: rotate(360deg); } }
            @media (max-width: 600px) { .d { display: none; } }
            .e::before { content: ''; mask: url("data:image/svg+xml;utf8,<svg a='b;c'/>") no-repeat; }
            :root { --Font: 'Inter', sans-serif; }
        "#;
        let rules = parse(css);
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].selectors, vec![".a", ".b:hover > .c"]);
        assert_eq!(rules[0].decls[1], RawDecl { prop: "padding".into(), value: "1px 2px".into(), important: true });
        assert_eq!(rules[1].media_max_width, Some(600.0));
        assert_eq!(rules[1].selectors, vec![".d"]);
        assert_eq!(rules[2].decls.len(), 2);
        assert!(rules[2].decls[1].value.contains("a='b;c'"));
        assert_eq!(rules[3].decls[0].prop, "--Font");
    }
}
