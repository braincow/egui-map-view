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
//!         ],
//!         stroke: Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
//!         fill: Color32::from_rgba_unmultiplied(255, 0, 0, 50),
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

use crate::layers::{Layer, dist_sq_to_segment, projection_factor, segments_intersect};
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Mesh, Painter, Pos2, Response, Shape, Stroke};
use log::warn;
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

/// The shape of a polygon area on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AreaShape {
    /// A freeform polygon defined by a list of points.
    Polygon(Vec<GeoPos>),
    /// A circle defined by its center and radius in meters.
    Circle {
        /// The geographical center of the circle.
        center: GeoPos,
        /// The radius of the circle in meters.
        radius: f64,
    },
}

/// A polygon area on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Area {
    /// The shape of the area.
    pub shape: AreaShape,
    #[serde(skip)]
    /// The stroke style for drawing the polygon outlines.
    pub stroke: Stroke,
    #[serde(skip)]
    /// The fill color of the polygon.
    pub fill: Color32,
}

/// Represents what part of an area is being dragged.
#[derive(Clone, Debug)]
enum DraggedObject {
    PolygonNode {
        area_index: usize,
        node_index: usize,
    },
    CircleCenter {
        area_index: usize,
    },
    CircleRadius {
        area_index: usize,
    },
}

/// Layer implementation that allows the user to draw polygons on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaLayer {
    areas: Vec<Area>,

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
    dragged_object: Option<DraggedObject>,
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
            node_radius: 5.0,
            node_fill: Color32::from_rgb(0, 128, 0),
            mode: AreaMode::default(),
            dragged_object: None,
        }
    }

    /// Adds a new area to the layer.
    pub fn add_area(&mut self, area: Area) {
        self.areas.push(area);
    }

    fn handle_modify_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.double_clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                // TODO: This only works for polygons.
                if self.find_node_at(pointer_pos, projection).is_none() {
                    if let Some((area_idx, node_idx)) =
                        self.find_line_segment_at(pointer_pos, projection)
                    {
                        if let Some(area) = self.areas.get_mut(area_idx) {
                            if let AreaShape::Polygon(points) = &mut area.shape {
                                let p1_screen = projection.project(points[node_idx]);
                                let p2_screen =
                                    projection.project(points[(node_idx + 1) % points.len()]);

                                let t = projection_factor(pointer_pos, p1_screen, p2_screen);

                                // Interpolate in screen space and unproject to get the new geographical position.
                                let new_pos_screen = p1_screen.lerp(p2_screen, t);
                                let new_pos_geo = projection.unproject(new_pos_screen);

                                points.insert(node_idx + 1, new_pos_geo);

                                // This interaction is fully handled, so we can return.
                                return response.hovered();
                            }
                        }
                    }
                }
            }
        }

        if response.drag_started() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.dragged_object = self.find_object_at(pointer_pos, projection);
            }
        }

        if response.dragged() {
            if let Some(dragged_object) = &self.dragged_object {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    match dragged_object {
                        DraggedObject::PolygonNode {
                            area_index,
                            node_index,
                        } => {
                            if self.is_move_valid(*area_index, *node_index, pointer_pos, projection)
                            {
                                if let Some(area) = self.areas.get_mut(*area_index) {
                                    if let AreaShape::Polygon(points) = &mut area.shape {
                                        if let Some(node) = points.get_mut(*node_index) {
                                            *node = projection.unproject(pointer_pos);
                                        }
                                    }
                                }
                            }
                        }
                        DraggedObject::CircleCenter { area_index } => {
                            if let Some(area) = self.areas.get_mut(*area_index) {
                                if let AreaShape::Circle { center, .. } = &mut area.shape {
                                    *center = projection.unproject(pointer_pos);
                                }
                            }
                        }
                        DraggedObject::CircleRadius { area_index } => {
                            if let Some(area) = self.areas.get_mut(*area_index) {
                                if let AreaShape::Circle { center, radius } = &mut area.shape {
                                    // Convert the new screen-space radius back to meters.
                                    let center_screen = projection.project(*center);
                                    let new_radius_pixels = pointer_pos.distance(center_screen);
                                    let new_edge_screen =
                                        center_screen + egui::vec2(new_radius_pixels, 0.0);
                                    let new_edge_geo = projection.unproject(new_edge_screen);

                                    // Calculate distance in meters. This is an approximation that works well for smaller distances.
                                    let distance_lon = (new_edge_geo.lon - center.lon).abs()
                                        * (111_320.0 * center.lat.to_radians().cos());
                                    let distance_lat =
                                        (new_edge_geo.lat - center.lat).abs() * 110_574.0;
                                    *radius = (distance_lon.powi(2) + distance_lat.powi(2)).sqrt();
                                }
                            }
                        }
                    }
                }
            }
        }

        if response.drag_stopped() {
            self.dragged_object = None;
        }

        let is_dragging = self.dragged_object.is_some();

        if is_dragging {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if let Some(pointer_pos) = response.hover_pos() {
            if self.find_object_at(pointer_pos, projection).is_some() {
                response.ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        is_dragging || response.hovered()
    }

    fn find_object_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
    ) -> Option<DraggedObject> {
        let click_tolerance_sq = (self.node_radius * 3.0).powi(2);

        for (area_idx, area) in self.areas.iter().enumerate().rev() {
            match &area.shape {
                AreaShape::Polygon(points) => {
                    for (node_idx, node) in points.iter().enumerate() {
                        let node_screen_pos = projection.project(*node);
                        if node_screen_pos.distance_sq(screen_pos) < click_tolerance_sq {
                            return Some(DraggedObject::PolygonNode {
                                area_index: area_idx,
                                node_index: node_idx,
                            });
                        }
                    }
                }
                AreaShape::Circle { center, radius } => {
                    let center_screen = projection.project(*center);

                    // Convert radius from meters to screen pixels to correctly detect handle clicks.
                    let point_on_circle_geo = GeoPos {
                        lon: center.lon + (radius / (111_320.0 * center.lat.to_radians().cos())),
                        lat: center.lat,
                    };
                    let point_on_circle_screen = projection.project(point_on_circle_geo);
                    let radius_pixels = center_screen.distance(point_on_circle_screen);

                    // Check for radius handle
                    let distance_to_edge =
                        (center_screen.distance(screen_pos) - radius_pixels).abs();
                    if distance_to_edge < self.node_radius * 2.0 {
                        return Some(DraggedObject::CircleRadius {
                            area_index: area_idx,
                        });
                    }

                    // Check for center
                    if center_screen.distance_sq(screen_pos) < click_tolerance_sq {
                        return Some(DraggedObject::CircleCenter {
                            area_index: area_idx,
                        });
                    }
                }
            }
        }

        None
    }

    fn find_node_at(&self, screen_pos: Pos2, projection: &MapProjection) -> Option<(usize, usize)> {
        // This function is now a subset of find_object_at, kept for double-click to add node.
        // It probably should be refactored.
        self.find_line_segment_at(screen_pos, projection)
    }
    fn find_line_segment_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
    ) -> Option<(usize, usize)> {
        let click_tolerance = (self.node_radius * 2.0).powi(2);

        for (area_idx, area) in self.areas.iter().enumerate().rev() {
            if let AreaShape::Polygon(points) = &area.shape {
                if points.len() < 2 {
                    continue;
                }
                for i in 0..points.len() {
                    let p1 = projection.project(points[i]);
                    let p2 = projection.project(points[(i + 1) % points.len()]);

                    if dist_sq_to_segment(screen_pos, p1, p2) < click_tolerance {
                        return Some((area_idx, i));
                    }
                }
            }
        }
        None
    }

    /// Checks if moving a node to a new position would cause the polygon to self-intersect.
    fn is_move_valid(
        &self,
        area_idx: usize,
        node_idx: usize,
        new_screen_pos: Pos2,
        projection: &MapProjection,
    ) -> bool {
        let area = if let Some(area) = self.areas.get(area_idx) {
            area
        } else {
            return false; // Should not happen
        };

        let points = match &area.shape {
            AreaShape::Polygon(points) => points,
            _ => return true, // Not a polygon, no intersections possible.
        };

        if points.len() < 3 {
            return true;
        }
        let screen_points: Vec<Pos2> = points.iter().map(|p| projection.project(*p)).collect();

        let n = screen_points.len();
        let prev_node_idx = (node_idx + n - 1) % n;
        let next_node_idx = (node_idx + 1) % n;

        // The two edges that are being modified by the drag.
        let new_edge1 = (screen_points[prev_node_idx], new_screen_pos);
        let new_edge2 = (new_screen_pos, screen_points[next_node_idx]);

        for i in 0..n {
            let p1_idx = i;
            let p2_idx = (i + 1) % n;

            // Don't check against the edges connected to the dragged node.
            if p1_idx == node_idx || p2_idx == node_idx {
                continue;
            }

            let edge_to_check = (screen_points[p1_idx], screen_points[p2_idx]);

            // Check against the first new edge.
            if p1_idx != prev_node_idx && p2_idx != prev_node_idx {
                if segments_intersect(new_edge1.0, new_edge1.1, edge_to_check.0, edge_to_check.1) {
                    return false;
                }
            }

            // Check against the second new edge.
            if p1_idx != next_node_idx && p2_idx != next_node_idx {
                if segments_intersect(new_edge2.0, new_edge2.1, edge_to_check.0, edge_to_check.1) {
                    return false;
                }
            }
        }

        true
    }
}

impl Area {
    /// Returns the points of the area. For a circle, it generates a polygon approximation.
    fn get_points(&self, projection: &MapProjection) -> Vec<GeoPos> {
        match &self.shape {
            AreaShape::Polygon(points) => points.clone(),
            AreaShape::Circle { center, radius } => {
                let num_points = 64;
                let mut circle_points = Vec::with_capacity(num_points);

                // Convert radius from meters to screen pixels.
                let center_geo = *center;
                let point_on_circle_geo = GeoPos {
                    lon: center_geo.lon
                        + (radius / (111_320.0 * center_geo.lat.to_radians().cos())),
                    lat: center_geo.lat,
                };
                let center_screen = projection.project(center_geo);
                let point_on_circle_screen = projection.project(point_on_circle_geo);
                let radius_pixels = center_screen.distance(point_on_circle_screen);

                for i in 0..num_points {
                    let angle = (i as f64 / num_points as f64) * 2.0 * std::f64::consts::PI;
                    let point_screen = center_screen
                        + egui::vec2(
                            radius_pixels * angle.cos() as f32,
                            radius_pixels * angle.sin() as f32,
                        );
                    circle_points.push(projection.unproject(point_screen));
                }
                circle_points
            }
        }
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
            let points = area.get_points(projection);
            let screen_points: Vec<Pos2> = points.iter().map(|p| projection.project(*p)).collect();

            // Draw polygon outline
            if screen_points.len() >= 3 {
                // Use a generic path for the stroke.

                let path_shape = Shape::Path(egui::epaint::PathShape {
                    points: screen_points.clone(),
                    closed: true,
                    fill: Color32::TRANSPARENT,
                    stroke: area.stroke.into(),
                });
                painter.add(path_shape);

                // Triangulate for the fill.
                let flat_points: Vec<f64> = screen_points
                    .iter()
                    .flat_map(|p| [p.x as f64, p.y as f64])
                    .collect();
                let indices = earcutr::earcut(&flat_points, &[], 2).unwrap(); // <-- TODO: FIX UNWRAP!

                let mut mesh = Mesh::default();
                mesh.vertices = screen_points
                    .iter()
                    .map(|p| egui::epaint::Vertex {
                        pos: *p,
                        uv: Default::default(),
                        color: area.fill,
                    })
                    .collect();
                mesh.indices = indices.into_iter().map(|i| i as u32).collect();
                painter.add(Shape::Mesh(mesh.into()));
            } else {
                warn!("Invalid amount of points in area. {:?}", area);
            }

            // Draw nodes only when in modify mode
            if self.mode == AreaMode::Modify {
                match &area.shape {
                    AreaShape::Polygon(_) => {
                        for point in &screen_points {
                            painter.circle_filled(*point, self.node_radius, self.node_fill);
                        }
                    }
                    AreaShape::Circle { center, radius } => {
                        let center_screen = projection.project(*center);

                        // Convert radius from meters to screen pixels to correctly position the handle.
                        let point_on_circle_geo = GeoPos {
                            lon: center.lon
                                + (radius / (111_320.0 * center.lat.to_radians().cos())),
                            lat: center.lat,
                        };
                        let point_on_circle_screen = projection.project(point_on_circle_geo);
                        let radius_pixels = center_screen.distance(point_on_circle_screen);

                        painter.circle_filled(center_screen, self.node_radius, self.node_fill);
                        let radius_handle_pos = center_screen + egui::vec2(radius_pixels, 0.0);
                        painter.circle_filled(radius_handle_pos, self.node_radius, self.node_fill);
                    }
                }
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
