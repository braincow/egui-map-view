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
//!     fn default() -> Self {
//!         let mut map = Map::new(OpenStreetMapConfig::default());
//!         map.add_layer("drawing1", DrawingLayer::default());
//!         if let Some(layer) = map.layers_mut().get_mut("drawing1") {
//!           if let Some(drawing_layer) =
//!             layer.as_any_mut().downcast_mut::<DrawingLayer>()
//!           {
//!               drawing_layer.draw_mode = DrawMode::Draw;
//!           }
//!         }
//!         Self { map }
//!     }
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
use egui::{Color32, Painter, Pos2, Response, Stroke};
use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::layers::Layer;
use crate::projection::{GeoPos, MapProjection};

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
            DrawMode::Draw => {
                if response.hovered() {
                    response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
                }

                if response.clicked() && response.ctx.input(|i| i.modifiers.shift) {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let geo_pos = projection.unproject(pointer_pos);
                        if let Some(last_line) = self.polylines.last_mut() {
                            last_line.push(geo_pos.into());
                        } else {
                            // No polylines exist yet, so create a new one.
                            let geo_pos2 = projection.unproject(pointer_pos + egui::vec2(1.0, 0.0));
                            self.polylines.push(vec![geo_pos.into(), geo_pos2.into()]);
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
                            last_line.push(geo_pos.into());
                        }
                    }
                }

                // When drawing, we consume all interactions over the map,
                // so that the map does not pan or zoom.
                response.hovered()
            }
            DrawMode::Erase => {
                if response.hovered() {
                    response.ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
                }

                if response.dragged() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let erase_radius_screen = 10.0;
                        let erase_radius_sq = erase_radius_screen * erase_radius_screen;

                        let old_polylines = std::mem::take(&mut self.polylines);
                        let mut new_polylines = Vec::with_capacity(old_polylines.len());

                        for polyline in old_polylines {
                            if polyline.len() < 2 {
                                continue;
                            }

                            let mut current_segment = vec![polyline[0]];

                            for window in polyline.windows(2) {
                                let p1_geo = window[0];
                                let p2_geo = window[1];

                                let p1_screen = projection.project(p1_geo.into());
                                let p2_screen = projection.project(p2_geo.into());

                                if dist_sq_to_segment(pointer_pos, p1_screen, p2_screen)
                                    < erase_radius_sq
                                {
                                    // This segment is erased. Finalize the previous segment.
                                    if current_segment.len() > 1 {
                                        new_polylines.push(current_segment);
                                    }
                                    // Start a new segment from the second point of the erased one.
                                    current_segment = vec![p2_geo];
                                } else {
                                    // This segment is not erased, extend the current one.
                                    current_segment.push(p2_geo);
                                }
                            }

                            if current_segment.len() > 1 {
                                new_polylines.push(current_segment);
                            }
                        }
                        self.polylines = new_polylines;
                    }
                }
                response.hovered()
            }
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for polyline in &self.polylines {
            if polyline.len() > 1 {
                let screen_points: Vec<egui::Pos2> = polyline
                    .iter()
                    .map(|p| projection.project((*p).into()))
                    .collect();
                painter.add(egui::Shape::line(screen_points, self.stroke));
            }
        }
    }
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
