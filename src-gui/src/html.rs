#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub code: bool,
    pub italic: bool,
    pub link: Option<String>,
    pub link_class: Option<String>,
}

pub fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&times;", "\u{d7}")
        .replace("&amp;", "&")
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{}=\"", name);
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(decode_entities(&tag[start..end]))
}

pub fn parse(html: &str) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    let (mut bold, mut code, mut italic) = (0u32, 0u32, 0u32);
    let mut link: Option<(String, Option<String>)> = None;
    let mut rest = html;
    let mut push = |text: &str, bold: u32, code: u32, italic: u32, link: &Option<(String, Option<String>)>| {
        if text.is_empty() {
            return;
        }
        let span = Span {
            text: decode_entities(text),
            bold: bold > 0,
            code: code > 0,
            italic: italic > 0,
            link: link.as_ref().map(|(target, _)| target.clone()),
            link_class: link.as_ref().and_then(|(_, class)| class.clone()),
        };
        match out.last_mut() {
            Some(last) if last.bold == span.bold && last.code == span.code && last.italic == span.italic && last.link == span.link && last.link_class == span.link_class => {
                last.text.push_str(&span.text)
            }
            _ => out.push(span),
        }
    };
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else { break };
        let close = open + close;
        push(&rest[..open], bold, code, italic, &link);
        let tag = rest[open + 1..close].trim();
        let closing = tag.starts_with('/');
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
        let bump = |v: &mut u32| {
            if closing {
                *v = v.saturating_sub(1)
            } else {
                *v += 1
            }
        };
        match name.as_str() {
            "strong" | "b" => bump(&mut bold),
            "code" | "kbd" => bump(&mut code),
            "em" | "i" => bump(&mut italic),
            "br" => push("\n", bold, code, italic, &link),
            "a" => {
                link = if closing {
                    None
                } else {
                    let target = attr(tag, "data-click").or_else(|| attr(tag, "href").map(|href| format!("open:{href}"))).unwrap_or_default();
                    Some((target, attr(tag, "class")))
                }
            }
            _ => {}
        }
        rest = &rest[close + 1..];
    }
    push(rest, bold, code, italic, &link);
    out
}

pub fn plain(html: &str) -> String {
    parse(html).into_iter().map(|s| s.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_markup_into_styled_spans() {
        let spans = parse("Drop <strong>.lua</strong>, <code>a&amp;b</code> or <a href=\"https://x.y\">this</a><br>now");
        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["Drop ", ".lua", ", ", "a&b", " or ", "this", "\nnow"]);
        assert!(spans[1].bold && !spans[0].bold);
        assert!(spans[3].code);
        assert_eq!(spans[5].link.as_deref(), Some("open:https://x.y"));
        assert_eq!(plain("a <b>b</b> c"), "a b c");
    }
}
