use egui::Color32;

use super::parse::split_top;
use super::value::{
    parse_color, parse_gradient, parse_len, parse_px, parse_seconds, parse_shadows, parse_transform, Col, Gradient, Len, Shadow,
    Transform, Units, ROOT_FONT,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Display {
    None,
    Block,
    Flex,
    Grid,
    Inline,
    InlineBlock,
    InlineFlex,
}

impl Display {
    pub fn is_inline_level(self) -> bool {
        matches!(self, Display::Inline | Display::InlineBlock | Display::InlineFlex)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    Visible,
    Hidden,
    Auto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Auto,
    Start,
    End,
    Center,
    Stretch,
    Baseline,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Solid,
    Dashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WhiteSpace {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cursor {
    Auto,
    Default,
    Pointer,
    Text,
    NotAllowed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineHeight {
    Normal,
    Factor(f32),
    Px(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Sans,
    Mono,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    pub name: String,
    pub duration: f32,
    pub infinite: bool,
    pub linear: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Computed {
    pub display: Display,
    pub position: Position,
    pub inset: [Len; 4],
    pub width: Len,
    pub height: Len,
    pub min_width: Len,
    pub min_height: Len,
    pub max_width: Len,
    pub max_height: Len,
    pub margin: [Len; 4],
    pub padding: [Len; 4],
    pub border_width: [f32; 4],
    pub border_style: [BorderStyle; 4],
    pub border_color: [Col; 4],
    pub radius: [Len; 4],
    pub flex_row: bool,
    pub flex_wrap: bool,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Len,
    pub align_items: Align,
    pub align_self: Align,
    pub justify_content: Align,
    pub justify_items: Align,
    pub gap: [f32; 2],
    pub order: i32,
    pub vertical_shift: f32,
    pub grid_columns: Option<String>,
    pub grid_rows: Option<String>,
    pub grid_areas: Option<String>,
    pub grid_area: Option<String>,
    pub grid_column: Option<String>,
    pub overflow: [Overflow; 2],
    pub background_color: Color32,
    pub background_image: Option<Gradient>,
    pub background_svg: Option<std::sync::Arc<str>>,
    pub background_svg_size: Option<[f32; 2]>,
    pub mask_svg: Option<std::sync::Arc<str>>,
    pub resize_vertical: bool,
    pub centers_ink: bool,
    pub background_clip_text: bool,
    pub box_shadow: Vec<Shadow>,
    pub opacity: f32,
    pub transform: Transform,
    pub brightness: f32,
    pub grayscale: bool,
    pub blur: f32,
    pub outline_width: f32,
    pub outline_color: Col,
    pub outline_offset: f32,
    pub cursor: Cursor,
    pub visible: bool,
    pub pointer_events: bool,
    pub z_index: Option<i32>,
    pub object_fit_cover: bool,
    pub native_control: u8,
    pub accent: Option<Color32>,
    pub content: Option<String>,
    pub transition: Vec<(String, f32)>,
    pub animation: Option<Animation>,
    pub line_clamp: Option<usize>,
    pub color: Color32,
    pub font_size: f32,
    pub keyword_size: bool,
    pub font_weight: f32,
    pub family: Family,
    pub italic: bool,
    pub line_height: LineHeight,
    pub letter_spacing: f32,
    pub text_align: TextAlign,
    pub uppercase: bool,
    pub white_space: WhiteSpace,
    pub ellipsis: bool,
    pub underline: bool,
    pub break_anywhere: bool,
    pub selectable: bool,
}

impl Computed {
    pub fn root() -> Self {
        Self {
            display: Display::Block,
            position: Position::Static,
            inset: [Len::Auto; 4],
            width: Len::Auto,
            height: Len::Auto,
            min_width: Len::Auto,
            min_height: Len::Auto,
            max_width: Len::Auto,
            max_height: Len::Auto,
            margin: [Len::Px(0.0); 4],
            padding: [Len::Px(0.0); 4],
            border_width: [3.0; 4],
            border_style: [BorderStyle::None; 4],
            border_color: [Col::Current; 4],
            radius: [Len::Px(0.0); 4],
            flex_row: true,
            flex_wrap: false,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Len::Auto,
            align_items: Align::Auto,
            align_self: Align::Auto,
            justify_content: Align::Auto,
            justify_items: Align::Auto,
            gap: [0.0; 2],
            order: 0,
            vertical_shift: 0.0,
            grid_columns: None,
            grid_rows: None,
            grid_areas: None,
            grid_area: None,
            grid_column: None,
            overflow: [Overflow::Visible; 2],
            background_color: Color32::TRANSPARENT,
            background_image: None,
            background_svg: None,
            background_svg_size: None,
            mask_svg: None,
            resize_vertical: false,
            centers_ink: false,
            background_clip_text: false,
            box_shadow: Vec::new(),
            opacity: 1.0,
            transform: Transform::default(),
            brightness: 1.0,
            grayscale: false,
            blur: 0.0,
            outline_width: 0.0,
            outline_color: Col::Current,
            outline_offset: 0.0,
            cursor: Cursor::Auto,
            visible: true,
            pointer_events: true,
            z_index: None,
            object_fit_cover: false,
            native_control: 0,
            accent: None,
            content: None,
            transition: Vec::new(),
            animation: None,
            line_clamp: None,
            color: Color32::BLACK,
            font_size: ROOT_FONT,
            keyword_size: true,
            font_weight: 400.0,
            family: Family::Sans,
            italic: false,
            line_height: LineHeight::Normal,
            letter_spacing: 0.0,
            text_align: TextAlign::Left,
            uppercase: false,
            white_space: WhiteSpace::Normal,
            ellipsis: false,
            underline: false,
            break_anywhere: false,
            selectable: true,
        }
    }

    pub fn child_of(parent: &Computed) -> Self {
        Self {
            color: parent.color,
            font_size: parent.font_size,
            keyword_size: parent.keyword_size,
            font_weight: parent.font_weight,
            family: parent.family,
            italic: parent.italic,
            line_height: parent.line_height,
            letter_spacing: parent.letter_spacing,
            text_align: parent.text_align,
            uppercase: parent.uppercase,
            white_space: parent.white_space,
            break_anywhere: parent.break_anywhere,
            cursor: parent.cursor,
            visible: parent.visible,
            pointer_events: parent.pointer_events,
            selectable: parent.selectable,
            accent: parent.accent,
            ..Self::root()
        }
    }

    pub fn border(&self, side: usize) -> f32 {
        if self.border_style[side] == BorderStyle::None {
            0.0
        } else {
            self.border_width[side]
        }
    }

    pub fn line_height_px(&self) -> f32 {
        match self.line_height {
            LineHeight::Normal => {
                let metrics = self.font_metrics();
                (self.font_size * metrics.ascent).round() + (self.font_size * metrics.descent).round()
            }
            LineHeight::Factor(f) => {
                let size = (self.font_size as f64 * 64.0).round() / 64.0;
                let factor = (f as f64 * 1e5).round() / 1e5;
                ((size * factor * 64.0 + 1e-6).floor() / 64.0) as f32
            }
            LineHeight::Px(px) => (px * 64.0).floor() / 64.0,
        }
    }

    pub fn font_metrics(&self) -> crate::fonts::Metrics {
        match self.family {
            Family::Sans => crate::fonts::INTER,
            Family::Mono => crate::fonts::monospace_metrics(),
        }
    }

    pub fn transition_for(&self, property: &str) -> f32 {
        self.transition
            .iter()
            .filter(|(name, _)| name == "all" || name == property || (property == "background" && name == "background-color"))
            .map(|(_, secs)| *secs)
            .fold(0.0, f32::max)
    }

    pub fn max_transition(&self) -> f32 {
        self.transition.iter().map(|(_, secs)| *secs).fold(0.0, f32::max)
    }
}

pub fn apply_tag_defaults(tag: &str, style: &mut Computed) {
    match tag {
        "span" | "a" | "strong" | "b" | "em" | "i" | "code" | "label" | "svg" | "img" | "small" | "kbd" => style.display = Display::Inline,
        "button" | "input" | "textarea" | "select" => style.display = Display::InlineBlock,
        _ => {}
    }
    match tag {
        "h1" => {
            style.font_size *= 2.0;
            style.keyword_size = false;
            style.font_weight = 700.0;
        }
        "h2" => {
            style.font_size *= 1.5;
            style.keyword_size = false;
            style.font_weight = 700.0;
        }
        "h3" => {
            style.font_size *= 1.17;
            style.keyword_size = false;
            style.font_weight = 700.0;
        }
        "h4" | "th" | "strong" | "b" => style.font_weight = 700.0,
        "em" | "i" => style.italic = true,
        "small" => {
            style.font_size *= 0.8333;
            style.keyword_size = false;
        }
        "code" | "kbd" | "pre" => {
            if style.family != Family::Mono && style.keyword_size {
                style.font_size = 13.0;
            }
            style.family = Family::Mono;
            if tag == "pre" {
                style.white_space = WhiteSpace::Pre;
            }
        }
        "a" => {
            style.color = Color32::from_rgb(0, 0, 238);
            style.underline = true;
            style.cursor = Cursor::Pointer;
        }
        "button" => {
            style.font_size = 13.3333;
            style.font_weight = 400.0;
            style.line_height = LineHeight::Normal;
            style.letter_spacing = 0.0;
            style.uppercase = false;
            style.text_align = TextAlign::Center;
            style.color = Color32::BLACK;
            style.cursor = Cursor::Default;
            style.white_space = WhiteSpace::Normal;
            style.border_width = [2.0; 4];
            style.border_style = [BorderStyle::Solid; 4];
            style.background_color = Color32::from_rgb(239, 239, 239);
        }
        "input" | "textarea" | "select" => {
            style.font_size = 13.3333;
            style.font_weight = 400.0;
            style.line_height = LineHeight::Normal;
            style.letter_spacing = 0.0;
            style.uppercase = false;
            style.text_align = TextAlign::Left;
            style.color = Color32::BLACK;
            style.cursor = Cursor::Text;
            style.white_space = WhiteSpace::Normal;
            style.border_width = [2.0; 4];
            style.border_style = [BorderStyle::Solid; 4];
            style.background_color = Color32::WHITE;
        }
        _ => {}
    }
}

pub fn apply_input_type_defaults(kind: &str, style: &mut Computed) {
    let control = match kind {
        "checkbox" => 1,
        "radio" => 2,
        _ => return,
    };
    style.native_control = control;
    style.border_width = [0.0; 4];
    style.border_style = [BorderStyle::None; 4];
    style.background_color = Color32::TRANSPARENT;
    style.width = Len::Px(13.0);
    style.height = Len::Px(13.0);
    style.margin = [Len::Px(3.0), Len::Px(3.0), Len::Px(3.0), Len::Px(4.0)];
    style.cursor = Cursor::Default;
}

pub struct ApplyCx<'a> {
    pub parent: &'a Computed,
    pub viewport: [f32; 2],
}

fn sides<T: Copy>(values: &[T]) -> Option<[T; 4]> {
    Some(match values.len() {
        1 => [values[0]; 4],
        2 => [values[0], values[1], values[0], values[1]],
        3 => [values[0], values[1], values[2], values[1]],
        4 => [values[0], values[1], values[2], values[3]],
        _ => return None,
    })
}

fn align(value: &str) -> Option<Align> {
    Some(match value {
        "normal" | "auto" => Align::Auto,
        "flex-start" | "start" | "left" => Align::Start,
        "flex-end" | "end" | "right" => Align::End,
        "center" => Align::Center,
        "stretch" => Align::Stretch,
        "baseline" => Align::Baseline,
        "space-between" => Align::SpaceBetween,
        "space-around" => Align::SpaceAround,
        "space-evenly" => Align::SpaceEvenly,
        _ => return None,
    })
}

fn border_style(value: &str) -> Option<BorderStyle> {
    Some(match value {
        "none" | "hidden" => BorderStyle::None,
        "solid" => BorderStyle::Solid,
        "dashed" | "dotted" => BorderStyle::Dashed,
        _ => return None,
    })
}

fn border_shorthand(value: &str, units: Units) -> Option<(f32, BorderStyle, Col)> {
    let mut width = 3.0;
    let mut style = BorderStyle::None;
    let mut color = Col::Current;
    for token in split_top(value, ' ') {
        if let Some(s) = border_style(&token) {
            style = s;
        } else if let Some(px) = parse_px(&token, units) {
            width = px;
        } else {
            color = parse_color(&token)?;
        }
    }
    Some((width, style, color))
}

fn overflow(value: &str) -> Option<Overflow> {
    Some(match value {
        "visible" => Overflow::Visible,
        "hidden" | "clip" => Overflow::Hidden,
        "auto" | "scroll" => Overflow::Auto,
        _ => return None,
    })
}

fn svg_of(value: &str) -> Option<String> {
    let start = value.find("data:image/svg+xml")?;
    let rest = &value[start..];
    let data = &rest[rest.find(',')? + 1..];
    let end = match value[..start].chars().last().filter(|c| matches!(c, '"' | '\'')) {
        Some(quote) => data.find(&format!("{quote})"))?,
        None => data.find(')')?,
    };
    let raw = data[..end].as_bytes();
    let mut out = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (raw[at], raw.get(at + 1).copied().and_then(hex), raw.get(at + 2).copied().and_then(hex)) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                at += 3;
            }
            (byte, _, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok().filter(|svg| svg.contains("<svg"))
}

const IGNORED: &[&str] = &[
    "box-sizing",
    "-webkit-font-smoothing",
    "-moz-osx-font-smoothing",
    "app-region",
    "-webkit-app-region",
    "-webkit-user-select",
    "-moz-appearance",
    "-webkit-box-orient",
    "backdrop-filter",
    "-webkit-backdrop-filter",
    "border-collapse",
    "clip",
    "stroke-width",
    "list-style",
    "background-position",
    "background-repeat",
    "background-origin",
    "-webkit-text-fill-color",
    "text-rendering",
    "scroll-behavior",
    "will-change",
];

impl Computed {
    pub fn apply(&mut self, prop: &str, value: &str, cx: &ApplyCx<'_>) -> bool {
        let units = Units { em: self.font_size, viewport: cx.viewport };
        let len = |v: &str| parse_len(v, units);
        let tokens = || split_top(value, ' ');
        if IGNORED.contains(&prop) || prop.starts_with("--") {
            return true;
        }
        match prop {
            "display" => {
                self.display = match value {
                    "none" => Display::None,
                    "block" | "-webkit-box" | "list-item" | "table" => Display::Block,
                    "flex" => Display::Flex,
                    "grid" => Display::Grid,
                    "inline" => Display::Inline,
                    "inline-block" => Display::InlineBlock,
                    "inline-flex" => Display::InlineFlex,
                    _ => return false,
                }
            }
            "position" => {
                self.position = match value {
                    "static" => Position::Static,
                    "relative" => Position::Relative,
                    "absolute" => Position::Absolute,
                    "fixed" => Position::Fixed,
                    "sticky" => Position::Sticky,
                    _ => return false,
                }
            }
            "top" | "right" | "bottom" | "left" => {
                let Some(v) = len(value) else { return false };
                let index = ["top", "right", "bottom", "left"].iter().position(|s| *s == prop).unwrap();
                self.inset[index] = v;
            }
            "inset" => {
                let values: Option<Vec<Len>> = tokens().iter().map(|t| len(t)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                self.inset = all;
            }
            "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height" | "flex-basis" => {
                let Some(v) = len(value) else { return false };
                match prop {
                    "width" => self.width = v,
                    "height" => self.height = v,
                    "min-width" => self.min_width = v,
                    "min-height" => self.min_height = v,
                    "max-width" => self.max_width = v,
                    "max-height" => self.max_height = v,
                    _ => self.flex_basis = v,
                }
            }
            "margin" | "padding" => {
                let values: Option<Vec<Len>> = tokens().iter().map(|t| len(t)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                if prop == "margin" {
                    self.margin = all;
                } else {
                    self.padding = all;
                }
            }
            "margin-top" | "margin-right" | "margin-bottom" | "margin-left" | "padding-top" | "padding-right" | "padding-bottom"
            | "padding-left" => {
                let Some(v) = len(value) else { return false };
                let side = prop.rsplit('-').next().unwrap();
                let index = ["top", "right", "bottom", "left"].iter().position(|s| *s == side).unwrap();
                if prop.starts_with("margin") {
                    self.margin[index] = v;
                } else {
                    self.padding[index] = v;
                }
            }
            "padding-inline" | "margin-inline" => {
                let values: Vec<Len> = tokens().iter().filter_map(|t| len(t)).collect();
                let (Some(start), end) = (values.first().copied(), values.get(1).copied()) else { return false };
                let target = if prop == "padding-inline" { &mut self.padding } else { &mut self.margin };
                target[3] = start;
                target[1] = end.unwrap_or(start);
            }
            "border" => {
                let Some((width, style, color)) = border_shorthand(value, units) else { return false };
                self.border_width = [width; 4];
                self.border_style = [style; 4];
                self.border_color = [color; 4];
            }
            "border-top" | "border-right" | "border-bottom" | "border-left" => {
                let Some((width, style, color)) = border_shorthand(value, units) else { return false };
                let index = ["border-top", "border-right", "border-bottom", "border-left"].iter().position(|s| *s == prop).unwrap();
                self.border_width[index] = width;
                self.border_style[index] = style;
                self.border_color[index] = color;
            }
            "border-color" => {
                let values: Option<Vec<Col>> = tokens().iter().map(|t| parse_color(t)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                self.border_color = all;
            }
            "border-width" => {
                let values: Option<Vec<f32>> = tokens().iter().map(|t| parse_px(t, units)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                self.border_width = all;
            }
            "border-style" => {
                let values: Option<Vec<BorderStyle>> = tokens().iter().map(|t| border_style(t)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                self.border_style = all;
            }
            "border-top-color" | "border-right-color" | "border-bottom-color" | "border-left-color" => {
                let Some(color) = parse_color(value) else { return false };
                let index = ["border-top-color", "border-right-color", "border-bottom-color", "border-left-color"]
                    .iter()
                    .position(|s| *s == prop)
                    .unwrap();
                self.border_color[index] = color;
            }
            "border-radius" => {
                let values: Option<Vec<Len>> = tokens().iter().map(|t| len(t)).collect();
                let Some(all) = values.and_then(|v| sides(&v)) else { return false };
                self.radius = all;
            }
            "resize" => self.resize_vertical = matches!(value, "vertical" | "both"),
            "-smd-center-text" => self.centers_ink = value == "ink",
            "mask" | "-webkit-mask" => {
                self.mask_svg = svg_of(value).map(|svg| svg.replace("currentColor", "#fff").into());
            }
            "background-size" => {
                let first = split_top(value, ',').into_iter().next().unwrap_or_default();
                let sizes: Vec<f32> = split_top(&first, ' ').iter().filter_map(|t| len(t)).filter_map(|l| l.px()).collect();
                self.background_svg_size = match sizes.as_slice() {
                    [w, h] => Some([*w, *h]),
                    [w] => Some([*w, *w]),
                    _ => None,
                };
            }
            "background" | "background-color" | "background-image" => {
                if prop != "background-color" {
                    self.background_svg = split_top(value, ',').iter().find_map(|layer| svg_of(layer)).map(Into::into);
                    if let Some(gradient) = split_top(value, ',').iter().rev().find_map(|layer| parse_gradient(layer)) {
                        self.background_image = Some(gradient);
                        if prop == "background" {
                            self.background_color = Color32::TRANSPARENT;
                        }
                        return true;
                    }
                }
                if value == "none" {
                    self.background_image = None;
                    if prop == "background" {
                        self.background_color = Color32::TRANSPARENT;
                    }
                    return true;
                }
                if prop == "background-image" {
                    return false;
                }
                let layout_words = ["center", "cover", "contain", "no-repeat", "repeat", "/", "top", "left", "right", "bottom"];
                let words = tokens();
                let rest_is_layout = words.iter().skip(1).all(|w| layout_words.contains(&w.as_str()) || len(w).is_some());
                let Some(color) = words.first().and_then(|w| parse_color(w)).filter(|_| prop == "background-color" && words.len() == 1 || prop == "background" && rest_is_layout) else {
                    return false;
                };
                self.background_color = color.resolve(self.color);
                if prop == "background" {
                    self.background_image = None;
                }
            }
            "background-clip" | "-webkit-background-clip" => self.background_clip_text = value == "text",
            "box-shadow" => {
                let Some(shadows) = parse_shadows(value, units) else { return false };
                self.box_shadow = shadows;
            }
            "opacity" => {
                let Ok(v) = value.parse::<f32>() else { return false };
                self.opacity = v.clamp(0.0, 1.0);
            }
            "transform" => {
                let Some(transform) = parse_transform(value, units) else { return false };
                self.transform = transform;
            }
            "filter" => {
                for part in tokens() {
                    if let Some(args) = part.strip_prefix("brightness(") {
                        self.brightness = args.trim_end_matches(')').parse().unwrap_or(1.0);
                    } else if part.starts_with("grayscale(") {
                        self.grayscale = true;
                    } else if let Some(args) = part.strip_prefix("blur(") {
                        self.blur = parse_px(args.trim_end_matches(')'), units).unwrap_or(0.0);
                    } else if part == "none" {
                        self.brightness = 1.0;
                        self.grayscale = false;
                        self.blur = 0.0;
                    } else {
                        return false;
                    }
                }
            }
            "outline" => {
                if value == "none" || value == "0" {
                    self.outline_width = 0.0;
                } else {
                    let Some((width, style, color)) = border_shorthand(value, units) else { return false };
                    self.outline_width = if style == BorderStyle::None { 0.0 } else { width };
                    self.outline_color = color;
                }
            }
            "outline-offset" => {
                let Some(px) = parse_px(value, units) else { return false };
                self.outline_offset = px;
            }
            "flex-direction" => self.flex_row = value.starts_with("row"),
            "flex-wrap" => self.flex_wrap = value.starts_with("wrap"),
            "flex-grow" => self.flex_grow = value.parse().unwrap_or(0.0),
            "flex-shrink" => self.flex_shrink = value.parse().unwrap_or(1.0),
            "flex" => {
                let parts = tokens();
                match parts.as_slice() {
                    [one] if one == "none" => {
                        self.flex_grow = 0.0;
                        self.flex_shrink = 0.0;
                        self.flex_basis = Len::Auto;
                    }
                    [one] if one == "auto" => {
                        self.flex_grow = 1.0;
                        self.flex_shrink = 1.0;
                        self.flex_basis = Len::Auto;
                    }
                    [grow] => {
                        let Ok(grow) = grow.parse::<f32>() else { return false };
                        self.flex_grow = grow;
                        self.flex_shrink = 1.0;
                        self.flex_basis = Len::Px(0.0);
                    }
                    [grow, shrink, basis] => {
                        let (Ok(grow), Ok(shrink), Some(basis)) = (grow.parse::<f32>(), shrink.parse::<f32>(), len(basis)) else {
                            return false;
                        };
                        self.flex_grow = grow;
                        self.flex_shrink = shrink;
                        self.flex_basis = basis;
                    }
                    _ => return false,
                }
            }
            "align-items" | "align-self" | "justify-content" | "justify-items" => {
                let Some(v) = align(value) else { return false };
                match prop {
                    "align-items" => self.align_items = v,
                    "align-self" => self.align_self = v,
                    "justify-content" => self.justify_content = v,
                    _ => self.justify_items = v,
                }
            }
            "place-items" => {
                let Some(v) = align(value) else { return false };
                self.align_items = v;
                self.justify_items = v;
            }
            "gap" => {
                let values: Vec<f32> = tokens().iter().filter_map(|t| parse_px(t, units)).collect();
                match values.as_slice() {
                    [both] => self.gap = [*both, *both],
                    [row, column] => self.gap = [*row, *column],
                    _ => return false,
                }
            }
            "row-gap" | "column-gap" => {
                let Some(px) = parse_px(value, units) else { return false };
                self.gap[if prop == "row-gap" { 0 } else { 1 }] = px;
            }
            "order" => self.order = value.parse().unwrap_or(0),
            "vertical-align" => self.vertical_shift = value.strip_suffix("px").and_then(|px| px.parse::<f32>().ok()).map(|up| -up).unwrap_or(0.0),
            "grid-template-columns" => self.grid_columns = Some(value.to_string()),
            "grid-template-rows" => self.grid_rows = Some(value.to_string()),
            "grid-template-areas" => self.grid_areas = Some(value.to_string()),
            "grid-area" => self.grid_area = Some(value.to_string()),
            "grid-column" => self.grid_column = Some(value.to_string()),
            "overflow" => {
                let Some(v) = overflow(value) else { return false };
                self.overflow = [v, v];
            }
            "overflow-x" | "overflow-y" => {
                let Some(v) = overflow(value) else { return false };
                self.overflow[if prop == "overflow-x" { 0 } else { 1 }] = v;
            }
            "cursor" => {
                self.cursor = match value {
                    "pointer" => Cursor::Pointer,
                    "text" => Cursor::Text,
                    "not-allowed" => Cursor::NotAllowed,
                    "default" => Cursor::Default,
                    _ => Cursor::Auto,
                }
            }
            "visibility" => self.visible = value != "hidden",
            "pointer-events" => self.pointer_events = value != "none",
            "z-index" => self.z_index = value.parse().ok(),
            "object-fit" => self.object_fit_cover = value == "cover",
            "appearance" | "-webkit-appearance" => {
                if value == "none" {
                    self.native_control = 0;
                }
            }
            "accent-color" => self.accent = parse_color(value).map(|color| color.resolve(cx.parent.color)),
            "content" => self.content = Some(value.trim_matches(|c| c == '"' || c == '\'').to_string()),
            "transition" => {
                self.transition.clear();
                if value != "none" {
                    for part in split_top(value, ',') {
                        let words = split_top(&part, ' ');
                        let duration = words.iter().find_map(|w| parse_seconds(w)).unwrap_or(0.0);
                        let name = words.first().filter(|w| parse_seconds(w).is_none()).cloned().unwrap_or_else(|| "all".into());
                        self.transition.push((name, duration));
                    }
                }
            }
            "animation" => {
                if value == "none" {
                    self.animation = None;
                } else {
                    let words = tokens();
                    let duration = words.iter().find_map(|w| parse_seconds(w)).unwrap_or(0.0);
                    let Some(name) = words.first().cloned() else { return false };
                    self.animation =
                        Some(Animation { name, duration, infinite: words.iter().any(|w| w == "infinite"), linear: words.iter().any(|w| w == "linear") });
                }
            }
            "-webkit-line-clamp" => self.line_clamp = value.parse().ok(),
            "color" => {
                let Some(color) = parse_color(value) else { return false };
                self.color = color.resolve(cx.parent.color);
            }
            "font" => {
                if value != "inherit" {
                    return false;
                }
                self.font_size = cx.parent.font_size;
                self.font_weight = cx.parent.font_weight;
                self.family = cx.parent.family;
                self.italic = cx.parent.italic;
                self.line_height = cx.parent.line_height;
            }
            "font-size" => {
                if value == "inherit" {
                    self.font_size = cx.parent.font_size;
                    self.keyword_size = cx.parent.keyword_size;
                } else {
                    self.keyword_size = false;
                    let relative = Units { em: cx.parent.font_size, viewport: cx.viewport };
                    let size = match parse_len(value, relative) {
                        Some(Len::Px(px)) => px,
                        Some(Len::Percent(p)) => cx.parent.font_size * p,
                        _ => return false,
                    };
                    self.font_size = size;
                }
            }
            "font-weight" => {
                self.font_weight = match value {
                    "normal" => 400.0,
                    "bold" => 700.0,
                    "bolder" => 700.0,
                    "inherit" => cx.parent.font_weight,
                    other => match other.parse() {
                        Ok(w) => w,
                        Err(_) => return false,
                    },
                }
            }
            "font-family" => {
                self.family = if value == "inherit" {
                    cx.parent.family
                } else if value.to_ascii_lowercase().contains("mono") || value.contains("Consolas") {
                    Family::Mono
                } else {
                    Family::Sans
                }
            }
            "font-style" => self.italic = value == "italic",
            "line-height" => {
                self.line_height = if value == "normal" {
                    LineHeight::Normal
                } else if let Ok(factor) = value.parse::<f32>() {
                    LineHeight::Factor(factor)
                } else if let Some(px) = parse_px(value, units) {
                    LineHeight::Px(px)
                } else {
                    return false;
                }
            }
            "letter-spacing" => {
                let Some(px) = parse_px(value, units) else { return false };
                self.letter_spacing = px;
            }
            "text-align" => {
                self.text_align = match value {
                    "center" => TextAlign::Center,
                    "right" | "end" => TextAlign::Right,
                    _ => TextAlign::Left,
                }
            }
            "text-transform" => self.uppercase = value == "uppercase",
            "white-space" => {
                self.white_space = match value {
                    "nowrap" => WhiteSpace::Nowrap,
                    "pre" => WhiteSpace::Pre,
                    "pre-wrap" => WhiteSpace::PreWrap,
                    "pre-line" => WhiteSpace::PreLine,
                    _ => WhiteSpace::Normal,
                }
            }
            "text-overflow" => self.ellipsis = value == "ellipsis",
            "text-decoration" => self.underline = value.contains("underline"),
            "overflow-wrap" | "word-break" | "word-wrap" => self.break_anywhere = value != "normal",
            "user-select" => self.selectable = value != "none",
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(decls: &[(&str, &str)]) -> Computed {
        let parent = Computed::root();
        let cx = ApplyCx { parent: &parent, viewport: [1280.0, 860.0] };
        let mut style = Computed::child_of(&parent);
        for (prop, value) in decls {
            assert!(style.apply(prop, value, &cx), "unsupported: {prop}: {value}");
        }
        style
    }

    #[test]
    fn expands_shorthands_like_a_browser() {
        let s = applied(&[("padding", "0.6rem 1.25rem"), ("border", "1px solid rgba(88, 166, 255, 0.1)"), ("flex", "1"), ("margin", "0 auto")]);
        assert_eq!(s.padding, [Len::Px(9.6), Len::Px(20.0), Len::Px(9.6), Len::Px(20.0)]);
        assert_eq!(s.border(0), 1.0);
        assert_eq!((s.flex_grow, s.flex_shrink, s.flex_basis), (1.0, 1.0, Len::Px(0.0)));
        assert_eq!(s.margin[1], Len::Auto);
        let none = applied(&[("border", "none")]);
        assert_eq!(none.border(2), 0.0);
        let bottom = applied(&[("border", "none"), ("border-bottom", "2px solid transparent")]);
        assert_eq!((bottom.border(0), bottom.border(2)), (0.0, 2.0));
    }

    #[test]
    fn line_heights_land_on_the_same_64ths_as_in_a_browser() {
        let height = |size: &str, line: &str| applied(&[("font-size", size), ("line-height", line)]).line_height_px();
        assert_eq!(height("0.85rem", "1.6"), 21.75);
        assert_eq!(height("0.9rem", "1.6"), 23.046875);
        assert_eq!(height("0.85rem", "1.45"), 19.703125);
        assert_eq!(height("0.8rem", "1.6"), 20.46875);
        assert_eq!(height("20px", "1.4"), 28.0);
    }

    #[test]
    fn reads_svg_backgrounds_and_masks_from_data_urls() {
        let check = applied(&[
            ("background-image", "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'%3E%3Cpolyline points='20 6 9 17 4 12'/%3E%3C/svg%3E\"), linear-gradient(135deg, #58a6ff, #bc8cff)"),
            ("background-size", "12px 12px, cover"),
        ]);
        assert_eq!(check.background_svg.as_deref(), Some("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><polyline points='20 6 9 17 4 12'/></svg>"));
        assert_eq!(check.background_svg_size, Some([12.0, 12.0]));
        assert!(check.background_image.is_some());
        let mask = applied(&[("mask", "url(\"data:image/svg+xml;utf8,<svg viewBox='0 0 24 24' stroke='currentColor'><path d='M1 1h2'/></svg>\") center / contain no-repeat")]);
        assert_eq!(mask.mask_svg.as_deref(), Some("<svg viewBox='0 0 24 24' stroke='#fff'><path d='M1 1h2'/></svg>"));
        assert!(applied(&[("resize", "vertical")]).resize_vertical);
        assert!(applied(&[("-smd-center-text", "ink")]).centers_ink);
        assert!(!applied(&[("-smd-center-text", "line")]).centers_ink);
    }

    #[test]
    fn code_shrinks_to_13px_only_below_the_default_font_size() {
        let code = |parent: &Computed| {
            let mut style = Computed::child_of(parent);
            apply_tag_defaults("code", &mut style);
            style.font_size
        };
        assert_eq!(code(&Computed::root()), 13.0);
        assert_eq!(code(&applied(&[("font-size", "0.85rem")])), 13.6);
        assert_eq!(code(&Computed::child_of(&applied(&[("font-size", "0.8rem")]))), 12.8);
    }

    #[test]
    fn resolves_em_against_the_own_font_size() {
        let s = applied(&[("font-size", "0.8rem"), ("letter-spacing", "0.5em"), ("min-height", "1.4em")]);
        assert_eq!(s.font_size, 12.8);
        assert!((s.letter_spacing - 6.4).abs() < 0.001);
        assert!((s.min_height.px().unwrap() - 17.92).abs() < 0.001);
    }

    #[test]
    fn reads_transitions_and_backgrounds() {
        let s = applied(&[
            ("transition", "border-color 0.3s cubic-bezier(0.4, 0, 0.2, 1), box-shadow 0.2s"),
            ("background", "linear-gradient(135deg, #58a6ff, #bc8cff)"),
        ]);
        assert_eq!(s.transition_for("border-color"), 0.3);
        assert_eq!(s.transition_for("box-shadow"), 0.2);
        assert_eq!(s.transition_for("color"), 0.0);
        assert!(s.background_image.is_some());
    }
}
