//! Map layers.ccccccgcutbfutunntjgefeukfvchggtlvttjnbjjhvl
//!

use egui::{Color32, Painter, Response, Stroke};

use crate::projection::MapProjection;

/// A trait for map layers.
pub trait Layer {
    /// Handles user input for the layer.
    fn handle_input(&mut self, response: &Response, projection: &MapProjection);

    /// Draws the layer.
    fn draw(&self, painter: &Painter, projection: &MapProjection);
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
#[derive(Default)]
pub struct DrawingLayer {
    polylines: Vec<Vec<(f64, f64)>>,
    stroke: Stroke,
}

impl DrawingLayer {
    /// Creates a new `DrawingLayer`.
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            stroke: Stroke::new(2.0, Color32::RED),
        }
    }
}

impl Layer for DrawingLayer {
    fn handle_input(&mut self, response: &Response, projection: &MapProjection) {
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
