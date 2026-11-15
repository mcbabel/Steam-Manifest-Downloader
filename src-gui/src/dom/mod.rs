pub mod layout;
pub mod markup;
pub mod paint;
pub mod spec;
pub mod wrap;

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{pos2, Align, Color32, CursorIcon, FontFamily, FontId, Rect, Response, Sense, Stroke, Vec2};
use egui_taffy::{taffy, tid, LeafMeasure, MeasureWidth, Tui, TuiBuilderLogic};

use crate::html;
use crate::icons::{Icon, IconDef};
use crate::style::selector::{Chain, ACTIVE, DISABLED, EMPTY, FIRST_CHILD, FIRST_OF_TYPE, FOCUS, FOCUS_VISIBLE, HOVER, LAST_CHILD, ROOT};
use crate::style::{Computed, Cursor, Display, Env, Family, Len, NodeInfo, Position, PseudoElement, Sheet, TextAlign, WhiteSpace};
use paint::{ease, paint_box, paint_native_control, paint_outline, Visual};

pub use spec::{el, el_dyn, El};

#[derive(Clone, Debug)]
pub struct Record {
    pub path: String,
    pub rect: Rect,
    pub text: Option<String>,
    pub clips: bool,
    pub sticky: bool,
}

pub struct DomCx {
    pub images: crate::images::Images,
    pub sheet: Arc<Sheet>,
    pub env: Env,
    pub time: f64,
    pub frame: u64,
    pub record: Option<RefCell<Vec<Record>>>,
    pub rects: RefCell<std::collections::HashMap<&'static str, Rect>>,
    pub focused: std::cell::Cell<Option<&'static str>>,
    pub focus_first: std::cell::Cell<bool>,
}

#[derive(Clone)]
struct Memory {
    rect: Rect,
    children: usize,
    flow: bool,
    tight: bool,
    baseline: Option<f32>,
    height: f32,
    overflows: bool,
    drop: f32,
    from: Option<Visual>,
    to: Option<Visual>,
    started: f64,
    born: f64,
    seen: u64,
}

impl Memory {
    fn new(cx: &DomCx) -> Self {
        Self { rect: Rect::NOTHING, children: 0, flow: false, tight: false, baseline: None, height: 0.0, overflows: false, drop: 0.0, from: None, to: None, started: 0.0, born: cx.time, seen: cx.frame }
    }
}

struct Frame {
    node: NodeInfo,
    style: Arc<Computed>,
    signature: u64,
    id: egui::Id,
    children: usize,
    previous_children: Option<usize>,
    last_child: Option<NodeInfo>,
    tags_seen: Vec<&'static str>,
    color: Color32,
    path: String,
    inline_elements: usize,
    block_children: usize,
    text_children: usize,
    flow: bool,
    tight: bool,
    pending_space: bool,
    baselines: Baselines,
    prefix: Option<Prefix>,
    inline_box: bool,
    drop: f32,
}

#[derive(Clone, Default)]
struct Baselines(std::rc::Rc<std::cell::Cell<Option<(f32, f32)>>>);

impl Baselines {
    fn note(&self, first: f32, last: f32) {
        let seen = self.0.get();
        self.0.set(Some((seen.map_or(first, |(first, _)| first), last)));
    }
}

struct Prefix {
    text: String,
    format: egui::TextFormat,
    before: f32,
    after: f32,
}

pub struct Dom<'a> {
    pub tui: &'a mut Tui,
    pub cx: &'a DomCx,
    stack: Vec<Frame>,
    nodes: Vec<NodeInfo>,
    previous: Vec<Option<NodeInfo>>,
}

pub struct Out<R> {
    pub response: Option<Response>,
    pub inner: R,
    pub rect: Rect,
}

fn hash_of(parent: u64, node: &NodeInfo, previous: Option<&NodeInfo>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    parent.hash(&mut hasher);
    node.hash(&mut hasher);
    previous.hash(&mut hasher);
    hasher.finish()
}

fn cursor_icon(cursor: Cursor) -> Option<CursorIcon> {
    Some(match cursor {
        Cursor::Pointer => CursorIcon::PointingHand,
        Cursor::Text => CursorIcon::Text,
        Cursor::NotAllowed => CursorIcon::NotAllowed,
        Cursor::Default | Cursor::Auto => return None,
    })
}

const LINK_MARK: f32 = 0.001;

const BOX_MARK: f32 = 0.0004;

fn marks_of(format: &TextFormat) -> (Option<usize>, bool) {
    let value = (format.expand_bg - 1.0) / LINK_MARK;
    let link = (value + 0.2).floor().max(0.0);
    ((link >= 1.0).then(|| link as usize - 1), value - link > 0.2)
}

fn marked_rects(galley: &egui::Galley, origin: egui::Pos2, pick: impl Fn((Option<usize>, bool)) -> Option<usize>) -> Vec<(usize, Rect)> {
    let mut marks: Vec<Option<usize>> = Vec::new();
    for section in &galley.job.sections {
        let mark = pick(marks_of(&section.format));
        let count = galley.job.text[section.byte_range.start.0..section.byte_range.end.0].chars().count();
        marks.extend(std::iter::repeat(mark).take(count));
    }
    let mut out: Vec<(usize, Rect)> = Vec::new();
    let mut index = 0usize;
    for row in &galley.rows {
        let mut current: Option<(usize, Rect)> = None;
        for glyph in &row.glyphs {
            let mark = marks.get(index).copied().flatten();
            index += 1;
            let left = origin.x + row.pos.x + glyph.pos.x;
            let area = Rect::from_min_max(pos2(left, origin.y + row.pos.y), pos2(left + glyph.advance_width, origin.y + row.pos.y + row.height()));
            match (&mut current, mark) {
                (Some((open, rect)), Some(mark)) if *open == mark => *rect = rect.union(area),
                (_, mark) => {
                    out.extend(current.take());
                    current = mark.map(|mark| (mark, area));
                }
            }
        }
        out.extend(current.take());
        if row.ends_with_newline {
            index += 1;
        }
    }
    out
}

fn link_rects(galley: &egui::Galley, origin: egui::Pos2) -> Vec<(usize, Rect)> {
    marked_rects(galley, origin, |(link, _)| link)
}

#[derive(Clone)]
struct InlineBox {
    color: Color32,
    radius: f32,
    half_height: f32,
}

pub fn text_format(style: &Computed, color: Color32) -> TextFormat {
    let mut format = TextFormat {
        font_id: FontId::new(style.font_size, if style.family == Family::Mono { FontFamily::Monospace } else { FontFamily::Proportional }),
        color,
        line_height: Some(style.line_height_px()),
        extra_letter_spacing: style.letter_spacing,
        italics: style.italic,
        ..Default::default()
    };
    if style.underline {
        format.underline = Stroke::new((style.font_size / 14.0).max(1.0), color);
    }
    if style.family == Family::Sans {
        format.coords.push("wght", style.font_weight);
    }
    format
}

fn shape_text(text: &str, style: &Computed) -> String {
    let collapsed = match style.white_space {
        WhiteSpace::Normal | WhiteSpace::Nowrap => text.split_whitespace().collect::<Vec<_>>().join(" "),
        WhiteSpace::PreLine => text.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n"),
        WhiteSpace::Pre | WhiteSpace::PreWrap => text.to_string(),
    };
    if style.uppercase {
        collapsed.to_uppercase()
    } else {
        collapsed
    }
}

const WRAP_SLACK: f32 = 1.0;
struct LineBox {
    above: f32,
    content: f32,
    ascent: f32,
    shift: f32,
    paint_shift: f32,
}

impl LineBox {
    fn of(style: &Computed) -> Self {
        let metrics = style.font_metrics();
        let size = style.font_size;
        let height = style.line_height_px();
        let ascent = (metrics.ascent * size).round();
        let descent = (metrics.descent * size).round();
        let half = ((height - (ascent + descent)) / 2.0).floor();
        let exact = (height - (metrics.ascent + metrics.descent) * size) / 2.0 + metrics.ascent * size;
        let grid = |value: f32| (value * 32.0).round() / 32.0;
        let (face_ascent, face_descent) = (grid(metrics.ascent * size), grid(metrics.descent * size));
        let painted = (face_ascent + height.round() - height + 0.5 * (height - face_ascent - face_descent)).round();
        Self { above: half + ascent, content: ascent + descent, ascent, shift: half + ascent - exact, paint_shift: half + ascent - painted }
    }
}

fn place_rows(galley: Arc<egui::Galley>, first: f32, line_height: f32) -> (Arc<egui::Galley>, f32) {
    let baseline = |row: &egui::epaint::text::PlacedRow| row.row.glyphs.first().map(|glyph| glyph.pos.y);
    let Some(usual) = galley.rows.iter().find_map(baseline) else {
        return (galley, first.round());
    };
    let wanted: Vec<f32> = galley.rows.iter().enumerate().map(|(index, row)| (first + index as f32 * line_height).round() - baseline(row).unwrap_or(usual)).collect();
    let origin = wanted[0] - galley.rows[0].pos.y;
    if galley.rows.iter().zip(&wanted).all(|(row, top)| (row.pos.y + origin - top).abs() < 0.01) {
        return (galley, origin);
    }
    let mut placed = (*galley).clone();
    for (row, top) in placed.rows.iter_mut().zip(&wanted) {
        row.pos.y = top - origin;
    }
    if let Some(last) = placed.rows.last() {
        placed.rect.max.y = placed.rect.max.y.max(last.pos.y + last.row.size.y);
    }
    placed.mesh_bounds = placed.mesh_bounds.expand(placed.rows.len() as f32 + 2.0);
    (Arc::new(placed), origin)
}

type InkKey = (char, [u16; 2], [u16; 2], [usize; 2]);

fn glyph_ink(ctx: &egui::Context, glyph: &egui::epaint::text::Glyph) -> Option<Rect> {
    static KNOWN: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<InkKey, Option<Rect>>>> = std::sync::OnceLock::new();
    let uv = glyph.uv_rect;
    if uv.is_nothing() {
        return None;
    }
    let key = (glyph.chr, uv.min, uv.max, ctx.fonts(|fonts| fonts.font_image_size()));
    let mut known = KNOWN.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *known.entry(key).or_insert_with(|| covered(&ctx.fonts(|fonts| fonts.image()), uv.min, uv.max))
}

fn covered(image: &egui::ColorImage, min: [u16; 2], max: [u16; 2]) -> Option<Rect> {
    let [width, height] = image.size;
    let (left, top) = (usize::from(min[0]), usize::from(min[1]));
    let (right, bottom) = (usize::from(max[0]).min(width), usize::from(max[1]).min(height));
    let cover = |x: usize, y: usize| f32::from(image.pixels[y * width + x].a()) / 255.0;
    let column = |x: usize| (top..bottom).map(|y| cover(x, y)).fold(0.0, f32::max);
    let line = |y: usize| (left..right).map(|x| cover(x, y)).fold(0.0, f32::max);
    let first_column = (left..right).find(|x| column(*x) > 0.05)?;
    let last_column = (left..right).rev().find(|x| column(*x) > 0.05)?;
    let first_line = (top..bottom).find(|y| line(*y) > 0.05)?;
    let last_line = (top..bottom).rev().find(|y| line(*y) > 0.05)?;
    Some(Rect::from_min_max(
        pos2((first_column - left) as f32 + 1.0 - column(first_column), (first_line - top) as f32 + 1.0 - line(first_line)),
        pos2((last_column - left) as f32 + column(last_column), (last_line - top) as f32 + line(last_line)),
    ))
}

fn ink_rect(ctx: &egui::Context, galley: &egui::Galley) -> Option<Rect> {
    let scale = ctx.pixels_per_point();
    let snap = |value: f32| (value * scale).round() / scale;
    let mut ink: Option<Rect> = None;
    for row in &galley.rows {
        for glyph in &row.row.glyphs {
            let Some(texels) = glyph_ink(ctx, glyph) else { continue };
            let corner = glyph.pos + glyph.uv_rect.offset;
            let corner = pos2(snap(corner.x), snap(corner.y)) + row.pos.to_vec2();
            let part = Rect::from_min_max(corner + texels.min.to_vec2() / scale, corner + texels.max.to_vec2() / scale);
            ink = Some(ink.map_or(part, |all| all.union(part)));
        }
    }
    ink
}

fn ink_centered(ctx: &egui::Context, galley: &egui::Galley, at: egui::Pos2, target: egui::Pos2) -> egui::Pos2 {
    let scale = ctx.pixels_per_point();
    let snap = |value: f32| (value * scale).round() / scale;
    let Some(ink) = ink_rect(ctx, galley) else { return at };
    let at = pos2(snap(at.x), snap(at.y));
    let shown = at + ink.center().to_vec2();
    pos2(at.x + snap(target.x - shown.x), at.y + snap(target.y - shown.y))
}

fn paint_surface(ui: &egui::Ui, painter: &egui::Painter, rect: Rect, style: &Computed, visual: &Visual) {
    if let Some(mask) = &style.mask_svg {
        paint::paint_svg(ui, rect, mask, visual.background.gamma_multiply(visual.opacity));
        return;
    }
    paint_box(painter, rect, style, visual);
    if let Some(svg) = &style.background_svg {
        let size = style.background_svg_size.map(|[w, h]| egui::vec2(w, h)).unwrap_or(rect.size());
        paint::paint_svg(ui, Rect::from_center_size(rect.center(), size), svg, Color32::WHITE.gamma_multiply(visual.opacity));
    }
}

fn inline_content_box(rect: Rect, style: &Computed) -> Rect {
    let line = LineBox::of(style);
    let slack = style.line_height_px() - line.content;
    if slack <= 0.0 || rect.height() <= slack {
        return rect;
    }
    let top = rect.top() + line.above - line.ascent;
    Rect::from_min_max(pos2(rect.left(), top), pos2(rect.right(), top + rect.height() - slack))
}

fn prepared(job: &LayoutJob, style: &Computed, max_width: f32) -> LayoutJob {
    let mut job = job.clone();
    let wraps = !matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre);
    job.wrap.max_width = if wraps { max_width } else { f32::INFINITY };
    job.halign = match style.text_align {
        TextAlign::Left => Align::Min,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::Max,
    };
    if style.ellipsis && !wraps {
        job.wrap.max_width = max_width;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
    }
    if let Some(rows) = style.line_clamp {
        job.wrap.max_rows = rows;
    }
    if style.break_anywhere {
        job.wrap.break_anywhere = true;
    }
    job
}

fn longest_word(ctx: &egui::Context, job: &LayoutJob) -> f32 {
    let mut widest = 0.0f32;
    for section in &job.sections {
        let text = &job.text[section.byte_range.start.0..section.byte_range.end.0];
        for word in text.split_whitespace() {
            let mut single = LayoutJob::default();
            single.append(word, 0.0, section.format.clone());
            widest = widest.max(ctx.fonts_mut(|f| f.layout_job(single)).size().x);
        }
    }
    widest
}

fn wraps_by_words(style: &Computed) -> bool {
    !matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre) && style.line_clamp.is_none() && !style.break_anywhere && !style.ellipsis
}

fn aligned(mut job: LayoutJob, style: &Computed) -> LayoutJob {
    job.halign = match style.text_align {
        TextAlign::Left => Align::Min,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::Max,
    };
    job
}

fn measure_text(ctx: &egui::Context, job: &LayoutJob, style: &Computed, constraint: MeasureWidth) -> Vec2 {
    let size = measure_text_exact(ctx, job, style, constraint);
    if matches!(constraint, MeasureWidth::Exact(_)) {
        size
    } else {
        egui::vec2(layout::snap_up(size.x), size.y)
    }
}

fn measure_text_exact(ctx: &egui::Context, job: &LayoutJob, style: &Computed, constraint: MeasureWidth) -> Vec2 {
    if wraps_by_words(style) {
        let line = style.line_height_px();
        return match constraint {
            MeasureWidth::Exact(width) => egui::vec2(width, wrap::wrap(ctx, job, width).lines as f32 * line),
            MeasureWidth::AtMost(width) => {
                let wrapped = wrap::wrap(ctx, job, width);
                egui::vec2(wrapped.width.min(width), wrapped.lines as f32 * line)
            }
            MeasureWidth::MaxContent => {
                let wrapped = wrap::wrap(ctx, job, f32::INFINITY);
                egui::vec2(wrapped.width, wrapped.lines as f32 * line)
            }
            MeasureWidth::MinContent => {
                let narrow = wrap::wrap(ctx, job, 0.0);
                egui::vec2(narrow.min_content, narrow.lines as f32 * line)
            }
        };
    }
    let layout = |max_width: f32| {
        let galley = ctx.fonts_mut(|f| f.layout_job(prepared(job, style, max_width)));
        let size = galley.size();
        if size.y <= 0.0 {
            return size;
        }
        egui::vec2(size.x, galley.rows.len().max(1) as f32 * style.line_height_px())
    };
    let natural = |size: Vec2| size;
    let shrinkable = style.ellipsis || style.break_anywhere;
    match constraint {
        MeasureWidth::Exact(width) => egui::vec2(width, layout(width + WRAP_SLACK).y),
        MeasureWidth::AtMost(width) => {
            let size = natural(layout(width + WRAP_SLACK));
            egui::vec2(size.x.min(width), size.y)
        }
        MeasureWidth::MaxContent => natural(layout(f32::INFINITY)),
        MeasureWidth::MinContent => {
            if shrinkable {
                return egui::vec2(0.0, layout(f32::INFINITY).y);
            }
            if matches!(style.white_space, WhiteSpace::Nowrap | WhiteSpace::Pre) {
                return natural(layout(f32::INFINITY));
            }
            let width = longest_word(ctx, job).ceil();
            egui::vec2(width, layout(width + WRAP_SLACK).y)
        }
    }
}

impl<'a> Dom<'a> {
    pub fn root(tui: &'a mut Tui, cx: &'a DomCx, salt: &str) -> Self {
        let mut dom = Self { tui, cx, stack: Vec::new(), nodes: Vec::new(), previous: Vec::new() };
        let theme = if cx.env.light { "light" } else { "dark" };
        let html = NodeInfo { tag: "html", attrs: vec![("data-theme", theme)], state: ROOT, ..Default::default() };
        let body = NodeInfo { tag: "body", ..Default::default() };
        let base = Arc::new(Computed::root());
        let mut parent = base;
        let mut signature = 0u64;
        let mut path = String::new();
        for node in [html, body] {
            if !path.is_empty() {
                path.push_str(" > ");
            }
            path.push_str(node.tag);
            signature = hash_of(signature, &node, None);
            dom.nodes.push(node.clone());
            dom.previous.push(None);
            let chain = Chain { nodes: &dom.nodes, previous: &dom.previous };
            let style = cx.sheet.compute(&chain, None, &parent, signature, cx.env);
            parent = style.clone();
            dom.stack.push(Frame {
                id: egui::Id::unique(("dom", salt, node.tag)),
                path: path.clone(),
                color: style.color,
                node,
                style,
                signature,
                children: 0,
                previous_children: None,
                last_child: None,
                tags_seen: Vec::new(),
                inline_elements: 0,
                block_children: 0,
                text_children: 0,
                flow: false,
                tight: false,
                pending_space: false,
                baselines: Baselines::default(),
                prefix: None,
                inline_box: false,
                drop: 0.0,
            });
        }
        dom
    }

    pub fn virtual_parent(&mut self, element: impl Into<El>) {
        let element: El = element.into();
        let (frame, _) = self.enter(&element, 0);
        self.stack.push(frame);
    }

    pub fn ctx(&self) -> egui::Context {
        self.tui.egui_ctx().clone()
    }

    pub fn style(&self) -> &Computed {
        &self.stack[self.stack.len() - 1].style
    }

    fn memory(&self, id: egui::Id) -> Option<Memory> {
        let frame = self.cx.frame;
        self.tui.egui_ctx().data(|d| d.get_temp::<Memory>(id)).filter(|m| m.seen + 1 >= frame).map(|mut m| {
            m.seen = frame;
            m
        })
    }

    fn remember(ctx: &egui::Context, id: egui::Id, memory: Memory) {
        ctx.data_mut(|d| d.insert_temp(id, memory));
    }

    fn take_space(&mut self) -> f32 {
        let top = self.stack.len() - 1;
        if !self.stack[top].pending_space {
            return 0.0;
        }
        self.stack[top].pending_space = false;
        if !matches!(self.stack[top].style.display, Display::Block | Display::Inline | Display::InlineBlock) {
            return 0.0;
        }
        let format = text_format(&self.stack[top].style, Color32::WHITE);
        layout::snap(wrap::space_width(self.tui.egui_ctx(), &format))
    }

    fn enter(&mut self, element: &El, extra_state: u16) -> (Frame, Option<Memory>) {
        self.flush_prefix();
        let parent_index = self.stack.len() - 1;
        let (id, index, previous_sibling, first_of_type, parent_previous, parent_signature, parent_path) = {
            let parent = &mut self.stack[parent_index];
            let index = parent.children;
            parent.children += 1;
            let first_of_type = !parent.tags_seen.contains(&element.node.tag);
            if first_of_type {
                parent.tags_seen.push(element.node.tag);
            }
            let id = match element.key {
                Some(key) => parent.id.with(("k", key)),
                None => parent.id.with(index),
            };
            let path = if self.cx.record.is_some() { parent.path.clone() } else { String::new() };
            (id, index, parent.last_child.clone(), first_of_type, parent.previous_children, parent.signature, path)
        };
        let memory = self.memory(id);
        let mut node = element.node.clone();
        node.state |= extra_state;
        if index == 0 {
            node.state |= FIRST_CHILD;
        }
        if first_of_type {
            node.state |= FIRST_OF_TYPE;
        }
        if parent_previous.is_some_and(|count| count == index + 1) {
            node.state |= LAST_CHILD;
        }
        if self.cx.sheet.hover_sensitive(&node) {
            if let Some(memory) = &memory {
                if self.tui.egui_ui().rect_contains_pointer(memory.rect) {
                    node.state |= HOVER;
                }
            }
        }
        self.stack[parent_index].last_child = Some(node.clone());
        let signature = hash_of(parent_signature, &node, previous_sibling.as_ref());
        self.nodes.push(node.clone());
        self.previous.push(previous_sibling);
        let mut signature = signature;
        let style = {
            let chain = Chain { nodes: &self.nodes, previous: &self.previous };
            self.cx.sheet.compute(&chain, None, &self.stack[parent_index].style, signature, self.cx.env)
        };
        let style = if element.inline.is_empty() {
            style
        } else {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            signature.hash(&mut hasher);
            element.inline.hash(&mut hasher);
            signature = hasher.finish();
            let mut own = (*style).clone();
            let cx = crate::style::computed::ApplyCx { parent: &self.stack[parent_index].style, viewport: self.cx.env.viewport };
            for (property, value) in &element.inline {
                own.apply(property, value, &cx);
            }
            Arc::new(own)
        };
        let mut path = parent_path;
        if self.cx.record.is_some() {
            path.push_str(" > ");
            path.push_str(node.tag);
            if let Some(id) = node.id {
                path.push('#');
                path.push_str(id);
            }
            let mut classes = node.classes.clone();
            classes.sort_unstable();
            for class in classes {
                path.push('.');
                path.push_str(class);
            }
        }
        let frame = Frame {
            color: style.color,
            previous_children: memory.as_ref().map(|m| m.children),
            node,
            style,
            signature,
            id,
            children: 0,
            last_child: None,
            tags_seen: Vec::new(),
            path,
            inline_elements: 0,
            block_children: 0,
            text_children: 0,
            flow: false,
            tight: false,
            pending_space: false,
            baselines: Baselines::default(),
            prefix: None,
            inline_box: false,
            drop: 0.0,
        };
        if frame.style.display != Display::None && !matches!(frame.style.position, Position::Absolute | Position::Fixed) {
            let parent = &mut self.stack[parent_index];
            if frame.style.display.is_inline_level() {
                parent.inline_elements += 1;
            } else {
                parent.block_children += 1;
            }
        }
        (frame, memory)
    }

    fn leave(&mut self) {
        self.nodes.pop();
        self.previous.pop();
    }

    fn animate(&self, id: egui::Id, memory: &mut Memory, target: Visual, duration: f32) -> Visual {
        let now = self.cx.time;
        if duration <= 0.0 {
            memory.from = None;
            memory.to = Some(target.clone());
            return target;
        }
        let current = match (&memory.from, &memory.to) {
            (Some(from), Some(to)) => {
                let t = (((now - memory.started) as f32) / duration).clamp(0.0, 1.0);
                from.lerp(to, ease(t))
            }
            (None, Some(to)) => to.clone(),
            _ => target.clone(),
        };
        if memory.to.as_ref() != Some(&target) {
            memory.from = Some(current.clone());
            memory.to = Some(target);
            memory.started = now;
        }
        let running = memory.from.is_some() && ((now - memory.started) as f32) < duration;
        if running {
            self.tui.egui_ctx().request_repaint();
        } else {
            memory.from = None;
        }
        let _ = id;
        current
    }

    fn flush_prefix(&mut self) {
        let top = self.stack.len() - 1;
        if self.stack[top].prefix.take().is_some() {
            self.pseudo_as(PseudoElement::Before, true);
        }
    }

    fn pseudo(&mut self, which: PseudoElement) {
        self.pseudo_as(which, false);
    }

    fn pseudo_as(&mut self, which: PseudoElement, boxed: bool) {
        let frame = &self.stack[self.stack.len() - 1];
        if !self.cx.sheet.may_have(which, &frame.node) {
            return;
        }
        let style = {
            let chain = Chain { nodes: &self.nodes, previous: &self.previous };
            let signature = frame.signature ^ 0x5bd1_e995;
            self.cx.sheet.compute(&chain, Some(which), &frame.style, signature, self.cx.env)
        };
        if style.content.is_none() || style.display == Display::None {
            return;
        }
        let inline_text = which == PseudoElement::Before
            && !boxed
            && style.display == Display::Inline
            && style.position == Position::Static
            && style.width == Len::Auto
            && style.background_color.a() == 0
            && matches!(frame.style.display, Display::Block | Display::Inline | Display::InlineBlock);
        if inline_text {
            let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
            let prefix = Prefix { text: style.content.clone().unwrap_or_default(), format: text_format(&style, style.color), before: px(style.margin[3]), after: px(style.margin[1]) };
            let top = self.stack.len() - 1;
            self.stack[top].prefix = Some(prefix);
            return;
        }
        let id = frame.id.with(("pseudo", which as u8));
        let mut taffy_style = layout::to_taffy(&style, self.cx.env.viewport);
        if style.display.is_inline_level() && style.position == Position::Static {
            taffy_style.display = taffy::Display::Block;
        }
        let mut memory = self.memory(id).unwrap_or_else(|| Memory::new(self.cx));
        let target = Visual::of(&style, memory.rect.size());
        let visual = self.animate(id, &mut memory, target, style.max_transition());
        apply_layout_animation(&mut taffy_style, &visual);
        let content = style.content.clone().unwrap_or_default();
        let ctx = self.tui.egui_ctx().clone();
        let color = visual.color;
        let text_style = style.clone();
        self.tui.id(tid(id)).style(taffy_style).add_with_background_ui(
            |ui, container| {
                let rect = container.full_container();
                memory.rect = rect;
                memory.height = container.exact_container().height();
                if !style.visible || container.first_frame() {
                    Self::remember(&ctx, id, memory.clone());
                    return;
                }
                let mut painter = ui.painter().clone();
                painter.multiply_opacity(visual.opacity);
                if style.blur > 0.0 {
                    let glow = visual.gradient.as_ref().map(|g| g.stops[0].0).unwrap_or(visual.background);
                    let shadow = paint::ShadowPx { inset: false, offset: Vec2::ZERO, blur: style.blur * 2.0, spread: 0.0, color: glow };
                    let glow_visual = Visual { background: Color32::TRANSPARENT, gradient: None, shadows: vec![shadow], ..visual.clone() };
                    paint_box(&painter, rect, &style, &glow_visual);
                } else {
                    paint_surface(ui, &painter, rect.translate(visual.translate), &style, &visual);
                }
                if !content.is_empty() {
                    let mut job = LayoutJob::default();
                    job.append(&content, 0.0, text_format(&text_style, color));
                    let galley = painter.layout_job(job);
                    let pos = rect.center() - galley.size() / 2.0;
                    painter.galley(pos, galley, color);
                }
                Self::remember(&ctx, id, memory.clone());
            },
            |_, _| (),
        );
    }

    fn inline_in_block(&self, style: &Computed) -> Option<TextAlign> {
        let frame = &self.stack[self.stack.len() - 1];
        let parent = &frame.style;
        (!frame.flow && parent.display == Display::Block && style.display.is_inline_level() && style.position == Position::Static)
            .then_some(parent.text_align)
    }

    fn line_strut(&self, child: &Computed, memory: Option<&Memory>) -> (f32, f32) {
        let parent = &self.stack[self.stack.len() - 1].style;
        if child.display == Display::Inline {
            let exact = |style: &Computed| {
                let line = LineBox::of(style);
                line.above - line.shift
            };
            let (above, own) = (exact(parent), exact(child));
            let (line, own_line) = (parent.line_height_px(), child.line_height_px());
            return ((above - own).max(0.0), above.max(own) + (line - above).max(own_line - own));
        }
        let strut = LineBox::of(parent);
        let below = parent.line_height_px() - strut.above;
        let measured = memory.filter(|m| m.rect.is_finite() && m.height > 0.0).map(|m| (m.baseline.unwrap_or(m.height), m.height));
        let (baseline, height) = match measured {
            Some(measured) => measured,
            None if child.display == Display::InlineBlock => {
                let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
                let baseline = child.border(0) + px(child.padding[0]) + LineBox::of(child).above;
                (baseline, baseline)
            }
            None => return (0.0, 0.0),
        };
        ((strut.above - baseline).max(0.0), strut.above.max(baseline) + below.max(height - baseline))
    }

    fn strut_below(&self) -> f32 {
        let parent = &self.stack[self.stack.len() - 1].style;
        let line = LineBox::of(parent);
        parent.line_height_px() - line.above
    }

    fn node<R>(&mut self, element: El, sense: Option<Sense>, extra_state: u16, f: impl FnOnce(&mut Dom<'_>) -> R) -> Option<Out<R>> {
        let mut content = Some(f);
        let mut result = None;
        let out = self.node_dyn(element, sense, extra_state, &mut |dom| {
            if let Some(content) = content.take() {
                result = Some(content(dom));
            }
        })?;
        Some(Out { response: out.response, inner: result?, rect: out.rect })
    }

    fn node_dyn(&mut self, element: El, sense: Option<Sense>, extra_state: u16, f: &mut dyn FnMut(&mut Dom<'_>)) -> Option<Out<()>> {
        let (mut frame, memory) = self.enter(&element, extra_state);
        if frame.style.display == Display::None {
            self.leave();
            return None;
        }
        let align = self.inline_in_block(&frame.style);
        match align {
            Some(align) if frame.style.width == Len::Auto => {
                frame.inline_box = frame.style.display == Display::Inline;
                let (lead, height) = self.line_strut(&frame.style, memory.as_ref());
                let mut line = taffy::Style {
                    display: taffy::Display::Flex,
                    justify_content: match align {
                        TextAlign::Left => taffy::JustifyContent::FLEX_START,
                        TextAlign::Center => taffy::JustifyContent::CENTER,
                        TextAlign::Right => taffy::JustifyContent::FLEX_END,
                    },
                    ..Default::default()
                };
                if height > 0.0 {
                    line.align_items = taffy::AlignItems::FLEX_START;
                    line.padding.top = taffy::prelude::length(lead);
                    line.min_size.height = taffy::prelude::length(height);
                }
                let cx = self.cx;
                let line_id = frame.id.with("line");
                let stack = &mut self.stack;
                let nodes = &mut self.nodes;
                let previous = &mut self.previous;
                self.tui.id(tid(line_id)).style(line).add(|tui| {
                    let mut inner = Dom { tui, cx, stack: std::mem::take(stack), nodes: std::mem::take(nodes), previous: std::mem::take(previous) };
                    let out = inner.finish(frame, memory, element, sense, None, f);
                    *stack = inner.stack;
                    *nodes = inner.nodes;
                    *previous = inner.previous;
                    out
                })
            }
            other => self.finish(frame, memory, element, sense, other, f),
        }
    }

    fn finish(&mut self, mut frame: Frame, memory: Option<Memory>, element: El, sense: Option<Sense>, align: Option<TextAlign>, f: &mut dyn FnMut(&mut Dom<'_>)) -> Option<Out<()>> {
        let style = frame.style.clone();
        let inline_box = frame.inline_box;
        let id = frame.id;
        let mut memory = memory.unwrap_or_else(|| Memory::new(self.cx));
        let born = memory.born;
        let target = Visual::of(&style, memory.rect.size());
        let mut visual = self.animate(id, &mut memory, target, style.max_transition());
        if let Some(animation) = &style.animation {
            let age = (self.cx.time - born) as f32;
            if !animation.infinite && animation.duration > 0.0 && age < animation.duration {
                let t = ease((age / animation.duration).clamp(0.0, 1.0));
                match animation.name.as_str() {
                    "fadeIn" => {
                        visual.opacity *= t;
                        visual.translate.y += 10.0 * (1.0 - t);
                    }
                    "modalIn" => {
                        visual.opacity *= t;
                        visual.translate.y += 10.0 * (1.0 - t);
                        visual.scale *= 0.95 + 0.05 * t;
                    }
                    _ => {}
                }
                self.tui.egui_ctx().request_repaint();
            }
            if animation.infinite && animation.duration > 0.0 && style.visible {
                let t = (age / animation.duration).fract();
                if animation.name.contains("spin") {
                    visual.rotate = t * std::f32::consts::TAU;
                    self.tui.egui_ctx().request_repaint();
                } else if animation.name == "pulse" {
                    let phase = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
                    visual.opacity *= 1.0 - 0.6 * ease(phase);
                    self.tui.egui_ctx().request_repaint();
                }
            }
        }
        frame.color = visual.color;

        let mut taffy_style = layout::to_taffy(&style, self.cx.env.viewport);
        if let Some(align) = align {
            align_in_block(&mut taffy_style, &style, align);
            if frame.node.tag == "textarea" {
                let bottom = if let Len::Px(bottom) = style.margin[2] { bottom } else { 0.0 };
                taffy_style.margin.bottom = taffy::prelude::length(bottom + self.strut_below());
            }
        }
        apply_layout_animation(&mut taffy_style, &visual);
        let lead = self.take_space();
        if lead > 0.0 && style.display.is_inline_level() {
            let own = if let Len::Px(left) = style.margin[3] { left } else { 0.0 };
            taffy_style.margin.left = taffy::prelude::length(own + lead);
        }
        let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
        let top = self.stack.len() - 1;
        if self.stack[top].flow && style.display.is_inline_level() && style.vertical_shift != 0.0 {
            let shift = style.vertical_shift;
            if shift > 0.0 {
                taffy_style.margin.top = taffy::prelude::length(px(style.margin[0]) + 2.0 * shift);
                self.stack[top].drop = self.stack[top].drop.max(shift);
            } else {
                taffy_style.margin.bottom = taffy::prelude::length(px(style.margin[2]) - 2.0 * shift);
            }
        }
        let flows = memory.flow && matches!(style.display, Display::Block | Display::Inline | Display::InlineBlock);
        if flows && memory.drop > 0.0 {
            taffy_style.padding.bottom = taffy::prelude::length(px(style.padding[2]) + memory.drop);
        }
        if flows {
            let format = text_format(&style, style.color);
            let space = wrap::space_width(self.tui.egui_ctx(), &format);
            taffy_style.display = taffy::Display::Flex;
            taffy_style.flex_direction = taffy::FlexDirection::Row;
            let shrink_to_fit = style.display == Display::InlineBlock && style.width == Len::Auto;
            taffy_style.flex_wrap = if shrink_to_fit { taffy::FlexWrap::NoWrap } else { taffy::FlexWrap::Wrap };
            taffy_style.align_items = taffy::AlignItems::CENTER;
            taffy_style.gap.width = taffy::prelude::length(if memory.tight { 0.0 } else { layout::snap(space) });
            taffy_style.justify_content = match style.text_align {
                TextAlign::Left => taffy::JustifyContent::FLEX_START,
                TextAlign::Center => taffy::JustifyContent::CENTER,
                TextAlign::Right => taffy::JustifyContent::FLEX_END,
            };
        }
        frame.flow = flows;
        let transformed = visual.translate != Vec2::ZERO || visual.scale != 1.0;
        let popup = matches!(style.position, Position::Absolute | Position::Fixed) && style.z_index.is_some_and(|z| z >= 20);
        let popup_layer = egui::LayerId::new(egui::Order::Tooltip, id.with("popup"));
        let layer = if popup { popup_layer } else { self.tui.egui_ui().layer_id() };
        let ctx = self.tui.egui_ctx().clone();
        let start = transformed.then(|| ctx.graphics(|g| g.get(layer).map_or(egui::layers::ShapeIdx(0), |l| l.next_idx())));
        let cx = self.cx;
        let path = frame.path.clone();
        let label = element.label.clone();
        let title = element.title.clone();
        let cursor = cursor_icon(style.cursor);
        let outline_style = style.clone();
        let outline_visual = visual.clone();
        let opacity = visual.opacity;
        let mut seen_rect = Rect::NOTHING;
        let mut seen_exact = Rect::NOTHING;
        let checked = frame.node.is(crate::style::selector::CHECKED);
        let scrolls = style.overflow.iter().any(|o| *o == crate::style::Overflow::Auto);
        let clips = style.overflow.iter().any(|o| *o != crate::style::Overflow::Visible);
        let sense = sense.or_else(|| (style.position == Position::Sticky).then(Sense::click));
        let bar = scrolls.then(|| self.scrollbar(&frame));
        if let Some(bar) = bar.as_ref().filter(|_| memory.overflows && style.overflow[1] == crate::style::Overflow::Auto) {
            taffy_style.scrollbar_width = bar.width;
        }
        let mut overflows = memory.overflows;
        let stack = &mut self.stack;
        let nodes = &mut self.nodes;
        let previous = &mut self.previous;
        stack.push(frame);
        let mut builder = self.tui.id(tid(id)).style(taffy_style);
        if popup {
            builder = builder.layer(popup_layer);
        }
        let sticky_bottom = (style.position == Position::Sticky).then(|| if let Len::Px(bottom) = style.inset[2] { Some(bottom) } else { None }).flatten();
        if let Some(bottom) = sticky_bottom {
            builder = builder.stick_bottom(bottom);
        }
        if let Some(bar) = bar {
            builder = builder.mut_egui_style(move |egui_style| bar.apply(egui_style));
        }
        let out = builder.add_with_background_ui(
            |ui, container| {
                let rect = container.full_container();
                seen_rect = rect;
                seen_exact = container.exact_container();
                if scrolls {
                    let layout = container.layout();
                    let room = layout.size.height - layout.border.top - layout.border.bottom;
                    overflows = layout.scrollable_overflow_rect.bottom > room + 0.5;
                }
                if opacity < 1.0 {
                    ui.multiply_opacity(opacity);
                }
                let shown = if inline_box { inline_content_box(rect, &style) } else { rect };
                if style.visible {
                    paint_surface(ui, ui.painter(), shown, &style, &visual);
                    if style.native_control != 0 {
                        paint_native_control(ui.painter(), rect, &style, checked, visual.opacity);
                    }
                }
                if let Some(record) = cx.record.as_ref().filter(|_| style.visible) {
                    record.borrow_mut().push(Record { path, rect: shown, text: None, clips, sticky: style.position == Position::Sticky });
                }
                let response = sense.filter(|_| style.pointer_events && style.visible).map(|sense| {
                    let mut response = ui.interact(rect, id.with("hit"), sense);
                    if let Some(label) = &label {
                        let label = label.clone();
                        response.widget_info(move || egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                    }
                    if let Some(icon) = cursor {
                        response = response.on_hover_cursor(icon);
                    }
                    if let Some(title) = &title {
                        response = response.on_hover_text(title.clone());
                    }
                    response
                });
                if response.is_none() {
                    if let (Some(title), true) = (&title, ui.rect_contains_pointer(rect)) {
                        ui.interact(rect, id.with("tip"), Sense::hover()).on_hover_text(title.clone());
                    }
                }
                response
            },
            |tui, _| {
                let mut inner = Dom { tui, cx, stack: std::mem::take(stack), nodes: std::mem::take(nodes), previous: std::mem::take(previous) };
                inner.pseudo(PseudoElement::Before);
                let result = f(&mut inner);
                inner.flush_prefix();
                inner.pseudo(PseudoElement::After);
                *stack = inner.stack;
                *nodes = inner.nodes;
                *previous = inner.previous;
                result
            },
        );
        let frame = self.stack.pop().expect("frame");
        self.leave();
        if outline_style.outline_width > 0.0 {
            paint_outline(self.tui.egui_ui().painter(), seen_rect, &outline_style, &outline_visual);
        }
        if let Some(start) = start {
            let pivot = seen_rect.center().to_vec2();
            let transform = egui::emath::TSTransform::from_translation(visual.translate + pivot)
                * egui::emath::TSTransform::from_scaling(visual.scale)
                * egui::emath::TSTransform::from_translation(-pivot);
            ctx.graphics_mut(|g| {
                let list = g.entry(layer);
                let end = list.next_idx();
                list.transform_range(start, end, transform);
            });
        }
        if element.scroll && seen_rect.is_finite() {
            self.tui.egui_ui().scroll_to_rect(seen_rect, None);
        }
        memory.rect = seen_rect;
        memory.overflows = overflows;
        memory.children = frame.children;
        if let Some(name) = element.track.or(frame.node.id) {
            self.cx.rects.borrow_mut().insert(name, seen_rect);
        }
        if seen_rect.is_finite() && style.display != Display::None && !matches!(style.position, Position::Absolute | Position::Fixed) {
            let own = frame.baselines.0.get().map(|(first, last)| if matches!(style.display, Display::Flex | Display::InlineFlex | Display::Grid) { first } else { last });
            let line = own.unwrap_or(seen_exact.bottom());
            let top = self.stack.len() - 1;
            self.stack[top].baselines.note(line, line);
            memory.height = seen_exact.height();
            let relative = own.map(|line| line - seen_exact.top());
            let moved = match (relative, memory.baseline) {
                (Some(now), Some(before)) => (now - before).abs() > 0.5,
                (None, None) => false,
                _ => true,
            };
            if moved {
                memory.baseline = relative;
                if style.display.is_inline_level() {
                    ctx.request_discard("inline baseline");
                }
            }
        }
        let flow = frame.inline_elements >= 1 && frame.block_children == 0 && frame.inline_elements + frame.text_children >= 2;
        if flow != memory.flow || frame.tight != memory.tight || (frame.drop - memory.drop).abs() > 0.1 {
            memory.flow = flow;
            memory.tight = frame.tight;
            memory.drop = frame.drop;
            ctx.request_discard("inline flow");
        }
        if frame.pending_space && frame.style.display.is_inline_level() {
            let top = self.stack.len() - 1;
            self.stack[top].pending_space = true;
        }
        Self::remember(&ctx, id, memory);
        Some(Out { response: out.background, inner: out.main, rect: seen_rect })
    }

    fn scrollbar(&mut self, frame: &Frame) -> Scrollbar {
        let mut bar = Scrollbar { width: 8.0, thumb: Color32::from_rgba_unmultiplied(128, 128, 128, 90), thumb_hover: None, track: Color32::TRANSPARENT, radius: 4.0 };
        self.nodes.push(frame.node.clone());
        self.previous.push(None);
        for which in [PseudoElement::Scrollbar, PseudoElement::ScrollbarThumb, PseudoElement::ScrollbarTrack] {
            if !self.cx.sheet.may_have(which, &frame.node) {
                continue;
            }
            let chain = Chain { nodes: &self.nodes, previous: &self.previous };
            let style = self.cx.sheet.compute(&chain, Some(which), &frame.style, frame.signature ^ 0x9747_b28c, self.cx.env);
            match which {
                PseudoElement::Scrollbar => {
                    if let Len::Px(width) = style.width {
                        bar.width = width;
                    }
                }
                PseudoElement::ScrollbarThumb => {
                    bar.thumb = style.background_color;
                    if let Len::Px(radius) = style.radius[0] {
                        bar.radius = radius;
                    }
                }
                _ => bar.track = style.background_color,
            }
        }
        let mut hovered = frame.node.clone();
        hovered.state |= HOVER;
        if self.cx.sheet.may_have(PseudoElement::ScrollbarThumb, &hovered) {
            let last = self.nodes.len() - 1;
            self.nodes[last] = hovered;
            let chain = Chain { nodes: &self.nodes, previous: &self.previous };
            let style = self.cx.sheet.compute(&chain, Some(PseudoElement::ScrollbarThumb), &frame.style, frame.signature ^ 0x1b87_3593, self.cx.env);
            bar.thumb_hover = Some(style.background_color);
        }
        self.nodes.pop();
        self.previous.pop();
        bar
    }

    pub fn el<R>(&mut self, element: impl Into<El>, f: impl FnOnce(&mut Dom<'_>) -> R) -> Option<R> {
        self.node(element.into(), None, 0, f).map(|out| out.inner)
    }

    pub fn empty(&mut self, element: impl Into<El>) {
        self.node(element.into(), None, 0, |_| ());
    }

    pub fn hit<R>(&mut self, element: impl Into<El>, sense: Sense, f: impl FnOnce(&mut Dom<'_>) -> R) -> Option<Out<R>> {
        self.node(element.into(), Some(sense), 0, f)
    }

    pub fn button<R>(&mut self, element: impl Into<El>, f: impl FnOnce(&mut Dom<'_>) -> R) -> Response {
        let mut content = Some(f);
        self.button_dyn(element.into(), &mut |dom| {
            if let Some(content) = content.take() {
                content(dom);
            }
        })
    }

    fn button_dyn(&mut self, element: El, f: &mut dyn FnMut(&mut Dom<'_>)) -> Response {
        let parent = &self.stack[self.stack.len() - 1];
        let id = match element.key {
            Some(key) => parent.id.with(("k", key)),
            None => parent.id.with(parent.children),
        };
        let hit = id.with("hit");
        let ctx = self.tui.egui_ctx().clone();
        let disabled = element.node.is(crate::style::selector::DISABLED);
        let mut state = 0;
        if !disabled {
            if ctx.memory(|m| m.has_focus(hit)) {
                state |= FOCUS;
                if let Some(name) = element.node.id {
                    self.cx.focused.set(Some(name));
                }
                if ctx.input(|i| !i.pointer.any_down()) && ctx.data(|d| d.get_temp::<bool>(egui::Id::unique("dom-keyboard-focus")).unwrap_or(false)) {
                    state |= FOCUS_VISIBLE;
                }
            }
            if ctx.is_being_dragged(hit) || ctx.read_response(hit).is_some_and(|r| r.is_pointer_button_down_on()) {
                state |= ACTIVE;
            }
        }
        let sense = if disabled { Sense::hover() } else { Sense::click() };
        let focusable = !disabled && matches!(element.node.tag, "button" | "a");
        match self.node_dyn(element, Some(sense), state, f) {
            Some(Out { response: Some(response), .. }) => {
                if focusable && self.cx.focus_first.replace(false) {
                    response.request_focus();
                }
                response
            }
            _ => self.tui.egui_ui_mut().interact(Rect::NOTHING, hit, Sense::hover()),
        }
    }

    pub fn text(&mut self, text: impl AsRef<str>) {
        let frame = &self.stack[self.stack.len() - 1];
        let style = frame.style.clone();
        let shaped = shape_text(text.as_ref(), &style);
        if shaped.is_empty() {
            return;
        }
        let color = frame.color;
        let mut job = LayoutJob::default();
        self.take_prefix(&mut job);
        job.append(&shaped, 0.0, text_format(&style, color));
        self.text_job(job, Some(shaped));
    }

    fn take_prefix(&mut self, job: &mut LayoutJob) {
        let top = self.stack.len() - 1;
        let Some(prefix) = self.stack[top].prefix.take() else { return };
        let ctx = self.tui.egui_ctx().clone();
        if prefix.before > 0.0 {
            job.append(wrap::SPACER, 0.0, wrap::spacer(&ctx, prefix.before, &prefix.format));
        }
        job.append(&prefix.text, 0.0, prefix.format.clone());
        if prefix.after > 0.0 {
            job.append(wrap::SPACER, 0.0, wrap::spacer(&ctx, prefix.after, &prefix.format));
        }
    }

    pub fn html(&mut self, markup: impl AsRef<str>) -> Option<String> {
        let frame = &self.stack[self.stack.len() - 1];
        let base = frame.style.clone();
        let base_color = frame.color;
        let signature = frame.signature;
        let mut job = LayoutJob::default();
        self.take_prefix(&mut job);
        let ctx = self.tui.egui_ctx().clone();
        let mut inline_box: Option<InlineBox> = None;
        let mut plain = String::new();
        let mut links: Vec<String> = Vec::new();
        let mut previous_link: Option<String> = None;
        for span in html::parse(markup.as_ref()) {
            let mut style = base.clone();
            let mut color = base_color;
            let tags: Vec<&'static str> =
                [(span.link.is_some(), "a"), (span.bold, "strong"), (span.code, "code"), (span.italic, "em")].iter().filter(|(on, _)| *on).map(|(_, tag)| *tag).collect();
            let mut parent = base.clone();
            let mut sig = signature;
            let mut pushed = 0;
            for tag in tags {
                let mut node = NodeInfo { tag, ..Default::default() };
                if tag == "a" {
                    if let Some(class) = &span.link_class {
                        node.classes = class.split_whitespace().map(spec::intern).collect();
                    }
                }
                sig = hash_of(sig, &node, None);
                self.nodes.push(node);
                self.previous.push(None);
                pushed += 1;
                let chain = Chain { nodes: &self.nodes, previous: &self.previous };
                style = self.cx.sheet.compute(&chain, None, &parent, sig, self.cx.env);
                color = style.color;
                parent = style.clone();
            }
            for _ in 0..pushed {
                self.nodes.pop();
                self.previous.pop();
            }
            let text = span.text.split('\n').map(|line| shape_text_keep_edges(line, &style)).collect::<Vec<_>>().join("\n");
            let mut format = text_format(&style, color);
            format.line_height = Some(base.line_height_px().max(style.line_height_px().min(base.line_height_px())));
            let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
            let boxed = span.code && style.background_color.a() > 0;
            if boxed {
                let metrics = style.font_metrics();
                let radius = if let Len::Px(radius) = style.radius[0] { radius } else { 0.0 };
                inline_box = Some(InlineBox { color: style.background_color, radius, half_height: (metrics.ascent + metrics.descent) * style.font_size / 2.0 + px(style.padding[0]) });
                format.expand_bg = 1.0 + BOX_MARK;
            } else if pushed > 0 && style.background_color.a() > 0 {
                format.background = style.background_color;
            }
            match &span.link {
                Some(target) => {
                    if previous_link.as_ref() != Some(target) {
                        links.push(target.clone());
                    }
                    format.expand_bg += LINK_MARK * links.len() as f32;
                }
                None => {}
            }
            previous_link = span.link.clone();
            plain.push_str(&text);
            let (left, right) = if boxed { (px(style.padding[3]), px(style.padding[1])) } else { (0.0, 0.0) };
            if left > 0.0 {
                job.append(wrap::SPACER, 0.0, wrap::spacer(&ctx, left, &format));
            }
            job.append(&text, 0.0, format.clone());
            if right > 0.0 {
                job.append(wrap::SPACER, 0.0, wrap::spacer(&ctx, right, &format));
            }
        }
        if plain.trim().is_empty() {
            return None;
        }
        self.text_job_boxes(job, Some(plain), links, inline_box)
    }

    fn text_job(&mut self, job: LayoutJob, plain: Option<String>) {
        self.text_job_links(job, plain, Vec::new());
    }

    fn text_job_links(&mut self, job: LayoutJob, plain: Option<String>, links: Vec<String>) -> Option<String> {
        self.text_job_boxes(job, plain, links, None)
    }

    fn text_job_boxes(&mut self, job: LayoutJob, plain: Option<String>, links: Vec<String>, inline_box: Option<InlineBox>) -> Option<String> {
        let mut clicked: Option<String> = None;
        let link_id = self.stack[self.stack.len() - 1].id.with(("link", self.stack[self.stack.len() - 1].children, self.stack[self.stack.len() - 1].text_children));
        let top = self.stack.len() - 1;
        self.stack[top].text_children += 1;
        let frame = &self.stack[top];
        let style = frame.style.clone();
        let clip_gradient = style.background_clip_text.then(|| style.background_image.clone()).flatten();
        let path = if self.cx.record.is_some() { format!("{} > #text", frame.path) } else { String::new() };
        let ctx = self.tui.egui_ctx().clone();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        job.hash(&mut hasher);
        (style.white_space as u8, style.ellipsis, style.break_anywhere, style.line_clamp, style.text_align as u8).hash(&mut hasher);
        let key = hasher.finish();
        let measure_job = job.clone();
        let measure_style = style.clone();
        let measure_ctx = ctx.clone();
        let leaf = LeafMeasure { key, measure: Arc::new(move |constraint| measure_text(&measure_ctx, &measure_job, &measure_style, constraint)) };
        let cx = self.cx;
        let selectable = style.selectable;
        let baselines = self.stack[top].baselines.clone();
        let mut leaf_style = taffy::Style::default();
        let lead = self.take_space();
        if lead > 0.0 {
            leaf_style.margin.left = taffy::prelude::length(lead);
        }
        self.tui.style(leaf_style).ui_measured(leaf, |ui, container| {
            let rect = container.full_container_without_border_and_padding();
            let width = container.unrounded_content_width();
            let galley = if wraps_by_words(&style) {
                ui.painter().layout_job(aligned(wrap::wrap(ui.ctx(), &job, width).job, &style))
            } else {
                ui.painter().layout_job(prepared(&job, &style, width + WRAP_SLACK))
            };
            if !style.visible || container.first_frame() {
                return;
            }
            let line = LineBox::of(&style);
            let first = container.exact_content_top() + line.above;
            baselines.note(first, first + (galley.rows.len().max(1) - 1) as f32 * style.line_height_px());
            let model_top = rect.top() + line.above - line.ascent;
            let (galley, origin) = place_rows(galley, first, style.line_height_px());
            let rect = Rect::from_min_size(pos2(rect.left(), origin), rect.size());
            if let Some(record) = &cx.record {
                let width = galley.size().x;
                let left = match style.text_align {
                    TextAlign::Left => rect.left(),
                    TextAlign::Center => rect.center().x - width / 2.0,
                    TextAlign::Right => rect.right() - width,
                };
                let top = model_top;
                let height = (galley.rows.len().max(1) - 1) as f32 * style.line_height_px() + line.content;
                record.borrow_mut().push(Record { path, rect: Rect::from_min_size(pos2(left, top), egui::vec2(width, height)), text: plain, clips: false, sticky: false });
            }
            if selectable {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(match style.text_align {
                    TextAlign::Left => Align::Min,
                    TextAlign::Center => Align::Center,
                    TextAlign::Right => Align::Max,
                })));
                child.add(egui::Label::new(galley).selectable(true));
                return;
            }
            let x = match style.text_align {
                TextAlign::Left => rect.left(),
                TextAlign::Center => rect.center().x,
                TextAlign::Right => rect.right(),
            };
            let at = if style.centers_ink { ink_centered(ui.ctx(), &galley, pos2(x, rect.top()), container.parent_rect().center()) } else { pos2(x, rect.top()) };
            if !links.is_empty() {
                for (index, area) in link_rects(&galley, at) {
                    let response = ui.interact(area, link_id.with((index, area.min.y as i32)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                    if response.clicked() {
                        clicked = links.get(index).cloned();
                    }
                }
            }
            if let Some(inline_box) = &inline_box {
                for (_, area) in marked_rects(&galley, at, |(_, boxed)| boxed.then_some(0)) {
                    let middle = area.center().y;
                    let shape = Rect::from_min_max(pos2(area.left(), middle - inline_box.half_height), pos2(area.right(), middle + inline_box.half_height));
                    ui.painter().rect_filled(shape, inline_box.radius, inline_box.color);
                }
            }
            if let Some(gradient) = &clip_gradient {
                let stops = gradient.positions();
                let pair = crate::paint::Gradient { from: stops[0].1, to: stops[stops.len() - 1].1 };
                crate::paint::gradient_text(ui.painter(), at, &galley, pair);
            } else {
                ui.painter().galley(at, galley, Color32::PLACEHOLDER);
            }
        });
        clicked
    }

    pub fn icon(&mut self, element: impl Into<El>, def: IconDef) {
        self.icon_with(element, def, 2.0, true);
    }

    pub fn icon_with(&mut self, element: impl Into<El>, def: IconDef, stroke: f32, round: bool) {
        let mut element: El = element.into();
        if element.node.tag == "div" {
            element.node.tag = "svg";
        }
        let (frame, memory) = self.enter(&element, 0);
        self.leave();
        let style = frame.style.clone();
        if style.display == Display::None {
            return;
        }
        let baselines = self.stack[self.stack.len() - 1].baselines.clone();
        let mut taffy_style = layout::to_taffy(&style, self.cx.env.viewport);
        taffy_style.display = taffy::Display::Block;
        let lead = self.take_space();
        if lead > 0.0 {
            let own = if let Len::Px(left) = style.margin[3] { left } else { 0.0 };
            taffy_style.margin.left = taffy::prelude::length(own + lead);
        }
        if style.width == Len::Auto && style.height == Len::Auto {
            let [width, height] = element.size.unwrap_or(def.view);
            taffy_style.size = taffy::Size { width: taffy::prelude::length(width), height: taffy::prelude::length(height) };
        }
        if let Some(align) = self.inline_in_block(&style) {
            align_in_block(&mut taffy_style, &style, align);
            if let Len::Px(bottom) = style.margin[2] {
                taffy_style.margin.bottom = taffy::prelude::length(bottom + self.strut_below());
            }
        }
        let parent = &self.stack[self.stack.len() - 1];
        if parent.flow && style.vertical_shift != 0.0 {
            let height = if let Len::Px(height) = style.height { height } else { element.size.unwrap_or(def.view)[1] };
            let line = parent.style.line_height_px();
            let wanted = LineBox::of(&parent.style).above - height + style.vertical_shift;
            let shift = wanted - (line - height) / 2.0;
            let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
            if shift > 0.0 {
                taffy_style.margin.top = taffy::prelude::length(px(style.margin[0]) + 2.0 * shift);
            } else {
                taffy_style.margin.bottom = taffy::prelude::length(px(style.margin[2]) - 2.0 * shift);
            }
        }
        taffy_style.flex_shrink = if style.flex_shrink == 1.0 && style.width != Len::Auto { 0.0 } else { style.flex_shrink };
        let id = frame.id;
        let mut memory = memory.unwrap_or_else(|| Memory::new(self.cx));
        let target = Visual::of(&style, memory.rect.size());
        let visual = self.animate(id, &mut memory, target, style.max_transition());
        let cx = self.cx;
        let ctx = self.tui.egui_ctx().clone();
        let path = frame.path.clone();
        let time = self.cx.time;
        self.tui.id(tid(id)).style(taffy_style).add_with_background_ui(
            |ui, container| {
                let rect = container.full_container();
                memory.rect = rect;
                let exact = container.exact_container();
                memory.height = exact.height();
                baselines.note(exact.bottom(), exact.bottom());
                if !style.visible {
                    Self::remember(&ctx, id, memory.clone());
                    return;
                }
                if let Some(record) = &cx.record {
                    record.borrow_mut().push(Record { path, rect, text: None, clips: false, sticky: false });
                }
                let tint = if matches!(def.name, "app_small" | "flag_gb" | "flag_de") { Color32::WHITE } else { visual.color };
                let mut icon = Icon::new(def, tint.gamma_multiply(visual.opacity)).stroke(stroke);
                if !round {
                    icon = icon.butt();
                }
                let spinning = style.animation.as_ref().filter(|a| a.infinite && a.name.contains("spin"));
                match spinning {
                    Some(animation) => {
                        let angle = ((time / animation.duration.max(0.01) as f64) % 1.0) as f32 * std::f32::consts::TAU;
                        icon.paint_rotated(ui, rect.translate(visual.translate), angle);
                        ui.ctx().request_repaint();
                    }
                    None => icon.paint(ui, rect.translate(visual.translate)),
                }
                Self::remember(&ctx, id, memory.clone());
            },
            |_, _| (),
        );
    }

    pub fn input(&mut self, element: impl Into<El>, value: &mut String, placeholder: &str) -> Response {
        self.input_with(element, value, placeholder, false)
    }

    pub fn input_with(&mut self, element: impl Into<El>, value: &mut String, placeholder: &str, password: bool) -> Response {
        self.edit(element.into(), value, placeholder, password, None)
    }

    pub fn textarea(&mut self, element: impl Into<El>, value: &mut String, placeholder: &str, rows: usize) -> Response {
        self.edit(element.into(), value, placeholder, false, Some(rows.max(1)))
    }

    fn edit(&mut self, mut element: El, value: &mut String, placeholder: &str, password: bool, rows: Option<usize>) -> Response {
        if element.node.tag == "div" {
            element.node.tag = "input";
        }
        let readonly = element.node.attrs.iter().any(|(name, _)| *name == "readonly");
        let parent = &self.stack[self.stack.len() - 1];
        let id = match element.key {
            Some(key) => parent.id.with(("k", key)),
            None => parent.id.with(parent.children),
        };
        let edit_id = id.with("edit");
        let ctx = self.tui.egui_ctx().clone();
        let disabled = element.node.is(crate::style::selector::DISABLED);
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let state = if focused { FOCUS | FOCUS_VISIBLE } else { 0 };
        let placeholder = placeholder.to_string();
        let mut response: Option<Response> = None;
        let out = self.node(element, None, state, |dom| {
            let frame = &dom.stack[dom.stack.len() - 1];
            let style = frame.style.clone();
            let color = frame.color;
            let hint_color = {
                let chain = Chain { nodes: &dom.nodes, previous: &dom.previous };
                let hint = dom.cx.sheet.compute(&chain, Some(PseudoElement::Placeholder), &style, frame.signature ^ 0x2545_f491, dom.cx.env);
                if dom.cx.sheet.may_have(PseudoElement::Placeholder, &frame.node) && hint.color != style.color {
                    hint.color
                } else {
                    color.gamma_multiply(0.55)
                }
            };
            let line = LineBox::of(&style);
            let row_height = style.line_height_px();
            let grip_id = edit_id.with("grip");
            let resizable = rows.is_some() && style.resize_vertical && !disabled;
            let extra = if resizable { ctx.data(|data| data.get_temp::<f32>(grip_id)).unwrap_or(0.0) } else { 0.0 };
            let height = row_height * rows.unwrap_or(1) as f32 + extra;
            let measure = LeafMeasure {
                key: height.to_bits() as u64,
                measure: Arc::new(move |constraint| match constraint {
                    MeasureWidth::Exact(width) => egui::vec2(width, height),
                    MeasureWidth::AtMost(width) => egui::vec2(width.min(180.0), height),
                    MeasureWidth::MaxContent => egui::vec2(180.0, height),
                    MeasureWidth::MinContent => egui::vec2(0.0, height),
                }),
            };
            let format = text_format(&style, color);
            let hint_format = text_format(&style, hint_color);
            let result = dom.tui.style(taffy::Style { flex_grow: 1.0, flex_shrink: 1.0, ..Default::default() }).ui_measured(measure, |ui, container| {
                let rect = container.full_container_without_border_and_padding().translate(egui::vec2(0.0, line.paint_shift));
                if !style.visible {
                    return ui.interact(Rect::NOTHING, edit_id.with("hidden"), Sense::hover());
                }
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(Align::Min)));
                child.set_clip_rect(rect.expand2(egui::vec2(1.0, 4.0)).intersect(ui.clip_rect()));
                let multiline = rows.is_some();
                let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
                    let mut job = LayoutJob::default();
                    let shown: String = if password { "\u{2022}".repeat(text.as_str().chars().count()) } else { text.as_str().to_string() };
                    job.append(&shown, 0.0, format.clone());
                    job.wrap.max_width = if multiline { wrap } else { f32::INFINITY };
                    ui.fonts_mut(|f| f.layout_job(job))
                };
                let mut hint = LayoutJob::default();
                hint.append(&placeholder, 0.0, hint_format.clone());
                let base = if multiline { egui::TextEdit::multiline(value).desired_rows(rows.unwrap_or(1)) } else { egui::TextEdit::singleline(value) };
                let edit = base
                    .id(edit_id)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(rect.width())
                    .min_size(egui::vec2(rect.width(), height))
                    .hint_text(hint)
                    .interactive(!disabled && !readonly)
                    .layouter(&mut layouter);
                let response = child.add(edit);
                if resizable {
                    let px = |len: Len| if let Len::Px(value) = len { value } else { 0.0 };
                    let corner = container.full_container_without_border_and_padding().right_bottom() + egui::vec2(px(style.padding[1]), px(style.padding[2]));
                    let grip = Rect::from_min_max(corner - egui::vec2(15.0, 15.0), corner);
                    let drag = ui.interact(grip, grip_id, Sense::drag()).on_hover_and_drag_cursor(CursorIcon::ResizeVertical);
                    if drag.dragged() {
                        let grown = (extra + drag.drag_delta().y).max(0.0);
                        ui.ctx().data_mut(|data| data.insert_temp(grip_id, grown));
                        ui.ctx().request_repaint();
                    }
                    let stroke = egui::Stroke::new(1.0, hint_color);
                    for inset in [4.0, 9.0] {
                        ui.painter().line_segment([pos2(grip.left() + inset, grip.bottom() - 2.0), pos2(grip.right() - 2.0, grip.top() + inset)], stroke);
                    }
                }
                response
            });
            response = Some(result);
        });
        let _ = out;
        match response {
            Some(response) => {
                if !disabled && !readonly && self.cx.focus_first.replace(false) {
                    response.request_focus();
                }
                response
            }
            None => self.tui.egui_ui_mut().interact(Rect::NOTHING, edit_id, Sense::hover()),
        }
    }

    pub fn markup(&mut self, source: impl AsRef<str>) -> Option<String> {
        let nodes = markup::parsed(source.as_ref());
        let mut clicked = None;
        self.markup_nodes(&nodes, &mut clicked);
        clicked
    }

    fn markup_nodes(&mut self, nodes: &[markup::Node], clicked: &mut Option<String>) {
        let mut index = 0;
        let top = self.stack.len() - 1;
        self.stack[top].tight = true;
        let inline_container = self.stack[top].style.display.is_inline_level();
        let mut first = self.stack[top].children == 0 && self.stack[top].text_children == 0;
        while index < nodes.len() {
            let mut lone_element = false;
            if markup::is_phrasing(&nodes[index]) {
                let end = (index..nodes.len()).find(|at| !markup::is_phrasing(&nodes[*at])).unwrap_or(nodes.len());
                let has_text = nodes[index..end].iter().any(|node| matches!(node, markup::Node::Text(_)) && !markup::is_blank(node));
                let has_element = nodes[index..end].iter().any(|node| matches!(node, markup::Node::El(e) if e.tag != "br"));
                lone_element = !has_text && has_element;
                if lone_element && !matches!(&nodes[index], markup::Node::El(e) if e.tag != "br") {
                    if !first || inline_container {
                        self.stack[top].pending_space = true;
                    }
                    index += 1;
                    continue;
                }
            }
            if markup::is_phrasing(&nodes[index]) && !lone_element {
                let start = index;
                while index < nodes.len() && markup::is_phrasing(&nodes[index]) {
                    index += 1;
                }
                let run = &nodes[start..index];
                if run.iter().all(markup::is_blank) {
                    if !first || inline_container {
                        self.stack[top].pending_space = true;
                    }
                    continue;
                }
                let mut html = String::new();
                markup::serialize(run, &mut html);
                if html.starts_with(char::is_whitespace) && (!first || inline_container) {
                    self.stack[top].pending_space = true;
                }
                let trailing = html.ends_with(char::is_whitespace);
                if let Some(link) = self.html(html.trim()) {
                    *clicked = Some(link);
                }
                if trailing {
                    self.stack[top].pending_space = true;
                }
                first = false;
                continue;
            }
            first = false;
            let markup::Node::El(e) = &nodes[index] else { unreachable!("text is phrasing") };
            index += 1;
            let mut element = el_dyn(&e.tag, e.attr("id"), e.attr("class").unwrap_or(""));
            for (name, value) in &e.attrs {
                match name.as_str() {
                    "class" | "id" | "data-click" | "data-context" | "data-icon" | "data-butt" | "data-stroke" | "data-hide-on-error" | "name" | "href" | "src" | "alt" | "aria-hidden" | "aria-label" => {}
                    "title" => element = element.title(value.clone()),
                    "data-key" => element = element.key(value),
                    "disabled" => element = element.state(DISABLED, true),
                    "style" => {
                        for declaration in value.split(';') {
                            if let Some((property, value)) = declaration.split_once(':') {
                                element = element.css_dyn(property.trim(), value.trim());
                            }
                        }
                    }
                    "width" | "height" => {}
                    _ => element = element.attr_dyn(name, value),
                }
            }
            if let (Some(width), Some(height)) = (e.attr("width").and_then(|v| v.parse::<f32>().ok()), e.attr("height").and_then(|v| v.parse::<f32>().ok())) {
                element = element.size(width, height);
            }
            element = element.state(EMPTY, e.children.is_empty());
            match e.tag.as_str() {
                "svg" | "icon" => {
                    if let Some(def) = e.attr("data-icon").or_else(|| e.attr("name")).and_then(crate::icons::by_name) {
                        let stroke = e.attr("data-stroke").and_then(|v| v.parse::<f32>().ok()).unwrap_or(2.0);
                        self.icon_with(element, def, stroke, e.attr("data-butt").is_none());
                    }
                }
                "img" => {
                    let hide = e.attr("data-hide-on-error").is_some();
                    self.image(element, e.attr("src").unwrap_or(""), hide);
                }
                _ => {
                    let action = e.attr("data-click").map(str::to_string).or_else(|| (e.tag == "a").then(|| e.attr("href").map(|href| format!("open:{href}"))).flatten());
                    match action {
                        Some(action) => {
                            let response = self.button(element, |dom| dom.markup_nodes(&e.children, clicked));
                            if response.clicked() {
                                *clicked = Some(action);
                            } else if response.secondary_clicked() {
                                if let Some(context) = e.attr("data-context") {
                                    *clicked = Some(context.to_string());
                                }
                            }
                        }
                        None => {
                            self.el(element, |dom| dom.markup_nodes(&e.children, clicked));
                        }
                    }
                }
            }
        }
    }

    pub fn image(&mut self, element: impl Into<El>, uri: &str, hide_on_error: bool) {
        let mut element: El = element.into();
        if element.node.tag == "div" {
            element.node.tag = "img";
        }
        let status = if uri.is_empty() { crate::images::Status::Failed } else { self.cx.images.status(uri) };
        if hide_on_error && status == crate::images::Status::Failed {
            return;
        }
        let (frame, memory) = self.enter(&element, 0);
        self.leave();
        let style = frame.style.clone();
        if style.display == Display::None {
            return;
        }
        let mut taffy_style = layout::to_taffy(&style, self.cx.env.viewport);
        let lead = self.take_space();
        if lead > 0.0 {
            let own = if let Len::Px(left) = style.margin[3] { left } else { 0.0 };
            taffy_style.margin.left = taffy::prelude::length(own + lead);
        }
        let natural = match status {
            crate::images::Status::Ready(size) => Some(size),
            _ => element.size,
        };
        if let Some([width, height]) = natural {
            match (style.width == Len::Auto, style.height == Len::Auto) {
                (true, true) => {
                    let [w, h] = element.size.unwrap_or([width, height]);
                    taffy_style.size = taffy::Size { width: taffy::prelude::length(w), height: taffy::prelude::length(h) };
                }
                (true, false) | (false, true) if height > 0.0 => taffy_style.aspect_ratio = Some(width / height),
                _ => {}
            }
        }
        if let Some(align) = self.inline_in_block(&style) {
            align_in_block(&mut taffy_style, &style, align);
        }
        let id = frame.id;
        let mut memory = memory.unwrap_or_else(|| Memory::new(self.cx));
        let target = Visual::of(&style, memory.rect.size());
        let visual = self.animate(id, &mut memory, target, style.max_transition());
        let cx = self.cx;
        let ctx = self.tui.egui_ctx().clone();
        let path = frame.path.clone();
        let uri = uri.to_string();
        self.tui.id(tid(id)).style(taffy_style).add_with_background_ui(
            |ui, container| {
                let rect = container.full_container();
                memory.rect = rect;
                memory.height = container.exact_container().height();
                if let Some(record) = &cx.record {
                    record.borrow_mut().push(Record { path, rect, text: None, clips: false, sticky: false });
                }
                if !style.visible {
                    Self::remember(&ctx, id, memory.clone());
                    return;
                }
                paint_box(ui.painter(), rect, &style, &visual);
                if let crate::images::Status::Ready([width, height]) = status {
                    let mut uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    if style.object_fit_cover && width > 0.0 && height > 0.0 && rect.width() > 0.0 && rect.height() > 0.0 {
                        let image_ratio = width / height;
                        let box_ratio = rect.width() / rect.height();
                        if image_ratio > box_ratio {
                            let visible = box_ratio / image_ratio;
                            uv = Rect::from_min_max(pos2((1.0 - visible) / 2.0, 0.0), pos2((1.0 + visible) / 2.0, 1.0));
                        } else {
                            let visible = image_ratio / box_ratio;
                            uv = Rect::from_min_max(pos2(0.0, (1.0 - visible) / 2.0), pos2(1.0, (1.0 + visible) / 2.0));
                        }
                    }
                    let radius = paint::radius_of(&style, rect);
                    egui::Image::new(uri.as_str()).uv(uv).corner_radius(radius).tint(Color32::WHITE.gamma_multiply(visual.opacity)).paint_at(ui, rect);
                }
                Self::remember(&ctx, id, memory.clone());
            },
            |_, _| (),
        );
    }

    pub fn leaf(&mut self, element: impl Into<El>, paint: impl FnOnce(&mut egui::Ui, Rect, &Computed)) {
        let element: El = element.into();
        let (frame, memory) = self.enter(&element, 0);
        self.leave();
        let style = frame.style.clone();
        if style.display == Display::None {
            return;
        }
        let taffy_style = layout::to_taffy(&style, self.cx.env.viewport);
        let id = frame.id;
        let mut memory = memory.unwrap_or_else(|| Memory::new(self.cx));
        let target = Visual::of(&style, memory.rect.size());
        let visual = self.animate(id, &mut memory, target, style.max_transition());
        let cx = self.cx;
        let ctx = self.tui.egui_ctx().clone();
        let path = frame.path.clone();
        self.tui.id(tid(id)).style(taffy_style).add_with_background_ui(
            |ui, container| {
                let rect = container.full_container();
                memory.rect = rect;
                memory.height = container.exact_container().height();
                if let Some(record) = &cx.record {
                    record.borrow_mut().push(Record { path, rect, text: None, clips: false, sticky: false });
                }
                paint_box(ui.painter(), rect, &style, &visual);
                paint(ui, rect, &style);
                Self::remember(&ctx, id, memory.clone());
            },
            |_, _| (),
        );
    }
}

#[derive(Clone, Copy)]
struct Scrollbar {
    width: f32,
    thumb: Color32,
    thumb_hover: Option<Color32>,
    track: Color32,
    radius: f32,
}

impl Scrollbar {
    fn apply(&self, style: &mut egui::Style) {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = self.width;
        scroll.bar_inner_margin = 0.0;
        scroll.bar_outer_margin = 0.0;
        scroll.handle_min_length = 24.0;
        scroll.fade.strength = 0.0;
        scroll.kinetic.rubber_band = false;
        style.spacing.scroll = scroll;
        let hover = self.thumb_hover.unwrap_or(self.thumb);
        let radius = egui::CornerRadius::from(self.radius);
        style.visuals.extreme_bg_color = self.track;
        for (widget, color) in [
            (&mut style.visuals.widgets.inactive, self.thumb),
            (&mut style.visuals.widgets.hovered, hover),
            (&mut style.visuals.widgets.active, hover),
        ] {
            widget.bg_fill = color;
            widget.weak_bg_fill = color;
            widget.fg_stroke.color = color;
            widget.corner_radius = radius;
        }
    }
}

fn apply_layout_animation(taffy_style: &mut taffy::Style, visual: &Visual) {
    if let Some(left) = visual.left {
        taffy_style.inset.left = taffy::prelude::length(left);
    }
    if let Some([px, percent]) = visual.width {
        taffy_style.size.width = if percent != 0.0 { taffy::prelude::percent(percent) } else { taffy::prelude::length(px) };
    }
}

fn align_in_block(taffy_style: &mut taffy::Style, style: &Computed, align: TextAlign) {
    let free = |len: Len| matches!(len, Len::Px(v) if v == 0.0);
    let auto = taffy::LengthPercentageAuto::auto;
    match align {
        TextAlign::Left => {}
        TextAlign::Center => {
            if free(style.margin[3]) {
                taffy_style.margin.left = auto();
            }
            if free(style.margin[1]) {
                taffy_style.margin.right = auto();
            }
        }
        TextAlign::Right => {
            if free(style.margin[3]) {
                taffy_style.margin.left = auto();
            }
        }
    }
}

fn shape_text_keep_edges(text: &str, style: &Computed) -> String {
    let shaped = shape_text(text, style);
    if matches!(style.white_space, WhiteSpace::Pre | WhiteSpace::PreWrap | WhiteSpace::PreLine) {
        return shaped;
    }
    let leading = text.starts_with(char::is_whitespace);
    let trailing = text.ends_with(char::is_whitespace);
    let mut out = String::with_capacity(shaped.len() + 2);
    if leading {
        out.push(' ');
    }
    out.push_str(&shaped);
    if trailing && !shaped.is_empty() {
        out.push(' ');
    }
    out
}

pub fn layer<R>(ctx: &egui::Context, cx: &DomCx, name: &str, order: egui::Order, modal: bool, f: impl FnOnce(&mut Dom<'_>) -> R) -> R {
    let window = ctx.content_rect();
    let area = egui::Area::new(egui::Id::unique(("dom-layer", name))).order(order).fixed_pos(window.min).interactable(true);
    let out = area.show(ctx, |ui| {
        ui.set_clip_rect(window);
        let mut root = ui.new_child(egui::UiBuilder::new().max_rect(window).layout(egui::Layout::top_down(Align::Min)));
        let root_style = taffy::Style {
            display: taffy::Display::Block,
            size: taffy::Size { width: taffy::prelude::length(window.width()), height: taffy::prelude::length(window.height()) },
            ..Default::default()
        };
        egui_taffy::tui(&mut root, egui::Id::unique(("dom-layer-tui", name))).reserve_space(window.size()).style(root_style).show(|tui| {
            let mut dom = Dom::root(tui, cx, name);
            f(&mut dom)
        })
    });
    if modal {
        ctx.memory_mut(|m| m.set_modal_layer(out.response.layer_id));
    }
    out.inner
}
