//! A layer for placing polygons on the map.
//!
//! # Example
//!
//! ```no_run
//! use eframe::egui;
//! use egui_map_view::{Map, config::OpenStreetMapConfig, layers::{area::{Area, AreaLayer, AreaMode, AreaShape::Polygon, FillType}, Layer}, projection::GeoPos};
//! use egui::{Color32, Stroke};
//!
//! struct MyApp {
//!     map: Map,
//! }
//!
//! impl Default for MyApp {
//!   fn default() -> Self {
//!     let mut map = Map::new(OpenStreetMapConfig::default());
//!
//!     let mut area_layer = AreaLayer::default();
//!     area_layer.add_area(Area {
//!         shape: Polygon(vec![
//!             GeoPos { lon: 10.0, lat: 55.0 },
//!             GeoPos { lon: 11.0, lat: 55.0 },
//!             GeoPos { lon: 10.5, lat: 55.5 },
//!         ]),
//!         stroke: Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
//!         fill: Color32::from_rgba_unmultiplied(255, 0, 0, 50),
//!         fill_type: FillType::Solid,
//!     });
//!     area_layer.mode = AreaMode::Modify;
//!
//!     map.add_layer("areas", area_layer);
//!
//!     Self { map }
//!   }
//! }
//!
//! impl eframe::App for MyApp {
//!     fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default().show_inside(ui, |ui| {
//!             ui.add(&mut self.map);
//!         });
//!     }
//! }
//! ```

/// Hatching geometries for the map area layer.
pub mod hatching;
/// The interactive map area layer implementation.
pub mod layer;
/// Types and definitions for the area layer.
pub mod types;

#[cfg(test)]
mod tests;

pub use layer::AreaLayer;
pub use types::{Area, AreaMode, AreaShape, FillType};
