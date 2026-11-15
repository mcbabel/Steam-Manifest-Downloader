use egui::Color32;

use super::parse::split_top;

pub const ROOT_FONT: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Len {
    Auto,
    Px(f32),
    Percent(f32),
    Calc { px: f32, percent: f32 },
}

impl Len {
    pub fn px(self) -> Option<f32> {
        match self {
            Len::Px(v) => Some(v),
            _ => None,
        }
    }

    pub fn resolve(self, basis: f32) -> Option<f32> {
        match self {
            Len::Auto => None,
            Len::Px(v) => Some(v),
            Len::Percent(p) => Some(basis * p),
            Len::Calc { px, percent } => Some(px + basis * percent),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Units {
    pub em: f32,
    pub viewport: [f32; 2],
}

fn number_prefix(text: &str) -> Option<(f32, &str)> {
    let end = text
        .char_indices()
        .find(|(i, c)| !(c.is_ascii_digit() || *c == '.' || (*i == 0 && (*c == '-' || *c == '+'))))
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    if end == 0 {
        return None;
    }
    text[..end].parse::<f32>().ok().map(|n| (n, &text[end..]))
}

fn function_args<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let text = text.trim();
    if text.len() > name.len() + 1 && text[..name.len()].eq_ignore_ascii_case(name) && text[name.len()..].starts_with('(') && text.ends_with(')') {
        Some(&text[name.len() + 1..text.len() - 1])
    } else {
        None
    }
}

fn calc_terms(expr: &str, units: Units) -> Option<Len> {
    let mut px = 0.0;
    let mut percent = 0.0;
    let mut sign = 1.0;
    for token in split_top(expr, ' ') {
        match token.as_str() {
            "+" => sign = 1.0,
            "-" => sign = -1.0,
            term => {
                match parse_len(term, units)? {
                    Len::Px(v) => px += sign * v,
                    Len::Percent(p) => percent += sign * p,
                    Len::Calc { px: a, percent: b } => {
                        px += sign * a;
                        percent += sign * b;
                    }
                    Len::Auto => return None,
                }
                sign = 1.0;
            }
        }
    }
    Some(if percent == 0.0 { Len::Px(px) } else { Len::Calc { px, percent } })
}

pub fn parse_len(text: &str, units: Units) -> Option<Len> {
    let text = text.trim();
    match text {
        "auto" | "none" | "unset" | "initial" | "normal" => return Some(Len::Auto),
        "0" => return Some(Len::Px(0.0)),
        _ => {}
    }
    if let Some(args) = function_args(text, "calc") {
        return calc_terms(args, units);
    }
    for (name, pick_min) in [("min", true), ("max", false)] {
        if let Some(args) = function_args(text, name) {
            let values: Vec<f32> = split_top(args, ',').iter().filter_map(|a| parse_len(a, units)?.px()).collect();
            let picked = if pick_min { values.iter().cloned().fold(f32::INFINITY, f32::min) } else { values.iter().cloned().fold(f32::NEG_INFINITY, f32::max) };
            return picked.is_finite().then_some(Len::Px(picked));
        }
    }
    let (number, unit) = number_prefix(text)?;
    Some(match unit.trim() {
        "px" | "" => Len::Px(number),
        "rem" => Len::Px(number * ROOT_FONT),
        "em" => Len::Px(number * units.em),
        "%" => Len::Percent(number / 100.0),
        "vh" => Len::Px(number / 100.0 * units.viewport[1]),
        "vw" => Len::Px(number / 100.0 * units.viewport[0]),
        _ => return None,
    })
}

pub fn parse_px(text: &str, units: Units) -> Option<f32> {
    parse_len(text, units)?.px()
}

pub fn parse_seconds(text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(ms) = text.strip_suffix("ms") {
        return ms.trim().parse::<f32>().ok().map(|v| v / 1000.0);
    }
    text.strip_suffix('s')?.trim().parse().ok()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Col {
    Current,
    Rgba(Color32),
}

impl Col {
    pub fn resolve(self, current: Color32) -> Color32 {
        match self {
            Col::Current => current,
            Col::Rgba(c) => c,
        }
    }
}

fn channel(text: &str) -> Option<u8> {
    let text = text.trim();
    if let Some(p) = text.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().map(|v| (v * 2.55).round().clamp(0.0, 255.0) as u8);
    }
    text.parse::<f32>().ok().map(|v| v.round().clamp(0.0, 255.0) as u8)
}

fn alpha(text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(p) = text.strip_suffix('%') {
        return p.trim().parse::<f32>().ok().map(|v| (v / 100.0).clamp(0.0, 1.0));
    }
    text.parse::<f32>().ok().map(|v| v.clamp(0.0, 1.0))
}

fn hex(text: &str) -> Option<Color32> {
    let digits: Vec<u8> = text.chars().map(|c| c.to_digit(16).map(|d| d as u8)).collect::<Option<_>>()?;
    let pair = |i: usize| digits[i] * 16 + digits[i + 1];
    Some(match digits.len() {
        3 => Color32::from_rgb(digits[0] * 17, digits[1] * 17, digits[2] * 17),
        4 => Color32::from_rgba_unmultiplied(digits[0] * 17, digits[1] * 17, digits[2] * 17, digits[3] * 17),
        6 => Color32::from_rgb(pair(0), pair(2), pair(4)),
        8 => Color32::from_rgba_unmultiplied(pair(0), pair(2), pair(4), pair(6)),
        _ => return None,
    })
}

fn unmultiplied(color: Color32) -> [f32; 4] {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    [r as f32, g as f32, b as f32, a as f32 / 255.0]
}

fn mix(first: Color32, share: f32, second: Color32) -> Color32 {
    let a = unmultiplied(first);
    let b = unmultiplied(second);
    let wa = share * a[3];
    let wb = (1.0 - share) * b[3];
    let alpha = wa + wb;
    if alpha <= 0.0 {
        return Color32::TRANSPARENT;
    }
    let ch = |i: usize| ((a[i] * wa + b[i] * wb) / alpha).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(ch(0), ch(1), ch(2), (alpha * 255.0).round().clamp(0.0, 255.0) as u8)
}

pub fn parse_color(text: &str) -> Option<Col> {
    let text = text.trim();
    let lower = text.to_ascii_lowercase();
    let named = match lower.as_str() {
        "transparent" => Some(Color32::TRANSPARENT),
        "white" => Some(Color32::WHITE),
        "black" => Some(Color32::BLACK),
        "red" => Some(Color32::from_rgb(255, 0, 0)),
        "gray" | "grey" => Some(Color32::from_rgb(128, 128, 128)),
        "currentcolor" | "inherit" => return Some(Col::Current),
        _ => None,
    };
    if let Some(color) = named {
        return Some(Col::Rgba(color));
    }
    if let Some(digits) = lower.strip_prefix('#') {
        return hex(digits).map(Col::Rgba);
    }
    for name in ["rgba", "rgb"] {
        if let Some(args) = function_args(&lower, name) {
            let parts: Vec<String> = if args.contains(',') {
                split_top(args, ',')
            } else {
                args.replace('/', " ").split_whitespace().map(str::to_string).collect()
            };
            if parts.len() < 3 {
                return None;
            }
            let a = parts.get(3).and_then(|p| alpha(p)).unwrap_or(1.0);
            return Some(Col::Rgba(Color32::from_rgba_unmultiplied(
                channel(&parts[0])?,
                channel(&parts[1])?,
                channel(&parts[2])?,
                (a * 255.0).round() as u8,
            )));
        }
    }
    if let Some(args) = function_args(&lower, "color-mix") {
        let parts = split_top(args, ',');
        if parts.len() == 3 {
            let side = |part: &str| -> Option<(Color32, Option<f32>)> {
                let tokens = split_top(part, ' ');
                let share = tokens.iter().find_map(|t| t.strip_suffix('%').and_then(|p| p.parse::<f32>().ok())).map(|p| p / 100.0);
                let color = tokens.iter().find(|t| !t.ends_with('%')).and_then(|t| parse_color(t))?;
                Some((color.resolve(Color32::TRANSPARENT), share))
            };
            let (first, first_share) = side(&parts[1])?;
            let (second, second_share) = side(&parts[2])?;
            let share = first_share.or(second_share.map(|s| 1.0 - s)).unwrap_or(0.5);
            return Some(Col::Rgba(mix(first, share, second)));
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub radial: bool,
    pub angle: f32,
    pub stops: Vec<(Color32, Option<f32>)>,
}

impl Gradient {
    pub fn positions(&self) -> Vec<(f32, Color32)> {
        let n = self.stops.len();
        let mut out: Vec<(f32, Color32)> = Vec::with_capacity(n);
        let mut known: Vec<Option<f32>> = self.stops.iter().map(|s| s.1).collect();
        if n > 0 && known[0].is_none() {
            known[0] = Some(0.0);
        }
        if n > 1 && known[n - 1].is_none() {
            known[n - 1] = Some(1.0);
        }
        let mut i = 0;
        while i < n {
            if known[i].is_none() {
                let start = i - 1;
                let mut end = i;
                while known[end].is_none() {
                    end += 1;
                }
                let (a, b) = (known[start].unwrap(), known[end].unwrap());
                for (k, slot) in known.iter_mut().enumerate().take(end).skip(i) {
                    *slot = Some(a + (b - a) * (k - start) as f32 / (end - start) as f32);
                }
                i = end;
            } else {
                i += 1;
            }
        }
        for (stop, position) in self.stops.iter().zip(known) {
            out.push((position.unwrap_or(0.0), stop.0));
        }
        out
    }
}

pub fn parse_gradient(text: &str) -> Option<Gradient> {
    let text = text.trim();
    let (radial, args) = if let Some(args) = function_args(text, "linear-gradient") {
        (false, args)
    } else if let Some(args) = function_args(text, "radial-gradient") {
        (true, args)
    } else {
        return None;
    };
    let mut parts = split_top(args, ',');
    let mut angle = 180.0;
    if let Some(first) = parts.first() {
        if let Some(deg) = first.strip_suffix("deg") {
            angle = deg.trim().parse().ok()?;
            parts.remove(0);
        } else if first.starts_with("to ") {
            angle = match first.as_str() {
                "to right" => 90.0,
                "to left" => 270.0,
                "to top" => 0.0,
                "to bottom" => 180.0,
                "to bottom right" => 135.0,
                _ => 180.0,
            };
            parts.remove(0);
        } else if radial && parse_color(first.split_whitespace().next().unwrap_or("")).is_none() {
            parts.remove(0);
        }
    }
    let mut stops = Vec::new();
    for part in parts {
        let tokens = split_top(&part, ' ');
        let color = parse_color(tokens.first()?)?.resolve(Color32::TRANSPARENT);
        let position = tokens.get(1).and_then(|t| t.strip_suffix('%')).and_then(|p| p.parse::<f32>().ok()).map(|p| p / 100.0);
        stops.push((color, position));
    }
    (stops.len() >= 2).then_some(Gradient { radial, angle, stops })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub inset: bool,
    pub x: f32,
    pub y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color: Col,
}

pub fn parse_shadows(text: &str, units: Units) -> Option<Vec<Shadow>> {
    if text.trim() == "none" {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for part in split_top(text, ',') {
        let mut shadow = Shadow { inset: false, x: 0.0, y: 0.0, blur: 0.0, spread: 0.0, color: Col::Current };
        let mut lengths = Vec::new();
        for token in split_top(&part, ' ') {
            if token == "inset" {
                shadow.inset = true;
            } else if let Some(px) = parse_px(&token, units) {
                lengths.push(px);
            } else if let Some(color) = parse_color(&token) {
                shadow.color = color;
            } else {
                return None;
            }
        }
        if lengths.len() < 2 {
            return None;
        }
        shadow.x = lengths[0];
        shadow.y = lengths[1];
        shadow.blur = lengths.get(2).copied().unwrap_or(0.0);
        shadow.spread = lengths.get(3).copied().unwrap_or(0.0);
        out.push(shadow);
    }
    Some(out)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub translate: [Len; 2],
    pub scale: f32,
    pub rotate: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self { translate: [Len::Px(0.0), Len::Px(0.0)], scale: 1.0, rotate: 0.0 }
    }
}

impl Transform {
    pub fn is_identity(&self) -> bool {
        *self == Self::default()
    }
}

pub fn parse_transform(text: &str, units: Units) -> Option<Transform> {
    let mut transform = Transform::default();
    if text.trim() == "none" {
        return Some(transform);
    }
    for part in split_top(text, ' ') {
        let open = part.find('(')?;
        let name = &part[..open];
        let args = part[open + 1..].trim_end_matches(')');
        match name {
            "translateX" => transform.translate[0] = parse_len(args, units)?,
            "translateY" => transform.translate[1] = parse_len(args, units)?,
            "translate" => {
                let both = split_top(args, ',');
                transform.translate[0] = parse_len(both.first()?, units)?;
                transform.translate[1] = both.get(1).and_then(|v| parse_len(v, units)).unwrap_or(Len::Px(0.0));
            }
            "scale" => transform.scale = args.trim().parse().ok()?,
            "rotate" => transform.rotate = args.trim().strip_suffix("deg")?.parse::<f32>().ok()?.to_radians(),
            _ => return None,
        }
    }
    Some(transform)
}

pub fn resolve_vars(value: &str, lookup: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = value.to_string();
    for _ in 0..16 {
        let Some(start) = out.find("var(") else { break };
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in out[start..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else { break };
        let inner = out[start + 4..end].to_string();
        let (name, fallback) = match inner.split_once(',') {
            Some((name, fallback)) => (name.trim().to_string(), Some(fallback.trim().to_string())),
            None => (inner.trim().to_string(), None),
        };
        let replacement = lookup(&name).or(fallback).unwrap_or_default();
        out.replace_range(start..=end, &replacement);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNITS: Units = Units { em: 20.0, viewport: [1000.0, 800.0] };

    #[test]
    fn reads_lengths_in_every_unit_the_stylesheet_uses() {
        assert_eq!(parse_len("1.5rem", UNITS), Some(Len::Px(24.0)));
        assert_eq!(parse_len("0.5em", UNITS), Some(Len::Px(10.0)));
        assert_eq!(parse_len("90%", UNITS), Some(Len::Percent(0.9)));
        assert_eq!(parse_len("calc(85vh - 2rem)", UNITS), Some(Len::Px(648.0)));
        assert_eq!(parse_len("calc(100% + 0.35rem)", UNITS), Some(Len::Calc { px: 5.6, percent: 1.0 }));
        assert_eq!(parse_len("min(90vw, 520px)", UNITS), Some(Len::Px(520.0)));
        assert_eq!(parse_len("auto", UNITS), Some(Len::Auto));
        assert_eq!(parse_len("-2px", UNITS), Some(Len::Px(-2.0)));
    }

    #[test]
    fn reads_colors_gradients_and_shadows() {
        assert_eq!(parse_color("#58a6ff"), Some(Col::Rgba(Color32::from_rgb(0x58, 0xa6, 0xff))));
        assert_eq!(parse_color("rgba(88, 166, 255, 0.1)"), Some(Col::Rgba(Color32::from_rgba_unmultiplied(88, 166, 255, 26))));
        assert_eq!(parse_color("currentColor"), Some(Col::Current));
        let mixed = parse_color("color-mix(in srgb, #f85149 55%, transparent)").unwrap().resolve(Color32::BLACK);
        let [r, g, b, a] = mixed.to_srgba_unmultiplied();
        assert!(r.abs_diff(248) <= 1 && g.abs_diff(81) <= 1 && b.abs_diff(73) <= 1 && a == 140);
        let gradient = parse_gradient("linear-gradient(90deg, transparent 0%, rgba(255, 255, 255, 0.12) 50%, transparent 100%)").unwrap();
        assert_eq!(gradient.angle, 90.0);
        assert_eq!(gradient.positions().iter().map(|p| p.0).collect::<Vec<_>>(), vec![0.0, 0.5, 1.0]);
        let three = parse_gradient("linear-gradient(135deg, #000, #111, #222)").unwrap();
        assert_eq!(three.positions()[1].0, 0.5);
        let shadows = parse_shadows("0 6px 18px rgba(0,0,0,0.5), inset 0 0 0 1px #fff", UNITS).unwrap();
        assert_eq!(shadows.len(), 2);
        assert!(shadows[1].inset && shadows[1].spread == 1.0 && shadows[0].blur == 18.0);
    }

    #[test]
    fn replaces_variables_with_fallbacks() {
        let lookup = |name: &str| match name {
            "--a" => Some("var(--b)".to_string()),
            "--b" => Some("#fff".to_string()),
            _ => None,
        };
        assert_eq!(resolve_vars("1px solid var(--a)", &lookup), "1px solid #fff");
        assert_eq!(resolve_vars("var(--missing, rgba(1, 2, 3, 0.5))", &lookup), "rgba(1, 2, 3, 0.5)");
    }
}
