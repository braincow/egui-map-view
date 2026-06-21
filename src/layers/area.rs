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

use crate::layers::{
    Layer, default_opacity, dist_sq_to_segment, projection_factor, segments_intersect,
    serde_color32, serde_stroke,
};
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
    /// All areas and their nodes are interactive.
    Modify,
    /// Only the selected area is interactive.
    ModifySelected,
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
        /// How many points should be used to draw the circle. If None the the point count is determined automatically which might look edged depending on zoom and projection.
        points: Option<i64>,
    },
    /// An ellipse defined by its center, major and minor radii in meters, and rotation in radians.
    Ellipse {
        /// The geographical center of the ellipse.
        center: GeoPos,
        /// The semi-major axis (radius) of the ellipse in meters.
        radius_major: f64,
        /// The semi-minor axis (radius) of the ellipse in meters.
        radius_minor: f64,
        /// The rotation of the ellipse in radians, measured counter-clockwise from the East (positive X-axis).
        rotation: f64,
        /// How many points should be used to draw the ellipse. If None, the point count is determined automatically.
        points: Option<i64>,
    },
}

/// How the interior of an area is filled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillType {
    /// No fill — only the outline is drawn.
    None,
    /// Solid color fill.
    #[default]
    Solid,
    /// Diagonal hatching lines using the stroke style.
    Hatching,
}

/// A polygon area on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Area {
    /// The shape of the area.
    pub shape: AreaShape,

    /// The stroke style for drawing the polygon outlines.
    #[serde(with = "serde_stroke")]
    pub stroke: Stroke,

    /// The fill color of the polygon.
    #[serde(with = "serde_color32")]
    pub fill: Color32,

    /// How the interior of the area is filled.
    #[serde(default)]
    pub fill_type: FillType,
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
    EllipseCenter {
        area_index: usize,
    },
    EllipseMajorRadius {
        area_index: usize,
    },
    EllipseMinorRadius {
        area_index: usize,
    },
    EllipseRotation {
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

    #[serde(skip)]
    hovered_object: Option<DraggedObject>,

    /// The opacity of the layer.
    #[serde(default = "default_opacity")]
    pub opacity: f32,

    #[serde(skip)]
    /// The index of the currently selected area. Only used when in `AreaMode::Selected`.
    pub selected_area: Option<usize>,
}

impl Default for AreaLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AreaLayer {
    /// Creates a new `AreaLayer`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            node_radius: 5.0,
            node_fill: Color32::from_rgb(0, 128, 0),
            mode: AreaMode::default(),
            dragged_object: None,
            hovered_object: None,
            opacity: 1.0,
            selected_area: None,
        }
    }

    /// Adds a new area to the layer.
    pub fn add_area(&mut self, area: Area) {
        self.areas.push(area);
    }

    /// Returns a reference to the areas in the layer.
    #[must_use]
    pub fn areas(&self) -> &Vec<Area> {
        &self.areas
    }

    /// Returns a mutable reference to the areas in the layer.
    pub fn areas_mut(&mut self) -> &mut Vec<Area> {
        &mut self.areas
    }

    /// Serializes the layer to a `GeoJSON` `FeatureCollection`.
    #[cfg(feature = "geojson")]
    pub fn to_geojson_str(&self) -> Result<String, serde_json::Error> {
        let features: Vec<geojson::Feature> = self
            .areas
            .clone()
            .into_iter()
            .map(geojson::Feature::from)
            .collect();
        let mut foreign_members = serde_json::Map::new();
        foreign_members.insert(
            "opacity".to_string(),
            serde_json::Value::from(f64::from(self.opacity)),
        );

        let feature_collection = geojson::FeatureCollection {
            bbox: None,
            features,
            foreign_members: Some(foreign_members),
        };
        serde_json::to_string(&feature_collection)
    }

    /// Deserializes a `GeoJSON` `FeatureCollection` and adds the features to the layer.
    #[cfg(feature = "geojson")]
    pub fn from_geojson_str(&mut self, s: &str) -> Result<(), serde_json::Error> {
        let feature_collection: geojson::FeatureCollection = serde_json::from_str(s)?;
        let new_areas: Vec<Area> = feature_collection
            .features
            .into_iter()
            .filter_map(|f| Area::try_from(f).ok())
            .collect();
        self.areas.extend(new_areas);

        if let Some(foreign_members) = feature_collection.foreign_members
            && let Some(value) = foreign_members.get("opacity")
            && let Some(opacity) = value.as_f64()
        {
            self.opacity = opacity as f32;
        }
        Ok(())
    }

    fn handle_modify_input(
        &mut self,
        response: &Response,
        projection: &MapProjection,
        limit_to_area: Option<usize>,
    ) -> bool {
        self.hovered_object = response
            .hover_pos()
            .and_then(|pos| self.find_object_at(pos, projection, limit_to_area));

        if response.double_clicked()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            // TODO: This only works for polygons.
            if self
                .find_node_at(pointer_pos, projection, limit_to_area)
                .is_none()
                && let Some((area_idx, node_idx)) =
                    self.find_line_segment_at(pointer_pos, projection, limit_to_area)
                && let Some(area) = self.areas.get_mut(area_idx)
                && let AreaShape::Polygon(points) = &mut area.shape
            {
                let p1_screen = projection.project(points[node_idx]);
                let p2_screen = projection.project(points[(node_idx + 1) % points.len()]);

                let t = projection_factor(pointer_pos, p1_screen, p2_screen);

                // Interpolate in screen space and unproject to get the new geographical position.
                let new_pos_screen = p1_screen.lerp(p2_screen, t);
                let new_pos_geo = projection.unproject(new_pos_screen);

                points.insert(node_idx + 1, new_pos_geo);

                // This interaction is fully handled, so we can return.
                return response.hovered();
            }
        }

        if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            self.dragged_object = self.find_object_at(pointer_pos, projection, limit_to_area);
        }

        if response.dragged()
            && let Some(dragged_object) = self.dragged_object.clone()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            match dragged_object {
                DraggedObject::PolygonNode {
                    area_index,
                    node_index,
                } => {
                    if self.is_move_valid(area_index, node_index, pointer_pos, projection)
                        && let Some(area) = self.areas.get_mut(area_index)
                    {
                        let mut revert_info = None;
                        if let AreaShape::Polygon(points) = &mut area.shape
                            && let Some(node) = points.get_mut(node_index)
                        {
                            let old_pos = *node;
                            *node = projection.unproject(pointer_pos);
                            revert_info = Some(old_pos);
                        }

                        if let Some(old_pos) = revert_info
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Polygon(points) = &mut area.shape {
                                points[node_index] = old_pos;
                            }
                        }
                    }
                }
                DraggedObject::CircleCenter { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_center = None;
                        if let AreaShape::Circle { center, .. } = &mut area.shape {
                            revert_center = Some(*center);
                            *center = projection.unproject(pointer_pos);
                        }

                        if let Some(old_center) = revert_center
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Circle { center, .. } = &mut area.shape {
                                *center = old_center;
                            }
                        }
                    }
                }
                DraggedObject::CircleRadius { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_radius = None;
                        if let AreaShape::Circle {
                            center,
                            radius,
                            points: _,
                        } = &mut area.shape
                        {
                            revert_radius = Some(*radius);
                            // Convert the new screen-space radius back to meters.
                            let center_screen = projection.project(*center);
                            let new_radius_pixels = pointer_pos.distance(center_screen);
                            let new_edge_screen =
                                center_screen + egui::vec2(new_radius_pixels, 0.0);
                            let new_edge_geo = projection.unproject(new_edge_screen);

                            // Calculate distance in meters. This is an approximation that works well for smaller distances.
                            let distance_lon = (new_edge_geo.lon - center.lon).abs()
                                * (111_320.0 * center.lat.to_radians().cos().max(1e-6));
                            let distance_lat = (new_edge_geo.lat - center.lat).abs() * 110_574.0;
                            let new_val = (distance_lon.powi(2) + distance_lat.powi(2)).sqrt();
                            if new_val.is_finite() {
                                *radius = new_val;
                            }
                        }

                        if let Some(old_radius) = revert_radius
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Circle { radius, .. } = &mut area.shape {
                                *radius = old_radius;
                            }
                        }
                    }
                }
                DraggedObject::EllipseCenter { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_center = None;
                        if let AreaShape::Ellipse { center, .. } = &mut area.shape {
                            revert_center = Some(*center);
                            *center = projection.unproject(pointer_pos);
                        }

                        if let Some(old_center) = revert_center
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Ellipse { center, .. } = &mut area.shape {
                                *center = old_center;
                            }
                        }
                    }
                }
                DraggedObject::EllipseMajorRadius { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_radius_major = None;
                        if let AreaShape::Ellipse {
                            center,
                            radius_major,
                            rotation,
                            ..
                        } = &mut area.shape
                        {
                            revert_radius_major = Some(*radius_major);
                            let center_screen = projection.project(*center);
                            let cos_rot = rotation.cos() as f32;
                            let sin_rot = rotation.sin() as f32;
                            let v_major = egui::vec2(cos_rot, sin_rot);
                            let new_radius_pixels =
                                (pointer_pos - center_screen).dot(v_major).max(1.0);
                            let new_edge_screen =
                                center_screen + egui::vec2(new_radius_pixels, 0.0);
                            let new_edge_geo = projection.unproject(new_edge_screen);
                            let distance_lon = (new_edge_geo.lon - center.lon).abs()
                                * (111_320.0 * center.lat.to_radians().cos().max(1e-6));
                            let distance_lat = (new_edge_geo.lat - center.lat).abs() * 110_574.0;
                            let new_val = (distance_lon.powi(2) + distance_lat.powi(2)).sqrt();
                            if new_val.is_finite() {
                                *radius_major = new_val;
                            }
                        }

                        if let Some(old_radius) = revert_radius_major
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Ellipse { radius_major, .. } = &mut area.shape {
                                *radius_major = old_radius;
                            }
                        }
                    }
                }
                DraggedObject::EllipseMinorRadius { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_radius_minor = None;
                        if let AreaShape::Ellipse {
                            center,
                            radius_minor,
                            rotation,
                            ..
                        } = &mut area.shape
                        {
                            revert_radius_minor = Some(*radius_minor);
                            let center_screen = projection.project(*center);
                            let cos_rot = rotation.cos() as f32;
                            let sin_rot = rotation.sin() as f32;
                            let v_minor = egui::vec2(-sin_rot, cos_rot);
                            let new_radius_pixels =
                                (pointer_pos - center_screen).dot(v_minor).max(1.0);
                            let new_edge_screen =
                                center_screen + egui::vec2(new_radius_pixels, 0.0);
                            let new_edge_geo = projection.unproject(new_edge_screen);
                            let distance_lon = (new_edge_geo.lon - center.lon).abs()
                                * (111_320.0 * center.lat.to_radians().cos().max(1e-6));
                            let distance_lat = (new_edge_geo.lat - center.lat).abs() * 110_574.0;
                            let new_val = (distance_lon.powi(2) + distance_lat.powi(2)).sqrt();
                            if new_val.is_finite() {
                                *radius_minor = new_val;
                            }
                        }

                        if let Some(old_radius) = revert_radius_minor
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Ellipse { radius_minor, .. } = &mut area.shape {
                                *radius_minor = old_radius;
                            }
                        }
                    }
                }
                DraggedObject::EllipseRotation { area_index } => {
                    if let Some(area) = self.areas.get_mut(area_index) {
                        let mut revert_rotation = None;
                        if let AreaShape::Ellipse {
                            center, rotation, ..
                        } = &mut area.shape
                        {
                            revert_rotation = Some(*rotation);
                            let center_screen = projection.project(*center);
                            let new_val = f64::from(
                                (pointer_pos - center_screen)
                                    .y
                                    .atan2((pointer_pos - center_screen).x),
                            );
                            if new_val.is_finite() {
                                *rotation = new_val;
                            }
                        }

                        if let Some(old_rotation) = revert_rotation
                            && !area.can_triangulate(projection)
                        {
                            warn!("Triangulation failed, cancelling drag");
                            self.dragged_object = None;
                            if let AreaShape::Ellipse { rotation, .. } = &mut area.shape {
                                *rotation = old_rotation;
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
        let active_object = self.dragged_object.as_ref().or(self.hovered_object.as_ref());

        if let Some(obj) = active_object {
            let cursor = match obj {
                DraggedObject::PolygonNode { .. }
                | DraggedObject::CircleCenter { .. }
                | DraggedObject::EllipseCenter { .. } => {
                    if is_dragging {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Move
                    }
                }
                DraggedObject::CircleRadius { .. }
                | DraggedObject::EllipseMajorRadius { .. }
                | DraggedObject::EllipseMinorRadius { .. } => {
                    if is_dragging {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::ResizeHorizontal
                    }
                }
                DraggedObject::EllipseRotation { .. } => {
                    if is_dragging {
                        egui::CursorIcon::Grabbing
                    } else {
                        egui::CursorIcon::Crosshair
                    }
                }
            };
            response.ctx.set_cursor_icon(cursor);
        }

        is_dragging || (response.hovered() && self.hovered_object.is_some())
    }

    fn find_object_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
        limit_to_area: Option<usize>,
    ) -> Option<DraggedObject> {
        let click_tolerance_sq = (self.node_radius * 3.0).powi(2);

        for (area_idx, area) in self.areas.iter().enumerate().rev() {
            if let Some(limit_idx) = limit_to_area
                && area_idx != limit_idx
            {
                continue;
            }
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
                AreaShape::Circle {
                    center,
                    radius,
                    points: _,
                } => {
                    let center_screen = projection.project(*center);

                    // Convert radius from meters to screen pixels to correctly detect handle clicks.
                    let point_on_circle_geo = GeoPos {
                        lon: center.lon
                            + (radius / (111_320.0 * center.lat.to_radians().cos().max(1e-6))),
                        lat: center.lat,
                    };
                    let point_on_circle_screen = projection.project(point_on_circle_geo);
                    let radius_pixels = center_screen.distance(point_on_circle_screen);

                    // Check for radius handle
                    let radius_handle_pos = center_screen + egui::vec2(radius_pixels, 0.0);
                    if radius_handle_pos.distance_sq(screen_pos) < click_tolerance_sq {
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
                AreaShape::Ellipse {
                    center,
                    radius_major,
                    radius_minor,
                    rotation,
                    points: _,
                } => {
                    let center_geo = *center;
                    let point_on_major_geo = GeoPos {
                        lon: center_geo.lon
                            + (radius_major
                                / (111_320.0 * center_geo.lat.to_radians().cos().max(1e-6))),
                        lat: center_geo.lat,
                    };
                    let point_on_minor_geo = GeoPos {
                        lon: center_geo.lon,
                        lat: center_geo.lat + (radius_minor / 110_574.0),
                    };
                    let center_screen = projection.project(center_geo);
                    let point_on_major_screen = projection.project(point_on_major_geo);
                    let point_on_minor_screen = projection.project(point_on_minor_geo);
                    let radius_major_pixels = center_screen.distance(point_on_major_screen);
                    let radius_minor_pixels = center_screen.distance(point_on_minor_screen);

                    let cos_rot = rotation.cos() as f32;
                    let sin_rot = rotation.sin() as f32;

                    // Rotation Handle: checked first
                    let rotation_handle_pos = center_screen
                        + egui::vec2(
                            (radius_major_pixels + 20.0) * cos_rot,
                            (radius_major_pixels + 20.0) * sin_rot,
                        );
                    if rotation_handle_pos.distance_sq(screen_pos) < click_tolerance_sq {
                        return Some(DraggedObject::EllipseRotation {
                            area_index: area_idx,
                        });
                    }

                    // Major Radius Handle
                    let major_handle_pos = center_screen
                        + egui::vec2(radius_major_pixels * cos_rot, radius_major_pixels * sin_rot);
                    if major_handle_pos.distance_sq(screen_pos) < click_tolerance_sq {
                        return Some(DraggedObject::EllipseMajorRadius {
                            area_index: area_idx,
                        });
                    }

                    // Minor Radius Handle
                    let minor_handle_pos = center_screen
                        + egui::vec2(
                            -radius_minor_pixels * sin_rot,
                            radius_minor_pixels * cos_rot,
                        );
                    if minor_handle_pos.distance_sq(screen_pos) < click_tolerance_sq {
                        return Some(DraggedObject::EllipseMinorRadius {
                            area_index: area_idx,
                        });
                    }

                    // Center Handle
                    if center_screen.distance_sq(screen_pos) < click_tolerance_sq {
                        return Some(DraggedObject::EllipseCenter {
                            area_index: area_idx,
                        });
                    }
                }
            }
        }

        None
    }

    fn find_node_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
        limit_to_area: Option<usize>,
    ) -> Option<(usize, usize)> {
        match self.find_object_at(screen_pos, projection, limit_to_area) {
            Some(DraggedObject::PolygonNode {
                area_index,
                node_index,
            }) => Some((area_index, node_index)),
            _ => None,
        }
    }

    fn find_line_segment_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
        limit_to_area: Option<usize>,
    ) -> Option<(usize, usize)> {
        let click_tolerance = (self.node_radius * 2.0).powi(2);

        for (area_idx, area) in self.areas.iter().enumerate().rev() {
            if let Some(limit_idx) = limit_to_area
                && area_idx != limit_idx
            {
                continue;
            }
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
            return false; // TODO: Should not happen
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
            if p1_idx != prev_node_idx
                && p2_idx != prev_node_idx
                && segments_intersect(new_edge1.0, new_edge1.1, edge_to_check.0, edge_to_check.1)
            {
                return false;
            }

            // Check against the second new edge.
            if p1_idx != next_node_idx
                && p2_idx != next_node_idx
                && segments_intersect(new_edge2.0, new_edge2.1, edge_to_check.0, edge_to_check.1)
            {
                return false;
            }
        }

        true
    }
}

impl Area {
    /// Checks if the area can be successfully triangulated.
    fn can_triangulate(&self, projection: &MapProjection) -> bool {
        let points = self.get_points(projection);
        let screen_points: Vec<Pos2> = points.iter().map(|p| projection.project(*p)).collect();

        if screen_points.len() < 3 {
            return true;
        }

        let flat_points: Vec<f64> = screen_points
            .iter()
            .flat_map(|p| [f64::from(p.x), f64::from(p.y)])
            .collect();
        earcutr::earcut(&flat_points, &[], 2).is_ok()
    }

    /// Returns the points of the area. For a circle, it generates a polygon approximation.
    fn get_points(&self, projection: &MapProjection) -> Vec<GeoPos> {
        match &self.shape {
            AreaShape::Polygon(points) => points.clone(),
            AreaShape::Circle {
                center,
                radius,
                points,
            } => {
                // Convert radius from meters to screen pixels.
                let center_geo = *center;
                let point_on_circle_geo = GeoPos {
                    lon: center_geo.lon
                        + (radius / (111_320.0 * center_geo.lat.to_radians().cos().max(1e-6))),
                    lat: center_geo.lat,
                };
                let center_screen = projection.project(center_geo);
                let point_on_circle_screen = projection.project(point_on_circle_geo);
                let radius_pixels = center_screen.distance(point_on_circle_screen);

                let num_points = if let Some(points) = points {
                    *points
                } else {
                    // Automatically determine the number of points based on the circle's radius
                    // to ensure it looks smooth.
                    (f64::from(radius_pixels) * 2.0 * std::f64::consts::PI / 10.0).ceil() as i64
                };
                let num_points = num_points.max(3);
                let mut circle_points = Vec::with_capacity(num_points as usize);

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
            AreaShape::Ellipse {
                center,
                radius_major,
                radius_minor,
                rotation,
                points,
            } => {
                let center_geo = *center;
                let point_on_major_geo = GeoPos {
                    lon: center_geo.lon
                        + (radius_major
                            / (111_320.0 * center_geo.lat.to_radians().cos().max(1e-6))),
                    lat: center_geo.lat,
                };
                let point_on_minor_geo = GeoPos {
                    lon: center_geo.lon,
                    lat: center_geo.lat + (radius_minor / 110_574.0),
                };
                let center_screen = projection.project(center_geo);
                let point_on_major_screen = projection.project(point_on_major_geo);
                let point_on_minor_screen = projection.project(point_on_minor_geo);
                let radius_major_pixels = center_screen.distance(point_on_major_screen);
                let radius_minor_pixels = center_screen.distance(point_on_minor_screen);

                let num_points = if let Some(points) = points {
                    *points
                } else {
                    let max_radius = radius_major_pixels.max(radius_minor_pixels);
                    (f64::from(max_radius) * 2.0 * std::f64::consts::PI / 10.0).ceil() as i64
                };
                let num_points = num_points.max(3);
                let mut ellipse_points = Vec::with_capacity(num_points as usize);
                let cos_rot = rotation.cos();
                let sin_rot = rotation.sin();

                for i in 0..num_points {
                    let angle = (i as f64 / num_points as f64) * 2.0 * std::f64::consts::PI;
                    let dx = f64::from(radius_major_pixels) * angle.cos();
                    let dy = f64::from(radius_minor_pixels) * angle.sin();
                    let rx = dx * cos_rot - dy * sin_rot;
                    let ry = dx * sin_rot + dy * cos_rot;
                    let point_screen = center_screen + egui::vec2(rx as f32, ry as f32);
                    ellipse_points.push(projection.unproject(point_screen));
                }
                ellipse_points
            }
        }
    }

    /// Checks if a screen position is inside the area.
    pub fn contains(&self, pos: Pos2, projection: &MapProjection) -> bool {
        match &self.shape {
            AreaShape::Circle { center, radius, .. } => {
                let center_screen = projection.project(*center);
                let point_on_circle_geo = GeoPos {
                    lon: center.lon
                        + (radius / (111_320.0 * center.lat.to_radians().cos().max(1e-6))),
                    lat: center.lat,
                };
                let point_on_circle_screen = projection.project(point_on_circle_geo);
                let radius_pixels = center_screen.distance(point_on_circle_screen);
                if radius_pixels <= 0.0 {
                    return false;
                }
                center_screen.distance_sq(pos) <= radius_pixels.powi(2)
            }
            AreaShape::Ellipse {
                center,
                radius_major,
                radius_minor,
                rotation,
                ..
            } => {
                let center_geo = *center;
                let point_on_major_geo = GeoPos {
                    lon: center_geo.lon
                        + (radius_major
                            / (111_320.0 * center_geo.lat.to_radians().cos().max(1e-6))),
                    lat: center_geo.lat,
                };
                let point_on_minor_geo = GeoPos {
                    lon: center_geo.lon,
                    lat: center_geo.lat + (radius_minor / 110_574.0),
                };
                let center_screen = projection.project(center_geo);
                let point_on_major_screen = projection.project(point_on_major_geo);
                let point_on_minor_screen = projection.project(point_on_minor_geo);
                let radius_major_pixels = center_screen.distance(point_on_major_screen);
                let radius_minor_pixels = center_screen.distance(point_on_minor_screen);

                if radius_major_pixels <= 0.0 || radius_minor_pixels <= 0.0 {
                    return false;
                }

                let v = pos - center_screen;
                let cos_rot = rotation.cos() as f32;
                let sin_rot = rotation.sin() as f32;
                let local_x = v.x * cos_rot + v.y * sin_rot;
                let local_y = -v.x * sin_rot + v.y * cos_rot;

                let rx_f64 = f64::from(radius_major_pixels);
                let ry_f64 = f64::from(radius_minor_pixels);
                (f64::from(local_x) / rx_f64).powi(2) + (f64::from(local_y) / ry_f64).powi(2) <= 1.0
            }
            AreaShape::Polygon(_) => {
                let points = self.get_points(projection);
                let screen_points: Vec<Pos2> =
                    points.iter().map(|p| projection.project(*p)).collect();
                if screen_points.len() < 3 {
                    return false;
                }
                let flat_points: Vec<f64> = screen_points
                    .iter()
                    .flat_map(|p| [f64::from(p.x), f64::from(p.y)])
                    .collect();
                if let Ok(indices) = earcutr::earcut(&flat_points, &[], 2) {
                    for chunk in indices.chunks_exact(3) {
                        let p1 = screen_points[chunk[0]];
                        let p2 = screen_points[chunk[1]];
                        let p3 = screen_points[chunk[2]];
                        if point_in_triangle(pos, p1, p2, p3) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}

fn point_in_triangle(p: Pos2, a: Pos2, b: Pos2, c: Pos2) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);

    !(has_neg && has_pos)
}

fn sign(p1: Pos2, p2: Pos2, p3: Pos2) -> f32 {
    (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
}

/// Generates diagonal hatching line segments clipped to the given polygon.
///
/// `screen_points` are the polygon vertices in screen space (must be >= 3 points).
/// `spacing` is the distance in pixels between parallel hatching lines.
/// `angle` is the angle of the hatching lines in radians (0 = horizontal, PI/4 = 45° diagonal).
///
/// Returns a list of line segments `(start, end)` that lie inside the polygon.
fn generate_hatching_lines(screen_points: &[Pos2], spacing: f32, angle: f32) -> Vec<(Pos2, Pos2)> {
    if screen_points.len() < 3 || spacing <= 0.0 {
        return Vec::new();
    }

    // Direction along the hatching lines and perpendicular to them.
    let dir = egui::vec2(angle.cos(), angle.sin());
    let perp = egui::vec2(-angle.sin(), angle.cos());

    // Project all polygon points onto the perpendicular axis to find the sweep range.
    let mut min_perp = f32::MAX;
    let mut max_perp = f32::MIN;
    for p in screen_points {
        let d = p.to_vec2().dot(perp);
        min_perp = min_perp.min(d);
        max_perp = max_perp.max(d);
    }

    let n = screen_points.len();
    let mut segments = Vec::new();

    // Sweep parallel lines across the polygon.
    let mut offset = min_perp + spacing;
    while offset < max_perp {
        // A point on the current sweep line: origin + offset along the perpendicular.
        let line_origin = Pos2::ZERO + perp * offset;

        // Find intersections of this sweep line with every polygon edge.
        let mut t_values: Vec<f32> = Vec::new();
        for i in 0..n {
            let a = screen_points[i];
            let b = screen_points[(i + 1) % n];
            let edge = b - a;

            // Solve: a + t_edge * edge = line_origin + t_line * dir
            // Cross product form: (a - line_origin) × dir = t_edge * (edge × dir)
            let denom = edge.x * dir.y - edge.y * dir.x;
            if denom.abs() < 1e-9 {
                continue; // Edge is parallel to the hatching line.
            }

            let diff = a - line_origin;
            let t_edge = -(diff.x * dir.y - diff.y * dir.x) / denom;

            if (0.0..=1.0).contains(&t_edge) {
                // Compute t_line: the parameter along the sweep line direction.
                let t_line = if dir.x.abs() > dir.y.abs() {
                    (a.x - line_origin.x + t_edge * edge.x) / dir.x
                } else {
                    (a.y - line_origin.y + t_edge * edge.y) / dir.y
                };
                t_values.push(t_line);
            }
        }

        // Sort intersections along the sweep line.
        t_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Pair up intersections (even-odd rule) to get interior segments.
        for pair in t_values.chunks_exact(2) {
            let p1 = line_origin + dir * pair[0];
            let p2 = line_origin + dir * pair[1];
            segments.push((p1, p2));
        }

        offset += spacing;
    }

    segments
}

impl Layer for AreaLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn opacity(&self) -> f32 {
        self.opacity
    }

    fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity;
    }

    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        match self.mode {
            AreaMode::Disabled => {
                self.hovered_object = None;
                false
            }
            AreaMode::Modify => self.handle_modify_input(response, projection, None),
            AreaMode::ModifySelected => {
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    // Find if any area was clicked to select it.
                    let clicked_area_idx =
                        self.areas.iter().enumerate().rev().find_map(|(idx, area)| {
                            if area.contains(pointer_pos, projection) {
                                Some(idx)
                            } else {
                                None
                            }
                        });

                    if clicked_area_idx != self.selected_area {
                        self.selected_area = clicked_area_idx;
                        return true;
                    }
                }

                if let Some(selected_idx) = self.selected_area {
                    self.handle_modify_input(response, projection, Some(selected_idx))
                } else {
                    false
                }
            }
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for (area_idx, area) in self.areas.iter().enumerate() {
            let points = area.get_points(projection);
            let screen_points: Vec<Pos2> = points.iter().map(|p| projection.project(*p)).collect();

            // Draw polygon outline
            if screen_points.len() >= 3 {
                let is_selected =
                    self.mode == AreaMode::ModifySelected && self.selected_area == Some(area_idx);
                let stroke = if is_selected {
                    Stroke {
                        width: area.stroke.width * 2.0,
                        color: area.stroke.color.gamma_multiply(self.opacity),
                    }
                } else {
                    Stroke {
                        color: area.stroke.color.gamma_multiply(self.opacity),
                        ..area.stroke
                    }
                };

                // Use a generic path for the stroke.
                let path_shape = Shape::Path(egui::epaint::PathShape {
                    points: screen_points.clone(),
                    closed: true,
                    fill: Color32::TRANSPARENT,
                    stroke: stroke.into(),
                });
                painter.add(path_shape);

                match area.fill_type {
                    FillType::None => { /* No fill */ }
                    FillType::Solid => {
                        // Triangulate for the fill.
                        let flat_points: Vec<f64> = screen_points
                            .iter()
                            .flat_map(|p| [f64::from(p.x), f64::from(p.y)])
                            .collect();
                        match earcutr::earcut(&flat_points, &[], 2) {
                            Ok(indices) => {
                                let mesh = Mesh {
                                    vertices: screen_points
                                        .iter()
                                        .map(|p| egui::epaint::Vertex {
                                            pos: *p,
                                            uv: Default::default(),
                                            color: area.fill.gamma_multiply(self.opacity),
                                        })
                                        .collect(),
                                    indices: indices.into_iter().map(|i| i as u32).collect(),
                                    ..Default::default()
                                };
                                painter.add(Shape::Mesh(mesh.into()));
                            }
                            Err(e) => {
                                warn!("Failed to triangulate area: {e:?}");
                            }
                        }
                    }
                    FillType::Hatching => {
                        let segments = generate_hatching_lines(
                            &screen_points,
                            8.0,
                            std::f32::consts::FRAC_PI_4,
                        );
                        for (a, b) in segments {
                            painter.line_segment(
                                [a, b],
                                Stroke {
                                    color: area.stroke.color.gamma_multiply(self.opacity),
                                    ..area.stroke
                                },
                            );
                        }
                    }
                }
            } else {
                warn!("Invalid amount of points in area. {area:?}");
            }

            // Draw nodes only when in modify mode or if specifically selected
            let show_nodes = self.mode == AreaMode::Modify
                || (self.mode == AreaMode::ModifySelected && self.selected_area == Some(area_idx));
            if show_nodes {
                let drag_fill = Color32::from_rgb(255, 140, 0); // High-contrast orange for active dragging
                match &area.shape {
                    AreaShape::Polygon(_) => {
                        for (node_idx, point) in screen_points.iter().enumerate() {
                            let is_dragged = if let Some(DraggedObject::PolygonNode {
                                area_index,
                                node_index,
                            }) = &self.dragged_object {
                                *area_index == area_idx && *node_index == node_idx
                            } else {
                                false
                            };

                            let is_hovered = if let Some(DraggedObject::PolygonNode {
                                area_index,
                                node_index,
                            }) = &self.hovered_object {
                                *area_index == area_idx && *node_index == node_idx
                            } else {
                                false
                            };

                            if is_dragged {
                                painter.circle_filled(
                                    *point,
                                    self.node_radius * 1.2,
                                    drag_fill.gamma_multiply(self.opacity),
                                );
                                painter.circle_stroke(
                                    *point,
                                    self.node_radius * 3.0,
                                    Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                                );
                            } else {
                                painter.circle_filled(
                                    *point,
                                    self.node_radius,
                                    self.node_fill.gamma_multiply(self.opacity),
                                );
                                if is_hovered {
                                    painter.circle_stroke(
                                        *point,
                                        self.node_radius * 3.0,
                                        Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                    );
                                }
                            }
                        }
                    }
                    AreaShape::Circle {
                        center,
                        radius,
                        points: _,
                    } => {
                        let center_screen = projection.project(*center);

                        // Convert radius from meters to screen pixels to correctly position the handle.
                        let point_on_circle_geo = GeoPos {
                            lon: center.lon
                                + (radius / (111_320.0 * center.lat.to_radians().cos().max(1e-6))),
                            lat: center.lat,
                        };
                        let point_on_circle_screen = projection.project(point_on_circle_geo);
                        let radius_pixels = center_screen.distance(point_on_circle_screen);

                        // Center Handle
                        let center_dragged = if let Some(DraggedObject::CircleCenter { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let center_hovered = if let Some(DraggedObject::CircleCenter { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        if center_dragged {
                            painter.circle_filled(
                                center_screen,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                center_screen,
                                self.node_radius * 3.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                center_screen,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if center_hovered {
                                painter.circle_stroke(
                                    center_screen,
                                    self.node_radius * 3.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }

                        // Radius Handle
                        let radius_handle_pos = center_screen + egui::vec2(radius_pixels, 0.0);
                        let radius_dragged = if let Some(DraggedObject::CircleRadius { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let radius_hovered = if let Some(DraggedObject::CircleRadius { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        if radius_dragged {
                            painter.circle_filled(
                                radius_handle_pos,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                radius_handle_pos,
                                self.node_radius * 2.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                radius_handle_pos,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if radius_hovered {
                                painter.circle_stroke(
                                    radius_handle_pos,
                                    self.node_radius * 2.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }
                    }
                    AreaShape::Ellipse {
                        center,
                        radius_major,
                        radius_minor,
                        rotation,
                        points: _,
                    } => {
                        let center_screen = projection.project(*center);

                        let point_on_major_geo = GeoPos {
                            lon: center.lon
                                + (radius_major
                                    / (111_320.0 * center.lat.to_radians().cos().max(1e-6))),
                            lat: center.lat,
                        };
                        let point_on_minor_geo = GeoPos {
                            lon: center.lon,
                            lat: center.lat + (radius_minor / 110_574.0),
                        };
                        let point_on_major_screen = projection.project(point_on_major_geo);
                        let point_on_minor_screen = projection.project(point_on_minor_geo);
                        let radius_major_pixels = center_screen.distance(point_on_major_screen);
                        let radius_minor_pixels = center_screen.distance(point_on_minor_screen);

                        let cos_rot = rotation.cos() as f32;
                        let sin_rot = rotation.sin() as f32;

                        let major_handle_pos = center_screen
                            + egui::vec2(
                                radius_major_pixels * cos_rot,
                                radius_major_pixels * sin_rot,
                            );
                        let minor_handle_pos = center_screen
                            + egui::vec2(
                                -radius_minor_pixels * sin_rot,
                                radius_minor_pixels * cos_rot,
                            );
                        let rotation_handle_pos = center_screen
                            + egui::vec2(
                                (radius_major_pixels + 20.0) * cos_rot,
                                (radius_major_pixels + 20.0) * sin_rot,
                            );

                        let center_dragged = if let Some(DraggedObject::EllipseCenter { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let center_hovered = if let Some(DraggedObject::EllipseCenter { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        let major_dragged = if let Some(DraggedObject::EllipseMajorRadius { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let major_hovered = if let Some(DraggedObject::EllipseMajorRadius { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        let minor_dragged = if let Some(DraggedObject::EllipseMinorRadius { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let minor_hovered = if let Some(DraggedObject::EllipseMinorRadius { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        let rotation_dragged = if let Some(DraggedObject::EllipseRotation { area_index }) = &self.dragged_object {
                            *area_index == area_idx
                        } else {
                            false
                        };
                        let rotation_hovered = if let Some(DraggedObject::EllipseRotation { area_index }) = &self.hovered_object {
                            *area_index == area_idx
                        } else {
                            false
                        };

                        // Draw connection line between major handle and rotation handle.
                        let conn_line_stroke = if rotation_dragged {
                            Stroke::new(1.5, drag_fill.gamma_multiply(self.opacity))
                        } else {
                            Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity))
                        };
                        painter.line_segment(
                            [major_handle_pos, rotation_handle_pos],
                            conn_line_stroke,
                        );

                        // 1. Center
                        if center_dragged {
                            painter.circle_filled(
                                center_screen,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                center_screen,
                                self.node_radius * 3.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                center_screen,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if center_hovered {
                                painter.circle_stroke(
                                    center_screen,
                                    self.node_radius * 3.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }

                        // 2. Major radius
                        if major_dragged {
                            painter.circle_filled(
                                major_handle_pos,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                major_handle_pos,
                                self.node_radius * 2.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                major_handle_pos,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if major_hovered {
                                painter.circle_stroke(
                                    major_handle_pos,
                                    self.node_radius * 2.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }

                        // 3. Minor radius
                        if minor_dragged {
                            painter.circle_filled(
                                minor_handle_pos,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                minor_handle_pos,
                                self.node_radius * 2.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                minor_handle_pos,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if minor_hovered {
                                painter.circle_stroke(
                                    minor_handle_pos,
                                    self.node_radius * 2.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }

                        // 4. Rotation
                        if rotation_dragged {
                            painter.circle_filled(
                                rotation_handle_pos,
                                self.node_radius * 1.2,
                                drag_fill.gamma_multiply(self.opacity),
                            );
                            painter.circle_stroke(
                                rotation_handle_pos,
                                self.node_radius * 2.0,
                                Stroke::new(2.0, drag_fill.gamma_multiply(self.opacity)),
                            );
                        } else {
                            painter.circle_filled(
                                rotation_handle_pos,
                                self.node_radius,
                                self.node_fill.gamma_multiply(self.opacity),
                            );
                            if rotation_hovered {
                                painter.circle_stroke(
                                    rotation_handle_pos,
                                    self.node_radius * 2.0,
                                    Stroke::new(1.0, self.node_fill.gamma_multiply(self.opacity)),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::MapProjection;
    use egui::{Rect, pos2, vec2};

    // Helper for creating a dummy projection for tests
    fn dummy_projection() -> MapProjection {
        MapProjection::new(
            10,                // zoom
            (0.0, 0.0).into(), // center
            Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1000.0)),
        )
    }

    #[test]
    fn area_layer_new() {
        let layer = AreaLayer::default();
        assert_eq!(layer.mode, AreaMode::Disabled);
        assert!(layer.areas.is_empty());
        assert_eq!(layer.node_radius, 5.0);
    }

    #[test]
    fn area_layer_add_area() {
        let mut layer = AreaLayer::default();
        assert_eq!(layer.areas.len(), 0);

        layer.add_area(Area {
            shape: AreaShape::Polygon(vec![
                (0.0, 0.0).into(),
                (1.0, 0.0).into(),
                (0.0, 1.0).into(),
            ]),
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        });

        assert_eq!(layer.areas.len(), 1);
    }

    #[test]
    fn circle_get_points_with_fixed_number() {
        let projection = dummy_projection();
        let area = Area {
            shape: AreaShape::Circle {
                center: (0.0, 0.0).into(),
                radius: 1000.0,
                points: Some(16),
            },
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        };

        let points = area.get_points(&projection);
        assert_eq!(points.len(), 16);
    }

    #[test]
    fn find_object_at_empty() {
        let layer = AreaLayer::default();
        let projection = dummy_projection();
        let position = pos2(100.0, 100.0);

        assert!(layer.find_object_at(position, &projection, None).is_none());
    }

    #[test]
    fn find_object_at_polygon_node() {
        let projection = dummy_projection();
        let mut layer = AreaLayer::default();
        let geo_pos = projection.unproject(pos2(100.0, 100.0));

        layer.add_area(Area {
            shape: AreaShape::Polygon(vec![geo_pos]),
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        });

        // Position is exactly on the node
        let found = layer.find_object_at(pos2(100.0, 100.0), &projection, None);
        assert!(matches!(
            found,
            Some(DraggedObject::PolygonNode {
                area_index: 0,
                node_index: 0
            })
        ));

        // Position is slightly off but within tolerance
        let found_nearby = layer.find_object_at(pos2(101.0, 101.0), &projection, None);
        assert!(matches!(
            found_nearby,
            Some(DraggedObject::PolygonNode {
                area_index: 0,
                node_index: 0
            })
        ));

        // Position is too far
        let not_found = layer.find_object_at(pos2(200.0, 200.0), &projection, None);
        assert!(not_found.is_none());
    }

    #[test]
    fn area_layer_serde() {
        let mut layer = AreaLayer::default();
        layer.add_area(Area {
            shape: AreaShape::Polygon(vec![(0.0, 0.0).into()]),
            stroke: Stroke::new(1.0, Color32::RED),
            fill: Color32::BLUE,
            fill_type: Default::default(),
        });

        let json = serde_json::to_string(&layer).unwrap();
        let deserialized: AreaLayer = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.areas.len(), 1);
        assert_eq!(deserialized.mode, AreaMode::Disabled); // Restored to default
    }

    #[test]
    fn test_can_triangulate_valid() {
        let projection = dummy_projection();
        let area = Area {
            shape: AreaShape::Polygon(vec![
                (0.0, 0.0).into(),
                (10.0, 0.0).into(),
                (0.0, 10.0).into(),
            ]),
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        };

        assert!(area.can_triangulate(&projection));
    }

    #[test]
    fn test_can_triangulate_insufficient_points() {
        let projection = dummy_projection();
        let area = Area {
            shape: AreaShape::Polygon(vec![(0.0, 0.0).into(), (10.0, 0.0).into()]),
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        };

        // Should return true as we don't consider < 3 points as a triangulation failure
        // (it simply doesn't draw anything)
        assert!(area.can_triangulate(&projection));
    }

    #[cfg(feature = "geojson")]
    mod geojson_tests {
        use super::*;

        #[test]
        fn area_layer_geojson_polygon() {
            let mut layer = AreaLayer::default();
            layer.add_area(Area {
                shape: AreaShape::Polygon(vec![
                    (10.0, 20.0).into(),
                    (30.0, 40.0).into(),
                    (50.0, 60.0).into(),
                ]),
                stroke: Stroke::new(2.0, Color32::from_rgb(0, 0, 255)),
                fill: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
                fill_type: Default::default(),
            });

            let geojson_str = layer.to_geojson_str().unwrap();

            let mut new_layer = AreaLayer::default();
            new_layer.from_geojson_str(&geojson_str).unwrap();

            assert_eq!(new_layer.areas.len(), 1);
            assert_eq!(layer.areas[0], new_layer.areas[0]);
        }

        #[test]
        fn area_layer_geojson_circle() {
            let mut layer = AreaLayer::default();
            layer.add_area(Area {
                shape: AreaShape::Circle {
                    center: (10.0, 20.0).into(),
                    radius: 1000.0,
                    points: Some(32),
                },
                stroke: Default::default(),
                fill: Default::default(),
                fill_type: Default::default(),
            });

            let geojson_str = layer.to_geojson_str().unwrap();
            let mut new_layer = AreaLayer::default();
            new_layer.from_geojson_str(&geojson_str).unwrap();

            assert_eq!(new_layer.areas.len(), 1);
            assert_eq!(layer.areas[0].shape, new_layer.areas[0].shape);
        }

        #[test]
        fn area_layer_geojson_ellipse() {
            let mut layer = AreaLayer::default();
            layer.add_area(Area {
                shape: AreaShape::Ellipse {
                    center: (10.0, 20.0).into(),
                    radius_major: 2000.0,
                    radius_minor: 1000.0,
                    rotation: 0.78,
                    points: Some(32),
                },
                stroke: Default::default(),
                fill: Default::default(),
                fill_type: Default::default(),
            });

            let geojson_str = layer.to_geojson_str().unwrap();
            let mut new_layer = AreaLayer::default();
            new_layer.from_geojson_str(&geojson_str).unwrap();

            assert_eq!(new_layer.areas.len(), 1);
            assert_eq!(layer.areas[0].shape, new_layer.areas[0].shape);
        }
    }

    #[test]
    fn ellipse_get_points_with_fixed_number() {
        let projection = dummy_projection();
        let area = Area {
            shape: AreaShape::Ellipse {
                center: (0.0, 0.0).into(),
                radius_major: 2000.0,
                radius_minor: 1000.0,
                rotation: 0.5,
                points: Some(24),
            },
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        };

        let points = area.get_points(&projection);
        assert_eq!(points.len(), 24);
    }

    #[test]
    fn ellipse_containment() {
        let projection = dummy_projection();
        let area = Area {
            shape: AreaShape::Ellipse {
                center: (0.0, 0.0).into(),
                radius_major: 2000.0,
                radius_minor: 1000.0,
                rotation: 0.0,
                points: None,
            },
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        };

        // Center is inside
        assert!(area.contains(projection.project((0.0, 0.0).into()), &projection));

        // Slightly to the East (lon changes) should be inside
        let point_inside = GeoPos {
            lon: 0.005,
            lat: 0.0,
        };
        assert!(area.contains(projection.project(point_inside), &projection));

        // Very far to the East (lon changes) should be outside
        let point_outside = GeoPos { lon: 0.5, lat: 0.0 };
        assert!(!area.contains(projection.project(point_outside), &projection));
    }

    #[test]
    fn find_node_at_on_segment() {
        let projection = dummy_projection();
        let mut layer = AreaLayer::default();

        let p1 = projection.unproject(pos2(100.0, 100.0));
        let p2 = projection.unproject(pos2(200.0, 100.0));

        layer.add_area(Area {
            shape: AreaShape::Polygon(vec![p1, p2, projection.unproject(pos2(150.0, 200.0))]), // Triangle
            stroke: Default::default(),
            fill: Default::default(),
            fill_type: Default::default(),
        });

        // Click exactly between p1 and p2
        let click_pos = pos2(150.0, 100.0);

        // Should NOT find a node
        assert!(layer.find_node_at(click_pos, &projection, None).is_none());

        // Should find the segment
        let segment = layer.find_line_segment_at(click_pos, &projection, None);
        assert!(segment.is_some());
        assert_eq!(segment.unwrap().0, 0); // area_index
        assert_eq!(segment.unwrap().1, 0);
    }
}
