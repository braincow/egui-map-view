//! A layer for placing polygons on the map.
//!
//! # Example
//!
//! ```no_run
//! use eframe::egui;
//! use egui_map_view::{Map, config::OpenStreetMapConfig, layers::{area::{Area, AreaLayer, AreaMode}, Layer}, projection::GeoPos};
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
//!         points: vec![
//!             GeoPos { lon: 10.0, lat: 55.0 },
//!             GeoPos { lon: 11.0, lat: 55.0 },
//!             GeoPos { lon: 10.5, lat: 55.5 },
//!         ]
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
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default().show(ctx, |ui| {
//!             ui.add(&mut self.map);
//!         });
//!     }
//! }
//! ```

use crate::layers::Layer;
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Painter, Pos2, Response, Shape, Stroke};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// The mode of the `AreaLayer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaMode {
    /// The layer is not interactive.
    #[default]
    Disabled,
    /// The user can add/remove/move nodes.
    Modify,
}

/// A polygon area on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Area {
    /// The nodes of the polygon. Must be 3 or more.
    pub points: Vec<GeoPos>,
}

/// Layer implementation that allows the user to draw polygons on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaLayer {
    areas: Vec<Area>,

    #[serde(skip)]
    /// The stroke style for drawing the polygon outlines.
    pub stroke: Stroke,

    #[serde(skip)]
    /// The fill color of the polygon.
    pub fill: Color32,

    #[serde(skip)]
    /// The radius of the nodes.
    pub node_radius: f32,

    #[serde(skip)]
    /// The fill color of the nodes.
    pub node_fill: Color32,

    #[serde(skip)]
    /// The current drawing mode.
    pub mode: AreaMode,

    #[serde(skip)]
    dragged_node: Option<(usize, usize)>, // (area_index, node_index)
}

impl Default for AreaLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AreaLayer {
    /// Creates a new `AreaLayer`.
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            stroke: Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
            fill: Color32::from_rgba_unmultiplied(255, 0, 0, 50),
            node_radius: 5.0,
            node_fill: Color32::from_rgb(0, 128, 0),
            mode: AreaMode::default(),
            dragged_node: None,
        }
    }

    /// Adds a new area to the layer.
    pub fn add_area(&mut self, area: Area) {
        if area.points.len() >= 3 {
            self.areas.push(area);
        }
    }

    fn handle_modify_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.drag_started() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.dragged_node = self.find_node_at(pointer_pos, projection);
            }
        }

        if response.dragged() {
            if let Some((area_idx, node_idx)) = self.dragged_node {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    if let Some(area) = self.areas.get_mut(area_idx) {
                        if let Some(node) = area.points.get_mut(node_idx) {
                            *node = projection.unproject(pointer_pos);
                        }
                    }
                }
            }
        }

        if response.drag_stopped() {
            self.dragged_node = None;
        }

        if self.dragged_node.is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if let Some(pointer_pos) = response.hover_pos() {
            if self.find_node_at(pointer_pos, projection).is_some() {
                response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        if response.clicked() {
            // TODO: Add/remove nodes
        }

        response.hovered()
    }

    fn find_node_at(&self, screen_pos: Pos2, projection: &MapProjection) -> Option<(usize, usize)> {
        for (area_idx, area) in self.areas.iter().enumerate().rev() {
            for (node_idx, node) in area.points.iter().enumerate() {
                let node_screen_pos = projection.project(*node);
                if node_screen_pos.distance(screen_pos) < self.node_radius * 2.0 {
                    return Some((area_idx, node_idx));
                }
            }
        }
        None
    }
}

impl Layer for AreaLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        match self.mode {
            AreaMode::Disabled => false,
            AreaMode::Modify => self.handle_modify_input(response, projection),
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for area in &self.areas {
            if area.points.len() < 2 {
                continue;
            }

            let screen_points: Vec<Pos2> =
                area.points.iter().map(|p| projection.project(*p)).collect();

            // Draw polygon outline
            if area.points.len() >= 3 {
                painter.add(Shape::convex_polygon(
                    screen_points.clone(),
                    self.fill,
                    self.stroke,
                ));
            } else {
                // Just a line if less than 3 points
                painter.add(Shape::line(screen_points.clone(), self.stroke));
            }

            // Draw nodes
            for point in &screen_points {
                painter.circle_filled(*point, self.node_radius, self.node_fill);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_layer_new() {
        let layer = AreaLayer::default();
        assert_eq!(layer.mode, AreaMode::Disabled);
        assert!(layer.areas.is_empty());
    }
}
