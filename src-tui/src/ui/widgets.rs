use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::input::TextInput;
use super::{Ctx, Fid, Kind};
use crate::app::action::{Action, InputId};
use crate::theme;

pub fn fill(buf: &mut Buffer, area: Rect, style: Style) {
    buf.set_style(area, style);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol(" ");
            }
        }
    }
}

pub fn width(s: &str) -> u16 {
    UnicodeWidthStr::width(s).min(u16::MAX as usize) as u16
}

pub fn truncate(s: &str, max: u16) -> String {
    let max = max as usize;
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > max {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

pub fn truncate_left(s: &str, max: u16) -> String {
    let max = max as usize;
    if UnicodeWidthStr::width(s) <= max {
        return s.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut rev = Vec::new();
    let mut used = 0;
    for c in s.chars().rev() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > max {
            break;
        }
        rev.push(c);
        used += w;
    }
    let mut out = String::from("…");
    out.extend(rev.into_iter().rev());
    out
}

pub fn text(buf: &mut Buffer, area: Rect, s: &str, style: Style) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    buf.set_stringn(
        area.x,
        area.y,
        truncate(s, area.width),
        area.width as usize,
        style,
    );
}

pub fn line(buf: &mut Buffer, area: Rect, line: Line) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    buf.set_line(area.x, area.y, &line, area.width);
}

pub fn text_right(buf: &mut Buffer, area: Rect, s: &str, style: Style) {
    let s = truncate(s, area.width);
    let w = width(&s);
    buf.set_string(area.x + area.width.saturating_sub(w), area.y, s, style);
}

pub fn wrap(s: &str, width: u16) -> Vec<String> {
    let width = width.max(1) as usize;
    let mut out = Vec::new();
    for para in s.split('\n') {
        let mut cur = String::new();
        let mut cur_w = 0usize;
        for word in para.split(' ') {
            let ww = UnicodeWidthStr::width(word);
            if cur_w > 0 && cur_w + 1 + ww > width {
                out.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
            if ww > width {
                for c in word.chars() {
                    let cw = c.width().unwrap_or(0);
                    if cur_w + cw > width {
                        out.push(std::mem::take(&mut cur));
                        cur_w = 0;
                    }
                    cur.push(c);
                    cur_w += cw;
                }
                continue;
            }
            if cur_w > 0 {
                cur.push(' ');
                cur_w += 1;
            }
            cur.push_str(word);
            cur_w += ww;
        }
        out.push(cur);
    }
    out
}

pub fn wrap_height(s: &str, width: u16) -> u16 {
    wrap(s, width).len().min(u16::MAX as usize) as u16
}

pub fn paragraph(buf: &mut Buffer, area: Rect, s: &str, style: Style) -> u16 {
    let lines = wrap(s, area.width);
    let mut used = 0;
    for (i, l) in lines.iter().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        buf.set_stringn(area.x, area.y + i as u16, l, area.width as usize, style);
        used += 1;
    }
    used
}

fn border_set() -> border::Set<'static> {
    border::ROUNDED
}

pub fn card(buf: &mut Buffer, area: Rect, title: Option<&str>, focused: bool) -> Rect {
    if area.width < 2 || area.height < 2 {
        return Rect::new(area.x, area.y, 0, 0);
    }
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_set(border_set())
        .border_style(theme::border(focused))
        .style(theme::base());
    if let Some(t) = title {
        let style = if focused {
            theme::accent_bold()
        } else {
            theme::text().add_modifier(Modifier::BOLD)
        };
        block = block.title(Line::from(Span::styled(format!(" {} ", t), style)));
    }
    let inner = block.inner(area);
    block.render(area, buf);
    inner
}

pub fn heading(buf: &mut Buffer, area: Rect, title: &str) {
    if area.height == 0 {
        return;
    }
    let title = truncate(title, area.width);
    let w = width(&title);
    buf.set_string(area.x, area.y, &title, theme::accent_bold());
    if area.width > w + 1 {
        let rule: String = "─".repeat((area.width - w - 1) as usize);
        buf.set_string(area.x + w + 1, area.y, rule, theme::border(false));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Primary,
    Secondary,
    Danger,
}

pub struct ButtonSpec {
    pub label: String,
    pub fid: Fid,
    pub action: Action,
    pub kind: Btn,
    pub enabled: bool,
}

impl ButtonSpec {
    pub fn new(label: impl Into<String>, fid: Fid, action: Action, kind: Btn) -> Self {
        ButtonSpec {
            label: label.into(),
            fid,
            action,
            kind,
            enabled: true,
        }
    }

    pub fn enabled(mut self, on: bool) -> Self {
        self.enabled = on;
        self
    }
}

pub fn button_width(label: &str) -> u16 {
    width(label) + 4
}

pub fn button(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    x: u16,
    y: u16,
    max_w: u16,
    spec: &ButtonSpec,
) -> u16 {
    let t = theme::get();
    let label = truncate(&spec.label, max_w.saturating_sub(4));
    let w = (width(&label) + 4).min(max_w);
    if w == 0 {
        return 0;
    }
    let rect = Rect::new(x, y, w, 1);
    let focused = ctx.is_focused(spec.fid) && spec.enabled;
    let hovered = ctx.is_hovered(rect) && spec.enabled;

    let style = if !spec.enabled {
        Style::default()
            .fg(t.text_muted)
            .bg(if t.truecolor { t.surface } else { Color::Reset })
    } else if t.truecolor {
        let (bg, fg) = match (spec.kind, focused || hovered) {
            (Btn::Primary, false) => (t.accent, t.on_accent),
            (Btn::Primary, true) => (t.purple, t.on_accent),
            (Btn::Secondary, false) => (t.surface_hi, t.text),
            (Btn::Secondary, true) => (t.accent, t.on_accent),
            (Btn::Danger, false) => (t.surface_hi, t.error),
            (Btn::Danger, true) => (t.error, t.on_accent),
        };
        let mut s = Style::default().bg(bg).fg(fg);
        if spec.kind == Btn::Primary || focused {
            s = s.add_modifier(Modifier::BOLD);
        }
        s
    } else {
        let fg = match spec.kind {
            Btn::Primary => t.accent,
            Btn::Secondary => t.text,
            Btn::Danger => t.error,
        };
        let mut s = Style::default().fg(fg).add_modifier(Modifier::BOLD);
        if focused || hovered {
            s = s.add_modifier(Modifier::REVERSED);
        }
        s
    };

    let body = if t.truecolor {
        format!("  {}  ", label)
    } else {
        format!("[ {} ]", label)
    };
    buf.set_stringn(x, y, &body, w as usize, style);
    if spec.enabled {
        ctx.focusable(spec.fid, rect, Kind::Button, Some(spec.action.clone()));
    }
    w
}

pub fn buttons(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    area: Rect,
    specs: &[ButtonSpec],
    align_right: bool,
) {
    if area.height == 0 {
        return;
    }
    let total: u16 = specs
        .iter()
        .map(|s| button_width(&s.label) + 2)
        .sum::<u16>()
        .saturating_sub(2);
    let mut x = if align_right && total < area.width {
        area.x + area.width - total
    } else {
        area.x
    };
    for spec in specs {
        let room = (area.x + area.width).saturating_sub(x);
        if room < 5 {
            break;
        }
        let w = button(buf, ctx, x, area.y, room, spec);
        x += w + 2;
    }
}

pub fn button_rows(specs: &[ButtonSpec], width: u16) -> u16 {
    let mut rows = 1;
    let mut used = 0u16;
    for s in specs {
        let w = button_width(&s.label);
        if used > 0 && used + w > width {
            rows += 1;
            used = 0;
        }
        used += w + 2;
    }
    rows
}

pub fn buttons_wrapped(buf: &mut Buffer, ctx: &mut Ctx, area: Rect, specs: &[ButtonSpec]) -> u16 {
    let mut x = area.x;
    let mut y = area.y;
    for spec in specs {
        let w = button_width(&spec.label);
        if x > area.x && x + w > area.right() {
            x = area.x;
            y += 1;
        }
        if y >= area.bottom() {
            break;
        }
        button(buf, ctx, x, y, area.right().saturating_sub(x), spec);
        x += w + 2;
    }
    y - area.y + 1
}

fn mark_style(focused: bool, hovered: bool) -> Style {
    let t = theme::get();
    let mut s = Style::default().fg(if focused { t.accent } else { t.text });
    if focused || hovered {
        s = s.add_modifier(Modifier::BOLD);
    }
    if focused && theme::monochrome() {
        s = s.add_modifier(Modifier::REVERSED);
    }
    s
}

#[allow(clippy::too_many_arguments)]
pub fn checkbox(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    area: Rect,
    checked: bool,
    label: &str,
    fid: Fid,
    action: Action,
    enabled: bool,
) -> u16 {
    toggle_like(
        buf,
        ctx,
        area,
        if checked { "[✓]" } else { "[ ]" },
        checked,
        label,
        fid,
        action,
        enabled,
    )
}

pub fn radio(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    area: Rect,
    selected: bool,
    label: &str,
    fid: Fid,
    action: Action,
) -> u16 {
    toggle_like(
        buf,
        ctx,
        area,
        if selected { "(●)" } else { "( )" },
        selected,
        label,
        fid,
        action,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn toggle_like(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    area: Rect,
    mark: &str,
    on: bool,
    label: &str,
    fid: Fid,
    action: Action,
    enabled: bool,
) -> u16 {
    if area.height == 0 || area.width < 4 {
        return 0;
    }
    let label = truncate(label, area.width.saturating_sub(4));
    let w = (width(&label) + 4).min(area.width);
    let rect = Rect::new(area.x, area.y, w, 1);
    let focused = ctx.is_focused(fid) && enabled;
    let hovered = ctx.is_hovered(rect) && enabled;
    let t = theme::get();
    let mark_st = if !enabled {
        theme::muted()
    } else if on {
        mark_style(focused, hovered).fg(if focused { t.purple } else { t.success })
    } else {
        mark_style(focused, hovered)
    };
    let label_st = if !enabled {
        theme::muted()
    } else if focused {
        theme::accent_bold()
    } else if hovered {
        theme::text().add_modifier(Modifier::BOLD)
    } else {
        theme::text()
    };
    buf.set_string(area.x, area.y, mark, mark_st);
    buf.set_string(area.x + 4, area.y, &label, label_st);
    if enabled {
        ctx.focusable(fid, rect, Kind::Button, Some(action));
    }
    w
}

pub struct InputSpec<'a> {
    pub label: &'a str,
    pub placeholder: &'a str,
    pub fid: Fid,
    pub id: InputId,
    pub error: bool,
}

pub fn input(buf: &mut Buffer, ctx: &mut Ctx, area: Rect, input: &mut TextInput, spec: InputSpec) {
    if area.height < 3 || area.width < 6 {
        return;
    }
    let area = Rect::new(area.x, area.y, area.width, 3);
    let focused = ctx.is_focused(spec.fid);
    let t = theme::get();
    let border_style = if spec.error {
        Style::default().fg(t.error)
    } else {
        theme::border(focused || ctx.is_hovered(area))
    };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(theme::base());
    if !spec.label.is_empty() {
        let st = if focused {
            theme::accent_bold()
        } else {
            theme::dim()
        };
        block = block.title(Line::from(Span::styled(format!(" {} ", spec.label), st)));
    }
    let inner = block.inner(area);
    block.render(area, buf);
    let field = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1);
    draw_field(buf, ctx, field, input, &spec, focused);
    ctx.focusable(spec.fid, area, Kind::Input(spec.id), None);
    ctx.click(area, Action::FocusInput(spec.id, 0));
    register_field_click(ctx, field, spec.id);
}

fn register_field_click(ctx: &mut Ctx, field: Rect, id: InputId) {
    for col in 0..field.width {
        ctx.click(
            Rect::new(field.x + col, field.y, 1, 1),
            Action::FocusInput(id, col),
        );
    }
}

fn draw_field(
    buf: &mut Buffer,
    ctx: &mut Ctx,
    field: Rect,
    input: &mut TextInput,
    spec: &InputSpec,
    focused: bool,
) {
    if field.width == 0 {
        return;
    }
    if input.value().is_empty() {
        buf.set_stringn(
            field.x,
            field.y,
            truncate(spec.placeholder, field.width),
            field.width as usize,
            theme::muted(),
        );
        if focused {
            ctx.set_cursor(Position::new(field.x, field.y));
        }
        return;
    }
    if !focused {
        let shown = truncate_left(&input.display_value(), field.width);
        buf.set_stringn(
            field.x,
            field.y,
            &shown,
            field.width as usize,
            theme::text(),
        );
        return;
    }
    let (visible, col) = input.view(field.width);
    buf.set_stringn(
        field.x,
        field.y,
        &visible,
        field.width as usize,
        theme::text(),
    );
    ctx.set_cursor(Position::new(field.x + col, field.y));
}

pub fn badge(text: &str, color: Color) -> Span<'static> {
    let t = theme::get();
    if t.truecolor {
        Span::styled(
            format!(" {} ", text),
            Style::default().fg(color).bg(t.surface_hi),
        )
    } else {
        Span::styled(format!("[{}]", text), Style::default().fg(color))
    }
}

pub fn gauge(buf: &mut Buffer, area: Rect, ratio: f64) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let t = theme::get();
    let ratio = ratio.clamp(0.0, 1.0);
    let total_eighths = (ratio * area.width as f64 * 8.0).round() as u32;
    let full = total_eighths / 8;
    let rem = total_eighths % 8;
    const PARTIAL: [&str; 8] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let empty_style = if t.truecolor {
        Style::default().fg(t.surface_hi).bg(t.bg)
    } else {
        theme::muted()
    };
    for i in 0..area.width {
        let x = area.x + i;
        let pos = if area.width > 1 {
            i as f32 / (area.width - 1) as f32
        } else {
            0.0
        };
        let color = theme::gradient(pos);
        let (sym, style) = if (i as u32) < full {
            ("█", Style::default().fg(color))
        } else if i as u32 == full && rem > 0 {
            (PARTIAL[rem as usize], Style::default().fg(color))
        } else {
            (if t.truecolor { "█" } else { "·" }, empty_style)
        };
        for y in area.top()..area.bottom() {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol(sym).set_style(style);
            }
        }
    }
}

pub fn scrollbar(buf: &mut Buffer, area: Rect, total: usize, visible: usize, offset: usize) {
    if area.height == 0 || total <= visible || area.width == 0 {
        return;
    }
    let h = area.height as usize;
    let thumb = ((visible * h) / total).max(1);
    let max_off = total - visible;
    let pos = (offset.min(max_off) * (h - thumb))
        .checked_div(max_off)
        .unwrap_or(0);
    let x = area.x + area.width - 1;
    for i in 0..h {
        let (sym, style) = if i >= pos && i < pos + thumb {
            ("┃", theme::accent())
        } else {
            ("│", theme::border(false))
        };
        if let Some(c) = buf.cell_mut((x, area.y + i as u16)) {
            c.set_symbol(sym).set_style(style);
        }
    }
}

pub fn list_window(
    selected: usize,
    offset: &mut usize,
    len: usize,
    height: usize,
) -> std::ops::Range<usize> {
    if height == 0 || len == 0 {
        *offset = 0;
        return 0..0;
    }
    if selected < *offset {
        *offset = selected;
    } else if selected >= *offset + height {
        *offset = selected + 1 - height;
    }
    *offset = (*offset).min(len.saturating_sub(height));
    *offset..(*offset + height).min(len)
}

pub fn spinner(tick: u64) -> &'static str {
    const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    FRAMES[(tick as usize) % FRAMES.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Busy,
    Success,
    Warning,
    Error,
}

pub fn status(buf: &mut Buffer, area: Rect, tone: Tone, msg: &str, tick: u64) -> u16 {
    if area.width < 4 || area.height == 0 {
        return 0;
    }
    let (icon, style) = match tone {
        Tone::Info => ("ℹ", theme::accent()),
        Tone::Busy => (spinner(tick), theme::accent()),
        Tone::Success => ("✓", theme::success()),
        Tone::Warning => ("!", theme::warning()),
        Tone::Error => ("✗", theme::error()),
    };
    buf.set_string(area.x, area.y, icon, style.add_modifier(Modifier::BOLD));
    let text_area = Rect::new(area.x + 2, area.y, area.width - 2, area.height);
    let text_style = match tone {
        Tone::Error => theme::error(),
        Tone::Success => theme::success(),
        Tone::Warning => theme::warning(),
        _ => theme::text(),
    };
    paragraph(buf, text_area, msg, text_style)
}

pub fn status_height(msg: &str, width: u16) -> u16 {
    wrap_height(msg, width.saturating_sub(2))
}

pub fn fmt_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= 1024.0 {
        format!("{:.1} KB", b / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_words_and_long_tokens() {
        assert_eq!(wrap("hello world foo", 11), vec!["hello world", "foo"]);
        assert_eq!(wrap("a\nb", 10), vec!["a", "b"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn truncates_with_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate_left("/very/long/path", 6), "…/path");
    }

    #[test]
    fn keeps_selection_in_window() {
        let mut off = 0;
        assert_eq!(list_window(7, &mut off, 20, 5), 3..8);
        assert_eq!(list_window(1, &mut off, 20, 5), 1..6);
        assert_eq!(list_window(19, &mut off, 20, 5), 15..20);
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1536), "1.5 KB");
        assert_eq!(fmt_bytes(5 * 1024 * 1024), "5.00 MB");
    }
}
