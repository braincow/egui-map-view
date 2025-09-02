//! Map layers.
//!

use egui::{Color32, Painter, Response, Stroke};
use std::any::Any;

use crate::projection::MapProjection;

/// A trait for map layers.
pub trait Layer: Any {
    /// Handles user input for the layer. Returns `true` if the input was handled and should not be
    /// processed further by the map.
    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool;

    /// Draws the layer.
    fn draw(&self, painter: &Painter, projection: &MapProjection);

    /// Gets the layer as a `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Gets the layer as a mutable `dyn Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A layer for freeform drawing on the map.
///
/// # Example
///
/// ```no_run
/// use eframe::egui;
/// use egui_map_view::{layers::DrawingLayer, Map, config::OpenStreetMapConfig};
///
/// struct MyApp {
///     map: Map,
/// }
///
/// impl Default for MyApp {
///     fn default() -> Self {
///         let mut map = Map::new(OpenStreetMapConfig::default());
///         map.add_layer(DrawingLayer::new());
///         Self { map }
///     }
/// }
///
/// impl eframe::App for MyApp {
///     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
///         egui::CentralPanel::default().show(ctx, |ui| {
///             ui.add(&mut self.map);
///         });
///     }
/// }
/// ```
#[derive(Clone, Default)]
pub struct DrawingLayer {
    polylines: Vec<Vec<(f64, f64)>>,
    stroke: Stroke,

    /// Whether the user can draw on the map.
    pub draw_enabled: bool,
}

impl DrawingLayer {
    /// Creates a new `DrawingLayer`.
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            stroke: Stroke::new(2.0, Color32::RED),
            draw_enabled: false,
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
        if !self.draw_enabled {
            return false;
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

        // If drawing is enabled, we consume all interactions over the map,
        // so that the map does not pan or zoom.
        response.hovered()
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
        assert!(!layer.draw_enabled);
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
}
