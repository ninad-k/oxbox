//! Preflattened vectors from the already-licensed Lucide icon set.
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Deserialize)]
pub struct Icons {
    icons: BTreeMap<String, Vec<Vec<[f32; 2]>>>,
}
impl Icons {
    pub fn bundled() -> Result<Self, serde_json::Error> {
        serde_json::from_str(include_str!("../../../catalog/icons.json"))
    }
    pub fn draw(&self, ui: &egui::Ui, name: &str, rect: Rect, color: Color32) {
        if let Some(paths) = self.icons.get(name) {
            let scale = rect.width().min(rect.height()) / 24.0;
            for points in paths {
                let points: Vec<_> = points
                    .iter()
                    .map(|point| {
                        Pos2::new(
                            rect.left() + point[0] * scale,
                            rect.top() + point[1] * scale,
                        )
                    })
                    .collect();
                ui.painter()
                    .add(egui::Shape::line(points, Stroke::new(1.7 * scale, color)));
            }
        }
    }
}
pub fn brand(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 6, Color32::from_rgb(23, 33, 46));
    let area = rect.shrink(size * 0.23);
    let white = Color32::from_rgb(245, 247, 250);
    let orange = Color32::from_rgb(239, 123, 62);
    painter.rect_stroke(
        area,
        5,
        Stroke::new(size * 0.07, white),
        egui::StrokeKind::Middle,
    );
    painter.line_segment(
        [area.center_top(), area.right_top()],
        Stroke::new(size * 0.085, orange),
    );
    painter.line_segment(
        [area.right_top(), area.right_center()],
        Stroke::new(size * 0.085, orange),
    );
}
pub fn pin(ui: &mut egui::Ui, favorite: bool, name: &str) -> egui::Response {
    let label = format!("{} {name}", if favorite { "Unpin" } else { "Pin" });
    let response = ui.add_sized([32.0, 32.0], egui::Button::new(""));
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), favorite, &label)
    });
    let center = response.rect.center();
    let points = (0..10)
        .map(|index| {
            let angle = std::f32::consts::TAU * index as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
            center + egui::vec2(angle.cos(), angle.sin()) * if index % 2 == 0 { 9.0 } else { 4.0 }
        })
        .collect();
    let color = if favorite {
        ui.visuals().selection.stroke.color
    } else {
        ui.visuals().text_color()
    };
    ui.painter().add(egui::Shape::closed_line(
        points,
        Stroke::new(if favorite { 2.0 } else { 1.3 }, color),
    ));
    response.on_hover_text(label)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_catalogue_tool_has_finite_bundled_vectors() {
        let icons = Icons::bundled().unwrap();
        for tool in oxbox_core::launcher::Catalog::bundled().unwrap().tools {
            let paths = icons
                .icons
                .get(&tool.icon)
                .expect("every tool icon is bundled");
            assert!(!paths.is_empty());
            assert!(paths.iter().all(|path| {
                path.len() > 1
                    && path
                        .iter()
                        .all(|point| point.iter().all(|value| value.is_finite()))
            }));
        }
    }
}
