use std::sync::Arc;

use egui::epaint::{Mesh, Vertex};
use egui::{vec2, Color32, Galley, Painter, Pos2, Rect, Shape};

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    a.lerp_to_gamma(b, t.clamp(0.0, 1.0))
}

#[derive(Clone, Copy, Debug)]
pub struct Gradient {
    pub from: Color32,
    pub to: Color32,
}

impl Gradient {
    pub fn at(&self, rect: Rect, p: Pos2) -> Color32 {
        let span = rect.width() + rect.height();
        let t = if span > 0.0 { ((p.x - rect.left()) + (p.y - rect.top())) / span } else { 0.0 };
        lerp_color(self.from, self.to, t)
    }
}

pub fn gradient_text(painter: &Painter, pos: Pos2, galley: &Arc<Galley>, gradient: Gradient) {
    let size = painter.fonts(|f| f.font_image_size());
    let normalizer = vec2(1.0 / size[0] as f32, 1.0 / size[1] as f32);
    let bounds = Rect::from_min_size(pos, galley.size());
    let mut mesh = Mesh::default();
    for row in &galley.rows {
        let offset = pos.to_vec2() + row.pos.to_vec2();
        let base = mesh.vertices.len() as u32;
        mesh.indices.extend(row.visuals.mesh.indices.iter().map(|i| i + base));
        mesh.vertices.extend(row.visuals.mesh.vertices.iter().map(|v| {
            let p = v.pos + offset;
            Vertex { pos: p, uv: (v.uv.to_vec2() * normalizer).to_pos2(), color: gradient.at(bounds, p) }
        }));
    }
    painter.add(Shape::mesh(mesh));
}
