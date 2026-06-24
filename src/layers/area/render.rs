use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Mesh, Painter, Pos2, Shape, Stroke};
use log::warn;

use super::hatching::generate_hatching_lines;
use super::layer::AreaLayer;
use super::types::{AreaMode, AreaShape, DraggedObject, FillType};

impl AreaLayer {
    pub(crate) fn draw_layer(&self, painter: &Painter, projection: &MapProjection) {
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
                            Some(projection.widget_rect),
                        );
                        for (a, b) in segments {
                            painter.line_segment(
                                [a, b],
                                Stroke {
                                    width: area.stroke.width,
                                    color: area.fill.gamma_multiply(self.opacity),
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
