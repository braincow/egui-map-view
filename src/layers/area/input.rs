use crate::layers::{dist_sq_to_segment, projection_factor, segments_intersect};
use crate::projection::{GeoPos, MapProjection};
use egui::{Pos2, Response};
use log::warn;

use super::layer::AreaLayer;
use super::types::{AreaShape, DraggedObject};

impl AreaLayer {
    pub(crate) fn handle_modify_input(
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

    pub(crate) fn find_object_at(
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

    pub(crate) fn find_node_at(
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

    pub(crate) fn find_line_segment_at(
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
