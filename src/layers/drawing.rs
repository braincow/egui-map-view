//! A layer for freeform drawing on the map.
//!
//! # Example
//!
//! ```no_run
//! use eframe::egui;
//! use egui_map_view::{layers::drawing::DrawingLayer, layers::drawing::DrawMode, Map, config::OpenStreetMapConfig};
//!
//! struct MyApp {
//!     map: Map,
//! }
//!
//! impl Default for MyApp {
//!   fn default() -> Self {
//!     let mut map = Map::new(OpenStreetMapConfig::default());
//!      map.add_layer("drawing", DrawingLayer::default());
//!      if let Some(drawing_layer) = map.layer_mut::<DrawingLayer>("drawing") {
//!        drawing_layer.draw_mode = DrawMode::Draw;
//!      }
//!      Self { map }
//!    }
//! }
//!
//! impl eframe::App for MyApp {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             ui.add(&mut self.map);
//!         });
//!     }
//! }
//! ```
use crate::layers::Layer;
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Painter, Pos2, Response, Stroke};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// The mode of the `DrawingLayer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawMode {
    /// The layer is not interactive.
    #[default]
    Disabled,
    /// The user can draw on the map.
    Draw,
    /// The user can erase drawings.
    Erase,
}

/// Layer implementation that allows the user to draw polylines on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawingLayer {
    polylines: Vec<Vec<GeoPos>>,

    #[serde(skip)]
    /// The stroke style for drawing aka line width and color.
    pub stroke: Stroke,

    #[serde(skip)]
    /// The current drawing mode.
    pub draw_mode: DrawMode,
}

impl DrawingLayer {
    /// Creates a new `DrawingLayer`.
    pub fn new(stroke: Stroke) -> Self {
        Self {
            polylines: Vec::new(),
            stroke,
            draw_mode: DrawMode::default(),
        }
    }
}

impl Default for DrawingLayer {
    fn default() -> Self {
        Self {
            polylines: Vec::new(),
            stroke: Stroke::new(2.0, Color32::RED),
            draw_mode: DrawMode::default(),
        }
    }
}

impl DrawingLayer {
    fn handle_draw_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.hovered() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let geo_pos = projection.unproject(pointer_pos);
                if let Some(last_line) = self.polylines.last_mut()
                    && response.ctx.input(|i| i.modifiers.shift)
                {
                    last_line.push(geo_pos);
                } else {
                    // No polylines exist yet, so create a new one.
                    let geo_pos2 = projection.unproject(pointer_pos + egui::vec2(1.0, 0.0));
                    self.polylines.push(vec![geo_pos, geo_pos2]);
                }
            }
        }

        if response.drag_started() {
            self.polylines.push(Vec::new());
        }

        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(last_line) = self.polylines.last_mut() {
                    let geo_pos = projection.unproject(pointer_pos);
                    last_line.push(geo_pos);
                }
            }
        }

        // When drawing, we consume all interactions over the map,
        // so that the map does not pan or zoom.
        response.hovered()
    }

    fn handle_erase_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.hovered() {
            response.ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
        }

        if response.dragged() || response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.erase_at(pointer_pos, projection);
            }
        }
        response.hovered()
    }

    fn erase_at(&mut self, pointer_pos: Pos2, projection: &MapProjection) {
        let erase_radius_screen = self.stroke.width;
        let erase_radius_sq = erase_radius_screen * erase_radius_screen;

        let old_polylines = std::mem::take(&mut self.polylines);
        self.polylines = old_polylines
            .into_iter()
            .flat_map(|polyline| {
                split_polyline_by_erase_circle(&polyline, pointer_pos, erase_radius_sq, projection)
            })
            .collect();
    }
}

impl Layer for DrawingLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        match self.draw_mode {
            DrawMode::Disabled => false,
            DrawMode::Draw => self.handle_draw_input(response, projection),
            DrawMode::Erase => self.handle_erase_input(response, projection),
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for polyline in &self.polylines {
            if polyline.len() > 1 {
                let screen_points: Vec<egui::Pos2> =
                    polyline.iter().map(|p| projection.project(*p)).collect();
                painter.add(egui::Shape::line(screen_points, self.stroke));
            }
        }
    }
}

/// Splits a polyline into multiple polylines based on whether segments are within the erase radius.
fn split_polyline_by_erase_circle(
    polyline: &[GeoPos],
    pointer_pos: Pos2,
    erase_radius_sq: f32,
    projection: &MapProjection,
) -> Vec<Vec<GeoPos>> {
    if polyline.len() < 2 {
        return vec![];
    }

    let screen_points: Vec<Pos2> = polyline.iter().map(|p| projection.project(*p)).collect();

    let mut new_polylines = Vec::new();
    let mut current_line = Vec::new();
    let mut in_visible_part = true;

    // Check if the first segment is erased to correctly set initial state.
    if dist_sq_to_segment(pointer_pos, screen_points[0], screen_points[1]) < erase_radius_sq {
        in_visible_part = false;
    } else {
        current_line.push(polyline[0]);
    }

    for i in 0..(polyline.len() - 1) {
        let p2_geo = polyline[i + 1];
        let p1_screen = screen_points[i];
        let p2_screen = screen_points[i + 1];

        let segment_is_erased =
            dist_sq_to_segment(pointer_pos, p1_screen, p2_screen) < erase_radius_sq;

        if in_visible_part {
            if segment_is_erased {
                // Transition from visible to erased.
                let t = projection_factor(pointer_pos, p1_screen, p2_screen);
                let split_point_screen = p1_screen.lerp(p2_screen, t);
                let split_point_geo = projection.unproject(split_point_screen);
                current_line.push(split_point_geo);

                if current_line.len() > 1 {
                    new_polylines.push(std::mem::take(&mut current_line));
                }
                in_visible_part = false;
            } else {
                // Continue visible part.
                current_line.push(p2_geo);
            }
        } else {
            // In erased part
            if !segment_is_erased {
                // Transition from erased to visible.
                let t = projection_factor(pointer_pos, p1_screen, p2_screen);
                let split_point_screen = p1_screen.lerp(p2_screen, t);
                let split_point_geo = projection.unproject(split_point_screen);

                // Start new line.
                current_line.push(split_point_geo);
                current_line.push(p2_geo);
                in_visible_part = true;
            }
            // Continue in erased part, do nothing.
        }
    }

    if current_line.len() > 1 {
        new_polylines.push(current_line);
    }

    new_polylines
}

/// Calculates the squared distance from a point to a line segment.
fn dist_sq_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let l2 = ab.length_sq();

    if l2 == 0.0 {
        // The segment is a point.
        return ap.length_sq();
    }

    // Project point p onto the line defined by a and b.
    // `t` is the normalized distance from a to the projection.
    let t = (ap.dot(ab) / l2).clamp(0.0, 1.0);

    // The closest point on the line segment.
    let closest_point = a + t * ab;

    p.distance_sq(closest_point)
}

/// Calculates the projection factor of a point onto a line segment.
/// Returns a value `t` from 0.0 to 1.0.
fn projection_factor(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let l2 = ab.length_sq();

    if l2 == 0.0 {
        return 0.0;
    }

    // Project point p onto the line defined by a and b.
    (ap.dot(ab) / l2).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_layer_new() {
        let layer = DrawingLayer::default();
        assert_eq!(layer.draw_mode, DrawMode::Disabled);
        assert!(layer.polylines.is_empty());
    }

    #[test]
    fn drawing_layer_as_any() {
        let layer = DrawingLayer::default();
        assert!(layer.as_any().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_as_any_mut() {
        let mut layer = DrawingLayer::default();
        assert!(layer.as_any_mut().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_serde() {
        let mut layer = DrawingLayer::default();
        layer.draw_mode = DrawMode::Draw; // This should not be serialized.
        layer.polylines.push(vec![
            GeoPos { lon: 1.0, lat: 2.0 },
            GeoPos { lon: 3.0, lat: 4.0 },
        ]);
        layer.stroke = Stroke::new(5.0, Color32::BLUE); // This should not be serialized.

        let json = serde_json::to_string(&layer).unwrap();

        // The serialized string should only contain polylines.
        assert!(json.contains(r#""polylines":[[{"lon":1.0,"lat":2.0},{"lon":3.0,"lat":4.0}]]"#));
        assert!(!json.contains("draw_mode"));
        assert!(!json.contains("stroke"));

        let deserialized: DrawingLayer = serde_json::from_str(&json).unwrap();

        // Check that polylines are restored correctly.
        assert_eq!(deserialized.polylines, layer.polylines);

        // Check that skipped fields have their values from the `default()` implementation,
        // not from the original `layer` object.
        assert_eq!(deserialized.draw_mode, DrawMode::Disabled);
        assert_eq!(deserialized.stroke.width, 2.0);
        assert_eq!(deserialized.stroke.color, Color32::RED);
    }
}
