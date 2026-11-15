use egui_taffy::taffy;
use taffy::prelude::{auto, fr, length, minmax, percent, repeat};
use taffy::{
    AlignContent, AlignItems, Dimension, FlexDirection, FlexWrap, GridTemplateComponent, LengthPercentage,
    LengthPercentageAuto, MaxTrackSizingFunction, MinTrackSizingFunction, Point, Rect, Size,
};

use crate::style::parse::split_top;
use crate::style::value::{parse_len, Units};
use crate::style::{Align, Computed, Display, Len, Overflow, Position};

pub fn snap(px: f32) -> f32 {
    (px * 64.0).floor() / 64.0
}

pub fn snap_up(px: f32) -> f32 {
    (px * 64.0).ceil() / 64.0
}

fn dimension(len: Len) -> Dimension {
    match len {
        Len::Auto => auto(),
        Len::Px(v) => length(snap(v)),
        Len::Percent(p) => percent(p),
        Len::Calc { px, percent: p } => {
            if p == 0.0 {
                length(snap(px))
            } else {
                percent(p)
            }
        }
    }
}

fn length_auto(len: Len) -> LengthPercentageAuto {
    match len {
        Len::Auto => LengthPercentageAuto::auto(),
        Len::Px(v) => length(snap(v)),
        Len::Percent(p) => percent(p),
        Len::Calc { px, percent: p } => {
            if p == 0.0 {
                length(snap(px))
            } else {
                percent(p)
            }
        }
    }
}

fn length_only(len: Len) -> LengthPercentage {
    match len {
        Len::Px(v) => length(snap(v)),
        Len::Percent(p) => percent(p),
        Len::Calc { px, .. } => length(snap(px)),
        Len::Auto => length(0.0),
    }
}

fn align_items(value: Align) -> Option<AlignItems> {
    Some(match value {
        Align::Auto => return None,
        Align::Start => AlignItems::FLEX_START,
        Align::End => AlignItems::FLEX_END,
        Align::Center => AlignItems::CENTER,
        Align::Stretch => AlignItems::STRETCH,
        Align::Baseline => AlignItems::BASELINE,
        _ => AlignItems::FLEX_START,
    })
}

fn align_content(value: Align) -> Option<AlignContent> {
    Some(match value {
        Align::Auto => return None,
        Align::Start => AlignContent::FLEX_START,
        Align::End => AlignContent::FLEX_END,
        Align::Center => AlignContent::CENTER,
        Align::Stretch => AlignContent::STRETCH,
        Align::SpaceBetween => AlignContent::SPACE_BETWEEN,
        Align::SpaceAround => AlignContent::SPACE_AROUND,
        Align::SpaceEvenly => AlignContent::SPACE_EVENLY,
        Align::Baseline => AlignContent::FLEX_START,
    })
}

fn min_track(text: &str, units: Units) -> Option<MinTrackSizingFunction> {
    Some(match text {
        "auto" => auto(),
        "0" => length(0.0),
        other => match parse_len(other, units)? {
            Len::Px(v) => length(v),
            Len::Percent(p) => percent(p),
            _ => return None,
        },
    })
}

fn max_track(text: &str, units: Units) -> Option<MaxTrackSizingFunction> {
    if let Some(share) = text.strip_suffix("fr") {
        return share.parse::<f32>().ok().map(fr);
    }
    Some(match text {
        "auto" => auto(),
        other => match parse_len(other, units)? {
            Len::Px(v) => length(v),
            Len::Percent(p) => percent(p),
            _ => return None,
        },
    })
}

fn track(text: &str, units: Units) -> Option<taffy::TrackSizingFunction> {
    if let Some(args) = text.strip_prefix("minmax(").and_then(|t| t.strip_suffix(')')) {
        let parts = split_top(args, ',');
        return Some(minmax(min_track(parts.first()?, units)?, max_track(parts.get(1)?, units)?));
    }
    if let Some(share) = text.strip_suffix("fr") {
        return share.parse::<f32>().ok().map(fr);
    }
    Some(match text {
        "auto" => auto(),
        other => match parse_len(other, units)? {
            Len::Px(v) => length(v),
            Len::Percent(p) => percent(p),
            _ => return None,
        },
    })
}

fn tracks(template: &str, units: Units) -> Vec<GridTemplateComponent<String>> {
    let mut out = Vec::new();
    for part in split_top(template, ' ') {
        if let Some(args) = part.strip_prefix("repeat(").and_then(|t| t.strip_suffix(')')) {
            let pieces = split_top(args, ',');
            let rest = pieces[1..].join(", ");
            let inner: Vec<taffy::TrackSizingFunction> = split_top(&rest, ' ').iter().filter_map(|t| track(t, units)).collect();
            match pieces.first().map(String::as_str) {
                Some("auto-fit") => out.push(repeat(taffy::RepetitionCount::AutoFit, inner)),
                Some("auto-fill") => out.push(repeat(taffy::RepetitionCount::AutoFill, inner)),
                Some(count) => {
                    if let Ok(count) = count.parse::<u16>() {
                        out.push(repeat(taffy::RepetitionCount::Count(count), inner));
                    }
                }
                None => {}
            }
        } else if let Some(single) = track(&part, units) {
            out.push(GridTemplateComponent::Single(single));
        }
    }
    out
}

pub fn to_taffy(style: &Computed, viewport: [f32; 2]) -> taffy::Style {
    let units = Units { em: style.font_size, viewport };
    let mut out = taffy::Style {
        display: match style.display {
            Display::None => taffy::Display::None,
            Display::Flex | Display::InlineFlex => taffy::Display::Flex,
            Display::Grid => taffy::Display::Grid,
            Display::Block | Display::Inline | Display::InlineBlock => taffy::Display::Block,
        },
        position: match style.position {
            Position::Absolute | Position::Fixed => taffy::Position::Absolute,
            _ => taffy::Position::Relative,
        },
        size: Size { width: dimension(style.width), height: dimension(style.height) },
        min_size: Size { width: length_auto(style.min_width), height: length_auto(style.min_height) },
        max_size: Size { width: length_auto(style.max_width), height: length_auto(style.max_height) },
        margin: Rect {
            top: length_auto(style.margin[0]),
            right: length_auto(style.margin[1]),
            bottom: length_auto(style.margin[2]),
            left: length_auto(style.margin[3]),
        },
        padding: Rect {
            top: length_only(style.padding[0]),
            right: length_only(style.padding[1]),
            bottom: length_only(style.padding[2]),
            left: length_only(style.padding[3]),
        },
        border: Rect {
            top: length(style.border(0)),
            right: length(style.border(1)),
            bottom: length(style.border(2)),
            left: length(style.border(3)),
        },
        flex_direction: if style.flex_row { FlexDirection::Row } else { FlexDirection::Column },
        flex_wrap: if style.flex_wrap { FlexWrap::Wrap } else { FlexWrap::NoWrap },
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: dimension(style.flex_basis),
        gap: Size { width: length(style.gap[1]), height: length(style.gap[0]) },
        overflow: Point {
            x: overflow(style.overflow[0]),
            y: overflow(style.overflow[1]),
        },
        ..Default::default()
    };
    if matches!(style.position, Position::Absolute | Position::Fixed | Position::Relative) {
        out.inset = Rect {
            top: length_auto(style.inset[0]),
            right: length_auto(style.inset[1]),
            bottom: length_auto(style.inset[2]),
            left: length_auto(style.inset[3]),
        };
    }
    if let Some(v) = align_items(style.align_items) {
        out.align_items = v;
    }
    if let Some(v) = align_items(style.align_self) {
        out.align_self = Some(v);
    }
    if let Some(v) = align_items(style.justify_items) {
        out.justify_items = v;
    }
    if let Some(v) = align_content(style.justify_content) {
        out.justify_content = v;
    }
    if let Some(template) = &style.grid_columns {
        out.grid_template_columns = tracks(template, units);
    }
    if let Some(template) = &style.grid_rows {
        out.grid_template_rows = tracks(template, units);
    }
    if style.grid_column.as_deref() == Some("1 / -1") {
        out.grid_column = taffy::Line { start: taffy::style_helpers::line(1), end: taffy::style_helpers::line(-1) };
    }
    if style.order < 0 {
        out.grid_row = taffy::Line { start: taffy::style_helpers::line(1), end: taffy::GridPlacement::Auto };
    }
    if let Some(area) = &style.grid_area {
        let lines: Vec<i16> = area.split('/').filter_map(|part| part.trim().parse().ok()).collect();
        if let [row, column] = lines[..] {
            out.grid_row = taffy::Line { start: taffy::style_helpers::line(row), end: taffy::GridPlacement::Auto };
            out.grid_column = taffy::Line { start: taffy::style_helpers::line(column), end: taffy::GridPlacement::Auto };
        }
    }
    out
}

fn overflow(value: Overflow) -> taffy::Overflow {
    match value {
        Overflow::Visible => taffy::Overflow::Visible,
        Overflow::Hidden => taffy::Overflow::Hidden,
        Overflow::Auto => taffy::Overflow::Scroll,
    }
}
