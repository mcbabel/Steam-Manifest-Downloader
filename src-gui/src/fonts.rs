use std::sync::{Arc, OnceLock, RwLock};

use egui::{FontData, FontDefinitions, FontFamily};

const INTER_LATIN: &[u8] = include_bytes!("../assets/fonts/inter-latin.ttf");
const INTER_LATIN_EXT: &[u8] = include_bytes!("../assets/fonts/inter-latin-ext.ttf");
const INTER_FULL: &[u8] = include_bytes!("../assets/fonts/inter-full.ttf");
const SYMBOLS: &[u8] = include_bytes!("../assets/fonts/symbols.ttf");
const SHAPES: &[u8] = include_bytes!("../assets/fonts/shapes.ttf");

const MONOSPACE_NAMES: [&str; 3] = ["Cascadia Code", "Fira Code", "Consolas"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub ascent: f32,
    pub descent: f32,
}

pub const INTER: Metrics = Metrics { ascent: 0.96875, descent: 0.2412 };
const HACK: Metrics = Metrics { ascent: 0.9282, descent: 0.2358 };

fn mono_metrics() -> &'static RwLock<Metrics> {
    static METRICS: OnceLock<RwLock<Metrics>> = OnceLock::new();
    METRICS.get_or_init(|| RwLock::new(HACK))
}

pub fn monospace_metrics() -> Metrics {
    *mono_metrics().read().unwrap_or_else(|e| e.into_inner())
}

struct SystemFont {
    data: Vec<u8>,
    index: u32,
    metrics: Metrics,
}

fn table(data: &[u8], index: u32, tag: &[u8; 4]) -> Option<usize> {
    let u32_at = |at: usize| data.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize);
    let u16_at = |at: usize| data.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let start = if data.get(0..4)? == b"ttcf" { u32_at(12 + 4 * index as usize)? } else { 0 };
    let count = u16_at(start + 4)?;
    (0..count).map(|i| start + 12 + 16 * i).find(|record| data.get(*record..record + 4) == Some(tag.as_slice())).and_then(|record| u32_at(record + 8))
}

fn read_metrics(data: &[u8], index: u32) -> Option<Metrics> {
    let i16_at = |at: usize| data.get(at..at + 2).map(|b| i16::from_be_bytes([b[0], b[1]]) as f32);
    let u16_at = |at: usize| data.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as f32);
    let head = table(data, index, b"head")?;
    let hhea = table(data, index, b"hhea")?;
    let units = u16_at(head + 18)?;
    if units <= 0.0 {
        return None;
    }
    Some(Metrics { ascent: i16_at(hhea + 4)? / units, descent: -i16_at(hhea + 6)? / units })
}

fn system_monospace() -> Option<SystemFont> {
    use fontique::{Collection, CollectionOptions, FontStyle, FontWeight, FontWidth, GenericFamily, SourceCache};
    let mut collection = Collection::new(CollectionOptions { shared: false, system_fonts: true });
    let mut cache = SourceCache::default();
    let mut families = Vec::new();
    for name in MONOSPACE_NAMES {
        if let Some(family) = collection.family_by_name(name) {
            families.push(family);
        }
    }
    let generic: Vec<_> = collection.generic_families(GenericFamily::Monospace).collect();
    for id in generic {
        if let Some(family) = collection.family(id) {
            families.push(family);
        }
    }
    for family in families {
        let font = family.match_font(FontWidth::NORMAL, FontStyle::Normal, FontWeight::NORMAL, false).or_else(|| family.default_font());
        let Some(font) = font else { continue };
        let Some(blob) = font.load(Some(&mut cache)) else { continue };
        let data = blob.data().to_vec();
        let index = font.index();
        if let Some(metrics) = read_metrics(&data, index) {
            return Some(SystemFont { data, index, metrics });
        }
    }
    None
}

pub fn text_rendering(ctx: &egui::Context, light: bool) {
    use egui::epaint::FontColorTransferFunction as Transfer;
    let wanted = if light { Transfer::Off } else { Transfer::Gamma(0.93) };
    if ctx.global_style().visuals.text_options.color_transfer_function != wanted {
        ctx.all_styles_mut(|style| style.visuals.text_options.color_transfer_function = wanted);
    }
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("inter-latin".into(), Arc::new(FontData::from_static(INTER_LATIN)));
    fonts.font_data.insert("inter-latin-ext".into(), Arc::new(FontData::from_static(INTER_LATIN_EXT)));
    fonts.font_data.insert("inter-full".into(), Arc::new(FontData::from_static(INTER_FULL)));
    fonts.font_data.insert("symbols".into(), Arc::new(FontData::from_static(SYMBOLS)));
    fonts.font_data.insert("shapes".into(), Arc::new(FontData::from_static(SHAPES)));
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "symbols".into());
    proportional.insert(0, "inter-full".into());
    proportional.insert(0, "shapes".into());
    proportional.insert(0, "inter-latin-ext".into());
    proportional.insert(0, "inter-latin".into());
    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    if let Some(system) = system_monospace() {
        *mono_metrics().write().unwrap_or_else(|e| e.into_inner()) = system.metrics;
        let mut data = FontData::from_owned(system.data);
        data.index = system.index;
        fonts.font_data.insert("system-monospace".into(), Arc::new(data));
        monospace.insert(0, "system-monospace".into());
    }
    monospace.push("inter-latin".into());
    monospace.push("inter-latin-ext".into());
    monospace.push("shapes".into());
    monospace.push("inter-full".into());
    monospace.push("symbols".into());
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ascent_and_descent_of_a_font_file() {
        let metrics = read_metrics(INTER_LATIN, 0).unwrap();
        assert!((metrics.ascent - INTER.ascent).abs() < 0.0001);
        assert!((metrics.descent - INTER.descent).abs() < 0.0001);
    }
}
