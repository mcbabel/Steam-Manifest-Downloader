pub mod file_browser;
pub mod input;
pub mod widgets;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};

use crate::app::action::{Action, InputId, ListId, ScrollTarget};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fid(pub &'static str, pub usize);

impl Fid {
    pub const fn new(name: &'static str) -> Self {
        Fid(name, 0)
    }
    pub const fn idx(name: &'static str, i: usize) -> Self {
        Fid(name, i)
    }
}

#[derive(Debug, Clone)]
pub enum Kind {
    Button,
    Input(InputId),
    List(ListId),
}

#[derive(Debug, Clone)]
pub struct Focusable {
    pub fid: Fid,
    pub rect: Rect,
    pub kind: Kind,
    pub action: Option<Action>,
}

#[derive(Debug, Clone)]
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
    pub fid: Option<Fid>,
    pub double: Option<Action>,
}

#[derive(Default)]
pub struct Ctx {
    pub focusables: Vec<Focusable>,
    pub hits: Vec<Hit>,
    pub scrolls: Vec<(Rect, ScrollTarget)>,
    pub cursor: Option<Position>,
    pub preferred_focus: Option<Fid>,
    pub focus: Option<Fid>,
    pub hover: Option<Position>,
    pub modal: bool,
    pub follow_focus: bool,
}

impl Ctx {
    pub fn new(focus: Option<Fid>, hover: Option<Position>) -> Self {
        Ctx {
            focus,
            hover,
            follow_focus: true,
            ..Default::default()
        }
    }

    pub fn is_focused(&self, fid: Fid) -> bool {
        self.focus == Some(fid)
    }

    pub fn is_hovered(&self, rect: Rect) -> bool {
        self.hover.is_some_and(|p| rect.contains(p))
    }

    pub fn focusable(&mut self, fid: Fid, rect: Rect, kind: Kind, action: Option<Action>) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.focusables.push(Focusable {
            fid,
            rect,
            kind,
            action: action.clone(),
        });
        if let Some(action) = action {
            self.hits.push(Hit {
                rect,
                action,
                fid: Some(fid),
                double: None,
            });
        }
    }

    pub fn click(&mut self, rect: Rect, action: Action) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.hits.push(Hit {
            rect,
            action,
            fid: None,
            double: None,
        });
    }

    pub fn row(&mut self, rect: Rect, list_fid: Fid, select: Action, activate: Action) {
        if rect.width == 0 || rect.height == 0 {
            return;
        }
        self.hits.push(Hit {
            rect,
            action: select,
            fid: Some(list_fid),
            double: Some(activate),
        });
    }

    pub fn scroll_region(&mut self, rect: Rect, target: ScrollTarget) {
        self.scrolls.push((rect, target));
    }

    pub fn set_cursor(&mut self, pos: Position) {
        self.cursor = Some(pos);
    }

    pub fn prefer(&mut self, fid: Fid) {
        if self.preferred_focus.is_none() {
            self.preferred_focus = Some(fid);
        }
    }

    pub fn begin_modal(&mut self) {
        self.focusables.clear();
        self.hits.clear();
        self.scrolls.clear();
        self.cursor = None;
        self.preferred_focus = None;
        self.modal = true;
    }

    pub fn hit_at(&self, pos: Position) -> Option<&Hit> {
        self.hits.iter().rev().find(|h| h.rect.contains(pos))
    }

    pub fn scroll_at(&self, pos: Position) -> Option<ScrollTarget> {
        self.scrolls
            .iter()
            .rev()
            .find(|(r, _)| r.contains(pos))
            .map(|(_, t)| *t)
    }

    pub fn find(&self, fid: Fid) -> Option<&Focusable> {
        self.focusables.iter().find(|f| f.fid == fid)
    }
}

pub struct ScrollView {
    area: Rect,
    pub buf: Buffer,
    focus_start: usize,
    hit_start: usize,
    scroll_start: usize,
    cursor_before: Option<Position>,
    hover_before: Option<Position>,
    y: u16,
}

impl ScrollView {
    pub fn begin(ctx: &mut Ctx, area: Rect, max_height: u16, offset: u16) -> Self {
        let content_w = area.width.saturating_sub(1);
        let virt = Rect::new(area.x, 0, content_w, max_height.max(area.height));
        let mut buf = Buffer::empty(virt);
        buf.set_style(virt, theme::base());
        let hover_before = ctx.hover;
        ctx.hover = hover_before
            .filter(|p| area.contains(*p))
            .map(|p| Position::new(p.x, p.y - area.y + offset));
        ScrollView {
            area,
            buf,
            focus_start: ctx.focusables.len(),
            hit_start: ctx.hits.len(),
            scroll_start: ctx.scrolls.len(),
            cursor_before: ctx.cursor,
            hover_before,
            y: 0,
        }
    }

    pub fn width(&self) -> u16 {
        self.buf.area.width
    }

    pub fn next(&mut self, height: u16) -> Rect {
        let max = self.buf.area.height;
        let h = height.min(max.saturating_sub(self.y));
        let r = Rect::new(self.area.x, self.y, self.buf.area.width, h);
        self.y = self.y.saturating_add(h);
        r
    }

    pub fn gap(&mut self, rows: u16) {
        self.y = (self.y + rows).min(self.buf.area.height);
    }

    pub fn used(&self) -> u16 {
        self.y
    }

    pub fn advance_to(&mut self, y: u16) {
        self.y = self.y.max(y.min(self.buf.area.height));
    }

    pub fn x(&self) -> u16 {
        self.area.x
    }

    pub fn finish(self, out: &mut Buffer, ctx: &mut Ctx, offset: &mut u16, target: ScrollTarget) {
        ctx.hover = self.hover_before;
        let area = self.area;
        let content_h = self.y.max(1);
        let max_off = content_h.saturating_sub(area.height);

        if let Some(focus) = ctx.focus.filter(|_| ctx.follow_focus) {
            if let Some(f) = ctx.focusables[self.focus_start..]
                .iter()
                .find(|f| f.fid == focus)
            {
                let top = f.rect.y;
                let bottom = f.rect.y + f.rect.height;
                if top < *offset {
                    *offset = top.saturating_sub(1);
                } else if bottom > *offset + area.height {
                    *offset = (bottom + 1).saturating_sub(area.height);
                }
            }
        }
        *offset = (*offset).min(max_off);
        let off = *offset;

        for row in 0..area.height {
            let vy = off + row;
            if vy >= self.buf.area.height {
                break;
            }
            for col in 0..self.buf.area.width {
                let x = area.x + col;
                if let (Some(src), Some(dst)) =
                    (self.buf.cell((x, vy)), out.cell_mut((x, area.y + row)))
                {
                    *dst = src.clone();
                }
            }
        }

        let translate = |r: Rect| -> Option<Rect> {
            let top = r.y as i32 - off as i32 + area.y as i32;
            let bottom = top + r.height as i32;
            let clip_top = top.max(area.y as i32);
            let clip_bottom = bottom.min((area.y + area.height) as i32);
            if clip_bottom <= clip_top {
                return None;
            }
            Some(Rect::new(
                r.x,
                clip_top as u16,
                r.width,
                (clip_bottom - clip_top) as u16,
            ))
        };

        let mut kept = Vec::new();
        for mut f in ctx.focusables.drain(self.focus_start..) {
            f.rect = translate(f.rect).unwrap_or(Rect::new(f.rect.x, area.y, 0, 0));
            kept.push(f);
        }
        ctx.focusables.extend(kept);

        let hits: Vec<Hit> = ctx
            .hits
            .drain(self.hit_start..)
            .filter_map(|mut h| {
                h.rect = translate(h.rect)?;
                Some(h)
            })
            .collect();
        ctx.hits.extend(hits);

        let scrolls: Vec<_> = ctx
            .scrolls
            .drain(self.scroll_start..)
            .filter_map(|(r, t)| translate(r).map(|r| (r, t)))
            .collect();
        ctx.scrolls.extend(scrolls);

        if ctx.cursor != self.cursor_before {
            ctx.cursor = ctx.cursor.and_then(|c| {
                let y = c.y as i32 - off as i32 + area.y as i32;
                (y >= area.y as i32 && y < (area.y + area.height) as i32)
                    .then_some(Position::new(c.x, y as u16))
            });
        }

        ctx.scroll_region(area, target);
        if content_h > area.height {
            widgets::scrollbar(
                out,
                area,
                content_h as usize,
                area.height as usize,
                off as usize,
            );
        }
    }
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

pub fn split_h(area: Rect, w: u16, gap: u16) -> (Rect, Rect) {
    let w = w.min(area.width);
    let left = Rect::new(area.x, area.y, w, area.height);
    let rx = (area.x + w + gap).min(area.x + area.width);
    let right = Rect::new(rx, area.y, area.x + area.width - rx, area.height);
    (left, right)
}

pub fn take_top(area: &mut Rect, h: u16) -> Rect {
    let h = h.min(area.height);
    let top = Rect::new(area.x, area.y, area.width, h);
    area.y += h;
    area.height -= h;
    top
}

pub fn take_bottom(area: &mut Rect, h: u16) -> Rect {
    let h = h.min(area.height);
    area.height -= h;
    Rect::new(area.x, area.y + area.height, area.width, h)
}

pub fn inset_h(area: Rect, m: u16) -> Rect {
    let m = m.min(area.width / 2);
    Rect::new(area.x + m, area.y, area.width - 2 * m, area.height)
}
