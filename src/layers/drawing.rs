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
//!         map.add_layer("drawing1", DrawingLayer::new());
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
use egui::{Color32, Painter, Response, Stroke};
use serde::{Deserialize, Serialize};
use std::any::Any;

use crate::layers::Layer;
use crate::projection::MapProjection;

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
    polylines: Vec<Vec<(f64, f64)>>,

    #[serde(skip)]
    stroke: Stroke,

    #[serde(skip)]
    /// The current drawing mode.
    pub draw_mode: DrawMode,
}

impl DrawingLayer {
    /// Creates a new `DrawingLayer`.
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            stroke: Stroke::new(2.0, Color32::RED),
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
            DrawMode::Erase => {
                if response.hovered() {
                    response.ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
                }

                if response.dragged() {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        let erase_radius_screen = 10.0;
                        let erase_radius_sq = erase_radius_screen * erase_radius_screen;

                        let mut new_polylines = Vec::new();
                        let old_polylines = std::mem::take(&mut self.polylines);

                        for polyline in old_polylines {
                            let mut segment = Vec::new();
                            for point_geo in polyline {
                                let point_screen = projection.project(point_geo);
                                if point_screen.distance_sq(pointer_pos) < erase_radius_sq {
                                    // Point is inside erase radius, finish the current segment.
                                    if segment.len() > 1 {
                                        new_polylines.push(segment);
                                    }
                                    segment = Vec::new();
                                } else {
                                    // Point is outside, add to current segment.
                                    segment.push(point_geo);
                                }
                            }
                            if segment.len() > 1 {
                                new_polylines.push(segment);
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
                let screen_points: Vec<egui::Pos2> =
                    polyline.iter().map(|p| projection.project(*p)).collect();
                painter.add(egui::Shape::line(screen_points, self.stroke));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_layer_new() {
        let layer = DrawingLayer::new();
        assert_eq!(layer.draw_mode, DrawMode::Disabled);
        assert!(layer.polylines.is_empty());
    }

    #[test]
    fn drawing_layer_as_any() {
        let layer = DrawingLayer::new();
        assert!(layer.as_any().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_as_any_mut() {
        let mut layer = DrawingLayer::new();
        assert!(layer.as_any_mut().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_serde() {
        let mut layer = DrawingLayer::new();
        layer.draw_mode = DrawMode::Draw; // This should not be serialized.
        layer.polylines.push(vec![(1.0, 2.0), (3.0, 4.0)]);
        layer.stroke = Stroke::new(5.0, Color32::BLUE); // This should not be serialized.

        let json = serde_json::to_string(&layer).unwrap();

        // The serialized string should only contain polylines.
        assert!(json.contains(r#""polylines":[[[1.0,2.0],[3.0,4.0]]]"#));
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
