use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};

use egui::text::{LayoutJob, TextFormat};

const FIT_SLACK: f32 = 0.5;
const ZERO_WIDTH_SPACE: char = '\u{200b}';
const NO_BREAK_SPACE: char = '\u{a0}';

#[derive(Clone)]
struct Piece {
    text: String,
    format: TextFormat,
}

#[derive(Clone)]
struct Token {
    pieces: Vec<Piece>,
    width: f32,
    space_after: bool,
    space_width: f32,
    newline_after: bool,
}

fn width_cache() -> &'static Mutex<HashMap<u64, f32>> {
    static CACHE: OnceLock<Mutex<HashMap<u64, f32>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn measure(ctx: &egui::Context, text: &str, format: &TextFormat) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    format.hash(&mut hasher);
    ctx.pixels_per_point().to_bits().hash(&mut hasher);
    let key = hasher.finish();
    if let Some(width) = width_cache().lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return *width;
    }
    let mut job = LayoutJob::default();
    job.append(text, 0.0, format.clone());
    job.wrap.max_width = f32::INFINITY;
    job.round_output_to_gui = false;
    let width = ctx.fonts_mut(|f| f.layout_job(job)).size().x;
    let mut cache = width_cache().lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() > 20_000 {
        cache.clear();
    }
    cache.insert(key, width);
    width
}

pub fn space_width(ctx: &egui::Context, format: &TextFormat) -> f32 {
    measure(ctx, " ", format)
}

pub const SPACER: &str = "\u{a0}";

pub fn spacer(ctx: &egui::Context, width: f32, like: &TextFormat) -> TextFormat {
    let mut format = like.clone();
    format.color = egui::Color32::TRANSPARENT;
    format.underline = egui::Stroke::NONE;
    format.extra_letter_spacing = 0.0;
    format.font_id.size = 20.0;
    let reference = measure(ctx, SPACER, &format).max(0.01);
    format.font_id.size = (width * 20.0 / reference).max(0.5);
    format
}

fn breaks_after(previous: Option<char>, current: char, next: Option<char>) -> bool {
    let Some(next) = next else { return false };
    if next.is_whitespace() {
        return false;
    }
    match current {
        '-' | '/' => previous.is_some_and(|p| p.is_alphanumeric()) && next.is_alphabetic(),
        '\u{2013}' | '\u{2014}' => true,
        ZERO_WIDTH_SPACE => true,
        _ => false,
    }
}

fn tokenize(ctx: &egui::Context, job: &LayoutJob) -> Vec<Token> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut current: Vec<Piece> = Vec::new();
    let mut last_format: Option<TextFormat> = None;
    let chars: Vec<(char, usize)> = job
        .sections
        .iter()
        .enumerate()
        .flat_map(|(index, section)| job.text[section.byte_range.start.0..section.byte_range.end.0].chars().map(move |c| (c, index)))
        .collect();
    let finish = |pieces: &mut Vec<Piece>, tokens: &mut Vec<Token>, space: bool, newline: bool, format: Option<&TextFormat>| {
        if pieces.is_empty() && !newline {
            if space {
                if let Some(last) = tokens.last_mut() {
                    last.space_after = true;
                }
            }
            return;
        }
        let width = pieces.iter().map(|p| measure(ctx, &p.text, &p.format)).sum();
        let space_format = pieces.last().map(|p| &p.format).or(format);
        let space_width = space_format.map(|f| measure(ctx, " ", f)).unwrap_or(0.0);
        tokens.push(Token { pieces: std::mem::take(pieces), width, space_after: space, space_width, newline_after: newline });
    };
    for (position, (chr, section)) in chars.iter().enumerate() {
        let format = &job.sections[*section].format;
        last_format = Some(format.clone());
        if *chr == '\n' {
            finish(&mut current, &mut tokens, false, true, Some(format));
            continue;
        }
        if chr.is_whitespace() && *chr != NO_BREAK_SPACE {
            finish(&mut current, &mut tokens, true, false, Some(format));
            continue;
        }
        if *chr != ZERO_WIDTH_SPACE {
            match current.last_mut() {
                Some(piece) if piece.format == *format => piece.text.push(*chr),
                _ => current.push(Piece { text: chr.to_string(), format: format.clone() }),
            }
        }
        let previous = chars[..position].iter().rev().map(|c| c.0).find(|c| *c != NO_BREAK_SPACE);
        let next = chars.get(position + 1).map(|c| c.0);
        if breaks_after(previous, *chr, next) {
            finish(&mut current, &mut tokens, false, false, Some(format));
        }
    }
    finish(&mut current, &mut tokens, false, false, last_format.as_ref());
    tokens
}

pub struct Wrapped {
    pub job: LayoutJob,
    pub lines: usize,
    pub width: f32,
    pub min_content: f32,
    pub max_content: f32,
}

pub fn wrap(ctx: &egui::Context, job: &LayoutJob, max_width: f32) -> Wrapped {
    let tokens = tokenize(ctx, job);
    let mut out = LayoutJob { halign: job.halign, justify: job.justify, first_row_min_height: job.first_row_min_height, ..Default::default() };
    out.wrap.max_width = f32::INFINITY;
    let mut lines = 1usize;
    let mut widest = 0.0f32;
    let mut line_width = 0.0f32;
    let mut pending_space: Option<(f32, TextFormat)> = None;
    let mut min_content = 0.0f32;
    let mut max_content = 0.0f32;
    let mut unbroken = 0.0f32;
    let mut at_line_start = true;
    for token in &tokens {
        min_content = min_content.max(token.width);
        let space = pending_space.as_ref().map(|s| s.0).unwrap_or(0.0);
        unbroken += space + token.width;
        if !at_line_start && line_width + space + token.width > max_width + FIT_SLACK {
            out.append("\n", 0.0, token.pieces.first().map(|p| p.format.clone()).unwrap_or_default());
            lines += 1;
            widest = widest.max(line_width);
            line_width = 0.0;
            pending_space = None;
        }
        if let Some((width, format)) = pending_space.take() {
            out.append(" ", 0.0, format);
            line_width += width;
        }
        for piece in &token.pieces {
            out.append(&piece.text, 0.0, piece.format.clone());
        }
        line_width += token.width;
        if token.newline_after {
            let format = token.pieces.last().map(|p| p.format.clone()).unwrap_or_default();
            out.append("\n", 0.0, format);
            lines += 1;
            widest = widest.max(line_width);
            max_content = max_content.max(unbroken);
            unbroken = 0.0;
            line_width = 0.0;
            pending_space = None;
            at_line_start = true;
        } else {
            at_line_start = false;
            if token.space_after {
                pending_space = token.pieces.last().map(|p| (token.space_width, p.format.clone()));
            }
        }
    }
    widest = widest.max(line_width);
    max_content = max_content.max(unbroken);
    Wrapped { job: out, lines, width: widest, min_content, max_content }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_break_opportunities_like_a_browser() {
        assert!(breaks_after(Some('k'), '-', Some('K')));
        assert!(!breaks_after(Some(' '), '-', Some('x')));
        assert!(!breaks_after(Some('a'), '-', Some('5')));
        assert!(breaks_after(Some('a'), '/', Some('b')));
        assert!(!breaks_after(Some('a'), '.', Some('b')));
        assert!(!breaks_after(Some('a'), '-', None));
    }
}
