use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use egui::{Color32, Rect, Ui};

pub use crate::icon_defs::*;

type Key = (&'static str, u32, bool);
type Source = (String, Arc<[u8]>);
type Cache = Mutex<HashMap<Key, Source>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Clone, Copy, Debug)]
pub struct Icon {
    pub def: IconDef,
    pub color: Color32,
    pub stroke_width: f32,
    pub round: bool,
}

impl Icon {
    pub fn new(def: IconDef, color: Color32) -> Self {
        Self { def, color, stroke_width: 2.0, round: true }
    }

    pub fn stroke(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn butt(mut self) -> Self {
        self.round = false;
        self
    }

    fn source(&self) -> Source {
        let key: Key = (self.def.name, (self.stroke_width * 100.0) as u32, self.round);
        let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(key)
            .or_insert_with(|| {
                let [w, h] = self.def.view;
                let attrs = if self.def.stroked {
                    let caps = if self.round { r#" stroke-linecap="round" stroke-linejoin="round""# } else { "" };
                    format!(r##" fill="none" stroke="#fff" stroke-width="{}"{}"##, self.stroke_width, caps)
                } else {
                    String::new()
                };
                let svg = format!(
                    r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"{attrs}>{}</svg>"#,
                    self.def.body
                );
                let uri = format!("bytes://icon/{}-{}-{}.svg", key.0, key.1, key.2);
                (uri, Arc::from(svg.into_bytes()))
            })
            .clone()
    }

    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let (uri, bytes) = self.source();
        egui::Image::from_bytes(uri, egui::load::Bytes::Shared(bytes))
            .tint(self.color)
            .paint_at(ui, rect);
    }

    pub fn paint_rotated(&self, ui: &Ui, rect: Rect, angle: f32) {
        let (uri, bytes) = self.source();
        egui::Image::from_bytes(uri, egui::load::Bytes::Shared(bytes))
            .tint(self.color)
            .rotate(angle, egui::Vec2::splat(0.5))
            .paint_at(ui, rect);
    }
}

pub fn by_name(name: &str) -> Option<IconDef> {
    const ALL: &[IconDef] = &[WIN_MINIMIZE, WIN_MAXIMIZE, WIN_CLOSE, DOWNLOAD, SETTINGS, CLOCK, LIST, BUG, MOON, ALERT_TRIANGLE, FOLDER, SEARCH, WRENCH, UPLOAD, FILE, FOLDER_OPEN, SPARKLE, CHEVRON_DOWN, HARD_DRIVE, STAR, X, AWARD, GAMEPAD_TAB, SLIDERS, SAVE, USER, USERS, BOX, COPY, POWER, CHEVRON_RIGHT, TRASH_MODAL, BAR_CHART, CHECK_CIRCLE, SHIELD_OFF, POINTER, GLOBE, GAMEPAD, SHIELD, STAR_UPDATE, GLOBE_LANG, PLAY_MODAL, REFRESH, ARROW_UP_CIRCLE, TRASH, CHECK, SUN, SETTINGS_ALT, PLAY, SHIELD_CHECK, FLAG_GB, FLAG_DE, APP_SMALL];
    ALL.iter().find(|def| def.name == name).copied()
}
