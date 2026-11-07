use std::borrow::Cow;
use std::sync::OnceLock;

use ratatui::buffer::Buffer;

static ASCII: OnceLock<bool> = OnceLock::new();

fn legacy_console() -> bool {
    if !cfg!(windows) {
        return false;
    }
    let set = |k: &str| std::env::var_os(k).is_some_and(|v| !v.is_empty());
    !(set("WT_SESSION") || set("TERM_PROGRAM") || set("ConEmuANSI") || set("ALACRITTY_LOG") || set("WEZTERM_PANE") || set("TERM"))
}

pub fn ascii() -> bool {
    *ASCII.get_or_init(|| match std::env::var("SMD_GLYPHS").map(|v| v.to_ascii_lowercase()).as_deref() {
        Ok("ascii") => true,
        Ok("unicode") => false,
        _ => legacy_console(),
    })
}

pub fn fallback(c: char) -> Option<&'static str> {
    Some(match c {
        '✓' | '✔' => "+",
        '✗' | '✕' | '✘' => "x",
        '●' | '◆' | '★' => "*",
        '○' => "o",
        '▸' | '▶' => ">",
        '◂' => "<",
        '↻' => "~",
        '⇡' => "^",
        '⇣' => "v",
        '‖' => "|",
        '↰' => "<",
        'ℹ' => "i",
        '┃' => "|",
        '▏' | '▎' | '▍' => "|",
        '▋' | '▊' | '▉' => "#",
        '⠋' | '⠸' => "|",
        '⠙' | '⠼' => "/",
        '⠹' | '⠴' => "-",
        '⠦' | '⠧' | '⠇' | '⠏' => "\\",
        '🎉' | '🖥' | '📦' => "*",
        '\u{fe0f}' => "",
        _ => return None,
    })
}

pub fn text(s: &str) -> Cow<'_, str> {
    if !ascii() || !s.chars().any(|c| fallback(c).is_some()) {
        return Cow::Borrowed(s);
    }
    Cow::Owned(map(s))
}

fn map(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match fallback(c) {
            Some(r) => out.push_str(r),
            None => out.push(c),
        }
    }
    out
}

pub fn apply(buf: &mut Buffer) {
    if !ascii() {
        return;
    }
    for cell in buf.content.iter_mut() {
        let sym = cell.symbol();
        if sym.chars().any(|c| fallback(c).is_some()) {
            let mapped = map(sym);
            cell.set_symbol(if mapped.is_empty() { " " } else { &mapped });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_what_the_old_console_cannot_draw() {
        assert_eq!(map("✓ depot 3591 done · 2× ✗"), "+ depot 3591 done · 2× x");
        assert_eq!(map("⠋⠙⠹"), "|/-");
        assert_eq!(map("plain text — ok"), "plain text — ok");
        assert_eq!(fallback('a'), None);
    }
}
