use std::f32::consts::{FRAC_PI_2, PI};

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{pos2, vec2, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

use crate::style::{BorderStyle, Computed, Gradient, Len};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowPx {
    pub inset: bool,
    pub offset: Vec2,
    pub blur: f32,
    pub spread: f32,
    pub color: Color32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Visual {
    pub background: Color32,
    pub gradient: Option<Gradient>,
    pub border: [Color32; 4],
    pub shadows: Vec<ShadowPx>,
    pub opacity: f32,
    pub translate: Vec2,
    pub scale: f32,
    pub rotate: f32,
    pub brightness: f32,
    pub color: Color32,
    pub outline: Color32,
    pub left: Option<f32>,
    pub width: Option<[f32; 2]>,
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Visual {
    pub fn of(style: &Computed, size: Vec2) -> Self {
        let translate = vec2(
            style.transform.translate[0].resolve(size.x).unwrap_or(0.0),
            style.transform.translate[1].resolve(size.y).unwrap_or(0.0),
        );
        Self {
            background: style.background_color,
            gradient: style.background_image.clone(),
            border: [0, 1, 2, 3].map(|i| style.border_color[i].resolve(style.color)),
            shadows: style
                .box_shadow
                .iter()
                .map(|s| ShadowPx { inset: s.inset, offset: vec2(s.x, s.y), blur: s.blur, spread: s.spread, color: s.color.resolve(style.color) })
                .collect(),
            opacity: style.opacity,
            translate,
            scale: style.transform.scale,
            rotate: 0.0,
            brightness: style.brightness,
            color: style.color,
            outline: style.outline_color.resolve(style.color),
            left: (style.transition_for("left") > 0.0).then(|| style.inset[3].px()).flatten(),
            width: (style.transition_for("width") > 0.0)
                .then(|| match style.width {
                    Len::Px(v) => Some([v, 0.0]),
                    Len::Percent(p) => Some([0.0, p]),
                    Len::Calc { px, percent } => Some([px, percent]),
                    Len::Auto => None,
                })
                .flatten(),
        }
    }

    pub fn lerp(&self, to: &Visual, t: f32) -> Visual {
        if t >= 1.0 {
            return to.clone();
        }
        if t <= 0.0 {
            return self.clone();
        }
        let count = self.shadows.len().max(to.shadows.len());
        let mut shadows = Vec::with_capacity(count);
        for i in 0..count {
            let fallback = |other: &ShadowPx| ShadowPx { color: Color32::TRANSPARENT, ..*other };
            let (a, b) = match (self.shadows.get(i), to.shadows.get(i)) {
                (Some(a), Some(b)) => (*a, *b),
                (Some(a), None) => (*a, fallback(a)),
                (None, Some(b)) => (fallback(b), *b),
                (None, None) => continue,
            };
            shadows.push(ShadowPx {
                inset: b.inset,
                offset: a.offset + (b.offset - a.offset) * t,
                blur: lerp(a.blur, b.blur, t),
                spread: lerp(a.spread, b.spread, t),
                color: lerp_color(a.color, b.color, t),
            });
        }
        Visual {
            background: lerp_color(self.background, to.background, t),
            gradient: if t < 0.5 && to.gradient.is_none() { self.gradient.clone() } else { to.gradient.clone().or_else(|| self.gradient.clone().filter(|_| t < 0.5)) },
            border: [0, 1, 2, 3].map(|i| lerp_color(self.border[i], to.border[i], t)),
            shadows,
            opacity: lerp(self.opacity, to.opacity, t),
            translate: self.translate + (to.translate - self.translate) * t,
            scale: lerp(self.scale, to.scale, t),
            rotate: to.rotate,
            brightness: lerp(self.brightness, to.brightness, t),
            color: lerp_color(self.color, to.color, t),
            outline: lerp_color(self.outline, to.outline, t),
            left: match (self.left, to.left) {
                (Some(a), Some(b)) => Some(lerp(a, b, t)),
                _ => to.left,
            },
            width: match (self.width, to.width) {
                (Some(a), Some(b)) => Some([lerp(a[0], b[0], t), lerp(a[1], b[1], t)]),
                _ => to.width,
            },
        }
    }
}

pub fn ease(t: f32) -> f32 {
    let (x1, y1, x2, y2) = (0.4f32, 0.0f32, 0.2f32, 1.0f32);
    let t = t.clamp(0.0, 1.0);
    let curve = |a: f32, b: f32, s: f32| 3.0 * (1.0 - s) * (1.0 - s) * s * a + 3.0 * (1.0 - s) * s * s * b + s * s * s;
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    let mut s = t;
    for _ in 0..24 {
        let x = curve(x1, x2, s);
        if (x - t).abs() < 0.0005 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) / 2.0;
    }
    curve(y1, y2, s)
}

pub fn radii(style: &Computed, rect: Rect) -> [f32; 4] {
    let limit = rect.width().min(rect.height()) / 2.0;
    [0, 1, 2, 3].map(|i| match style.radius[i] {
        Len::Px(v) => v.clamp(0.0, limit.max(0.0)),
        Len::Percent(p) => (rect.width().min(rect.height()) * p).clamp(0.0, limit.max(0.0)),
        Len::Calc { px, .. } => px.clamp(0.0, limit.max(0.0)),
        Len::Auto => 0.0,
    })
}

pub fn radius_of(style: &Computed, rect: Rect) -> CornerRadius {
    corner_radius(radii(style, rect))
}

fn corner_radius(radii: [f32; 4]) -> CornerRadius {
    let u = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    CornerRadius { nw: u(radii[0]), ne: u(radii[1]), se: u(radii[2]), sw: u(radii[3]) }
}

fn outline(rect: Rect, radii: [f32; 4], pixels_per_point: f32) -> Vec<(Pos2, Vec2)> {
    let corners = [
        (pos2(rect.right() - radii[1], rect.top() + radii[1]), -FRAC_PI_2, radii[1]),
        (pos2(rect.right() - radii[2], rect.bottom() - radii[2]), 0.0, radii[2]),
        (pos2(rect.left() + radii[3], rect.bottom() - radii[3]), FRAC_PI_2, radii[3]),
        (pos2(rect.left() + radii[0], rect.top() + radii[0]), PI, radii[0]),
    ];
    let mut out = Vec::new();
    for (center, start, r) in corners {
        let steps = if r <= 0.0 { 1 } else { ((r * pixels_per_point * FRAC_PI_2 / 1.5).ceil() as usize).clamp(3, 32) };
        for i in 0..=steps {
            let a = start + FRAC_PI_2 * (i as f32 / steps as f32);
            let n = vec2(a.cos(), a.sin());
            out.push((center + n * r, n));
        }
    }
    out
}

fn clip_half_plane(points: &[(Pos2, Vec2, bool)], direction: Vec2, limit: f32, keep_below: bool) -> Vec<(Pos2, Vec2, bool)> {
    let side = |p: Pos2| {
        let d = p.to_vec2().dot(direction) - limit;
        if keep_below {
            -d
        } else {
            d
        }
    };
    let mut out = Vec::with_capacity(points.len() + 2);
    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        let (a, b) = (side(current.0), side(next.0));
        if a >= 0.0 {
            out.push(current);
        }
        if (a >= 0.0) != (b >= 0.0) {
            let t = a / (a - b);
            let p = current.0 + (next.0 - current.0) * t;
            out.push((p, Vec2::ZERO, a >= 0.0));
        }
    }
    out
}

fn fan(painter: &Painter, points: &[(Pos2, Vec2, bool)], color_at: &dyn Fn(Pos2) -> Color32) {
    if points.len() < 3 {
        return;
    }
    let feather = 1.0 / painter.pixels_per_point();
    let center = points.iter().fold(Vec2::ZERO, |acc, p| acc + p.0.to_vec2()) / points.len() as f32;
    let center = center.to_pos2();
    let n = points.len() as u32;
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex { pos: center, uv: WHITE_UV, color: color_at(center) });
    for (p, normal, _) in points {
        let inner = *p - *normal * (feather * 0.5);
        let outer = *p + *normal * (feather * 0.5);
        mesh.vertices.push(Vertex { pos: inner, uv: WHITE_UV, color: color_at(inner) });
        mesh.vertices.push(Vertex { pos: outer, uv: WHITE_UV, color: if *normal == Vec2::ZERO { color_at(outer) } else { Color32::TRANSPARENT } });
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let (ii, io) = (1 + 2 * i, 2 + 2 * i);
        let (ji, jo) = (1 + 2 * j, 2 + 2 * j);
        mesh.indices.extend_from_slice(&[0, ii, ji]);
        mesh.indices.extend_from_slice(&[ii, io, jo, ii, jo, ji]);
    }
    painter.add(Shape::mesh(mesh));
}

pub fn fill_gradient(painter: &Painter, rect: Rect, radii: [f32; 4], gradient: &Gradient, opacity: f32, brightness: f32) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let tint = |c: Color32| {
        let lit = if brightness == 1.0 {
            c
        } else {
            let f = |v: u8| ((v as f32) * brightness).round().min(255.0) as u8;
            Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
        };
        if opacity < 1.0 {
            lit.gamma_multiply(opacity)
        } else {
            lit
        }
    };
    let angle = gradient.angle.to_radians();
    let direction = vec2(angle.sin(), -angle.cos());
    let length = (rect.width() * direction.x.abs() + rect.height() * direction.y.abs()).max(0.001);
    let origin = rect.center().to_vec2().dot(direction) - length / 2.0;
    let stops: Vec<(f32, Color32)> = gradient.positions().into_iter().map(|(p, c)| (p, tint(c))).collect();
    let base: Vec<(Pos2, Vec2, bool)> = outline(rect, radii, painter.pixels_per_point()).into_iter().map(|(p, n)| (p, n, false)).collect();
    for pair in stops.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if to.0 <= from.0 {
            continue;
        }
        let low = origin + from.0 * length;
        let high = origin + to.0 * length;
        let mut polygon = base.clone();
        if from.0 > 0.0 {
            polygon = clip_half_plane(&polygon, direction, low, false);
        }
        if to.0 < 1.0 {
            polygon = clip_half_plane(&polygon, direction, high, true);
        }
        let color_at = |p: Pos2| {
            let t = ((p.to_vec2().dot(direction) - low) / (high - low)).clamp(0.0, 1.0);
            lerp_color(from.1, to.1, t)
        };
        fan(painter, &polygon, &color_at);
    }
}

fn uniform(values: [f32; 4]) -> Option<f32> {
    (values[1] == values[0] && values[2] == values[0] && values[3] == values[0]).then_some(values[0])
}

fn dashed_border(painter: &Painter, rect: Rect, radii: [f32; 4], width: f32, color: Color32) {
    let inset = rect.shrink(width * 0.5);
    let shrunk = radii.map(|r| (r - width * 0.5).max(0.0));
    let mut points: Vec<Pos2> = outline(inset, shrunk, painter.pixels_per_point()).into_iter().map(|(p, _)| p).collect();
    if let Some(first) = points.first().copied() {
        points.push(first);
    }
    painter.extend(Shape::dashed_line(&points, Stroke::new(width, color), width * 3.0, width * 2.0));
}

pub fn paint_svg(ui: &egui::Ui, rect: Rect, svg: &std::sync::Arc<str>, tint: Color32) {
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::{Arc, Mutex, OnceLock};
    type Source = (String, Arc<[u8]>);
    static CACHE: OnceLock<Mutex<HashMap<u64, Source>>> = OnceLock::new();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    svg.hash(&mut hasher);
    let key = hasher.finish();
    let (uri, bytes) = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(key)
        .or_insert_with(|| (format!("bytes://css/{key:x}.svg"), Arc::from(svg.as_bytes())))
        .clone();
    egui::Image::from_bytes(uri, egui::load::Bytes::Shared(bytes)).tint(tint).paint_at(ui, rect);
}

pub fn paint_box(painter: &Painter, rect: Rect, style: &Computed, visual: &Visual) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let radii = radii(style, rect);
    let rounding = corner_radius(radii);
    for shadow in visual.shadows.iter().filter(|s| !s.inset && s.color.a() > 0) {
        if shadow.blur <= 0.0 {
            if shadow.spread > 0.0 {
                let expanded = CornerRadius {
                    nw: (radii[0] + shadow.spread).round() as u8,
                    ne: (radii[1] + shadow.spread).round() as u8,
                    se: (radii[2] + shadow.spread).round() as u8,
                    sw: (radii[3] + shadow.spread).round() as u8,
                };
                painter.rect_stroke(
                    rect.translate(shadow.offset).expand(shadow.spread),
                    expanded,
                    Stroke::new(shadow.spread, shadow.color),
                    StrokeKind::Inside,
                );
            }
            continue;
        }
        let shape = egui::epaint::Shadow {
            offset: [shadow.offset.x.round() as i8, shadow.offset.y.round() as i8],
            blur: shadow.blur.round().clamp(0.0, 255.0) as u8,
            spread: shadow.spread.round().clamp(0.0, 255.0) as u8,
            color: shadow.color,
        }
        .as_shape(rect, rounding);
        painter.add(shape);
    }
    if visual.background.a() > 0 {
        painter.rect_filled(rect, rounding, visual.background);
    }
    if let Some(gradient) = visual.gradient.as_ref().filter(|g| !g.radial && !style.background_clip_text) {
        fill_gradient(painter, rect, radii, gradient, 1.0, visual.brightness);
    }
    for shadow in visual.shadows.iter().filter(|s| s.inset && s.color.a() > 0) {
        if shadow.offset == Vec2::ZERO && shadow.spread > 0.0 {
            painter.rect_stroke(rect, rounding, Stroke::new(shadow.spread, shadow.color), StrokeKind::Inside);
        } else if shadow.offset.x > 0.0 {
            let bar = Rect::from_min_max(rect.min, pos2(rect.left() + shadow.offset.x, rect.bottom()));
            painter.rect_filled(bar, CornerRadius { nw: rounding.nw, sw: rounding.sw, ne: 0, se: 0 }, shadow.color);
        } else if shadow.offset.x < 0.0 {
            let bar = Rect::from_min_max(pos2(rect.right() + shadow.offset.x, rect.top()), rect.max);
            painter.rect_filled(bar, CornerRadius { ne: rounding.ne, se: rounding.se, nw: 0, sw: 0 }, shadow.color);
        } else if shadow.offset.y < 0.0 {
            let bar = Rect::from_min_max(pos2(rect.left(), rect.bottom() + shadow.offset.y), rect.max);
            painter.rect_filled(bar, CornerRadius { sw: rounding.sw, se: rounding.se, nw: 0, ne: 0 }, shadow.color);
        } else if shadow.offset.y > 0.0 {
            let bar = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + shadow.offset.y));
            painter.rect_filled(bar, CornerRadius { nw: rounding.nw, ne: rounding.ne, sw: 0, se: 0 }, shadow.color);
        }
    }
    let widths = [0, 1, 2, 3].map(|i| style.border(i));
    let same_color = visual.border[1] == visual.border[0] && visual.border[2] == visual.border[0] && visual.border[3] == visual.border[0];
    match uniform(widths) {
        Some(width) if width > 0.0 && same_color && visual.border[0].a() > 0 => {
            if style.border_style[0] == BorderStyle::Dashed {
                dashed_border(painter, rect, radii, width, visual.border[0]);
            } else {
                painter.rect_stroke(rect, rounding, Stroke::new(width, visual.border[0]), StrokeKind::Inside);
            }
        }
        Some(_) if same_color => {}
        Some(width) if width > 0.0 && radii.iter().all(|r| *r >= rect.width().min(rect.height()) / 2.0 - 0.5) => {
            let center = rect.center();
            let radius = rect.width().min(rect.height()) / 2.0 - width / 2.0;
            let usual = if visual.border[1] == visual.border[2] || visual.border[1] == visual.border[3] { visual.border[1] } else { visual.border[0] };
            if usual.a() > 0 {
                painter.circle_stroke(center, radius, Stroke::new(width, usual));
            }
            for side in 0..4 {
                if visual.border[side] == usual || visual.border[side].a() == 0 {
                    continue;
                }
                let middle = visual.rotate + (side as f32 - 1.0) * std::f32::consts::FRAC_PI_2;
                let points: Vec<egui::Pos2> = (0..=24)
                    .map(|step| {
                        let angle = middle + (step as f32 / 24.0 - 0.5) * std::f32::consts::FRAC_PI_2;
                        center + radius * vec2(angle.cos(), angle.sin())
                    })
                    .collect();
                painter.add(egui::Shape::line(points, Stroke::new(width, visual.border[side])));
            }
        }
        None if same_color && visual.border[0].a() > 0 && partial_width(widths).is_some() => {
            let width = partial_width(widths).unwrap_or(0.0);
            let inner = rect.shrink(width / 2.0);
            let corners = [inner.right_top(), inner.right_bottom(), inner.left_bottom(), inner.left_top()];
            let present = |side: usize| widths[side % 4] > 0.0;
            let Some(first) = (0..4).find(|side| !present(*side)) else { return };
            let mut side = first + 1;
            while side < first + 4 {
                if !present(side) {
                    side += 1;
                    continue;
                }
                let mut points = vec![corners[(side + 3) % 4]];
                while present(side) && side < first + 4 {
                    let corner = side % 4;
                    let reach = (radii[(corner + 1) % 4] - width / 2.0).max(0.0);
                    if present(side + 1) && side + 1 < first + 4 && reach > 0.0 {
                        let toward = [vec2(-1.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(1.0, 1.0)][corner];
                        let center = corners[corner] + toward * reach;
                        let from = (corner as f32 - 1.0) * std::f32::consts::FRAC_PI_2;
                        for step in 0..=8 {
                            let angle = from + step as f32 / 8.0 * std::f32::consts::FRAC_PI_2;
                            points.push(center + reach * vec2(angle.cos(), angle.sin()));
                        }
                    } else {
                        points.push(corners[corner]);
                    }
                    side += 1;
                }
                painter.add(egui::Shape::line(points, Stroke::new(width, visual.border[0])));
            }
        }
        _ => {
            let strips = [
                Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + widths[0])),
                Rect::from_min_max(pos2(rect.right() - widths[1], rect.top()), rect.max),
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - widths[2]), rect.max),
                Rect::from_min_max(rect.min, pos2(rect.left() + widths[3], rect.bottom())),
            ];
            for side in 0..4 {
                if widths[side] > 0.0 && visual.border[side].a() > 0 {
                    painter.rect_filled(strips[side], 0.0, visual.border[side]);
                }
            }
        }
    }
}

fn partial_width(widths: [f32; 4]) -> Option<f32> {
    let width = widths.iter().copied().fold(0.0, f32::max);
    (width > 0.0 && widths.iter().all(|w| *w == 0.0 || *w == width)).then_some(width)
}

pub fn paint_outline(painter: &Painter, rect: Rect, style: &Computed, visual: &Visual) {
    if style.outline_width <= 0.0 || visual.outline.a() == 0 {
        return;
    }
    let grow = style.outline_offset + style.outline_width;
    let radii = radii(style, rect).map(|r| r + grow);
    painter.rect_stroke(rect.expand(grow), corner_radius(radii), Stroke::new(style.outline_width, visual.outline), StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_starts_slow_and_ends_at_the_target() {
        assert_eq!(ease(0.0), 0.0);
        assert!((ease(1.0) - 1.0).abs() < 0.001);
        assert!(ease(0.25) < 0.25 && ease(0.75) > 0.75);
    }

    #[test]
    fn cuts_a_square_at_a_line() {
        let square = [
            (pos2(0.0, 0.0), Vec2::ZERO, false),
            (pos2(10.0, 0.0), Vec2::ZERO, false),
            (pos2(10.0, 10.0), Vec2::ZERO, false),
            (pos2(0.0, 10.0), Vec2::ZERO, false),
        ];
        let left = clip_half_plane(&square, vec2(1.0, 0.0), 4.0, true);
        assert_eq!(left.len(), 4);
        assert!(left.iter().all(|(p, _, _)| p.x <= 4.0 + 0.001));
    }
}

pub fn paint_native_control(painter: &Painter, rect: Rect, style: &Computed, checked: bool, opacity: f32) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let fade = |color: Color32| color.gamma_multiply(opacity);
    let accent = style.accent.unwrap_or(Color32::from_rgb(0, 117, 255));
    let outline = Color32::from_rgb(118, 118, 118);
    let size = rect.width().min(rect.height());
    let linear = |channel: u8| {
        let value = channel as f32 / 255.0;
        if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
    };
    let luminance = 0.2126 * linear(accent.r()) + 0.7152 * linear(accent.g()) + 0.0722 * linear(accent.b());
    let on_accent = if 1.05 / (luminance + 0.05) < 3.0 { Color32::from_rgb(59, 59, 59) } else { Color32::WHITE };
    if style.native_control == 2 {
        let radius = size / 2.0;
        let center = rect.center();
        let inside = if checked { on_accent } else { Color32::WHITE };
        painter.circle(center, radius - 0.5, fade(inside), Stroke::new(1.0, fade(if checked { accent } else { outline })));
        if checked {
            painter.circle_filled(center, radius - 3.0, fade(accent));
        }
        return;
    }
    let rounding = CornerRadius::same(2);
    if checked {
        painter.rect_filled(rect, rounding, fade(accent));
        let at = |x: f32, y: f32| pos2(rect.left() + x * size, rect.top() + y * size);
        painter.line(vec![at(0.22, 0.52), at(0.42, 0.72), at(0.78, 0.30)], Stroke::new((size / 6.5).max(1.5), fade(on_accent)));
    } else {
        painter.rect_filled(rect, rounding, fade(Color32::WHITE));
        painter.rect_stroke(rect, rounding, Stroke::new(1.0, fade(outline)), StrokeKind::Inside);
    }
}
