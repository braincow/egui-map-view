use crate::layers::{serde_color32, serde_stroke};
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Pos2, Stroke};
use serde::{Deserialize, Serialize};

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
    /// Diagonal hatching lines using the fill color and stroke width.
    Hatching,
}

/// A polygon area on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Area {
    /// The unique identifier of the area.
    #[serde(default = "uuid::Uuid::new_v4")]
    pub id: uuid::Uuid,

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
pub(crate) enum DraggedObject {
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

impl Default for Area {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            shape: AreaShape::Polygon(Vec::new()),
            stroke: Stroke::default(),
            fill: Color32::TRANSPARENT,
            fill_type: FillType::Solid,
        }
    }
}

impl Area {
    /// Creates a new area with the given shape, stroke, fill, and a newly generated ID.
    pub fn new(shape: AreaShape, stroke: Stroke, fill: Color32) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            shape,
            stroke,
            fill,
            fill_type: FillType::Solid,
        }
    }

    /// Checks if the area can be successfully triangulated.
    pub(crate) fn can_triangulate(&self, projection: &MapProjection) -> bool {
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
    pub(crate) fn get_points(&self, projection: &MapProjection) -> Vec<GeoPos> {
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
