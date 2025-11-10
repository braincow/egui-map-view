//! GeoJSON serialization and deserialization for layers.

use super::area::{Area, AreaShape};
use super::drawing::Polyline;
use super::text::{Text, TextSize};
use crate::projection::GeoPos;
use egui::{Color32, Stroke};
use geojson::{Feature, Geometry, Value};
use serde_json::{Map, Value as JsonValue};

fn geo_pos_to_vec(gp: &GeoPos) -> Vec<f64> {
    vec![gp.lon, gp.lat]
}

fn vec_to_geo_pos(pos: &[f64]) -> GeoPos {
    GeoPos {
        lon: pos[0],
        lat: pos[1],
    }
}

/// Adds crate name and version to the feature properties.
fn add_version_to_properties(properties: &mut Map<String, JsonValue>) {
    properties.insert(
        "x-egui-map-view-crate-name".to_string(),
        JsonValue::String(env!("CARGO_PKG_NAME").to_string()),
    );
    properties.insert(
        "x-egui-map-view-crate-version".to_string(),
        JsonValue::String(env!("CARGO_PKG_VERSION").to_string()),
    );
}

/// Checks the crate version from the feature properties and logs a warning on mismatch.
fn check_version_from_properties(properties: &Map<String, JsonValue>) {
    if let (Some(name), Some(version)) = (
        properties
            .get("x-egui-map-view-crate-name")
            .and_then(|v| v.as_str()),
        properties
            .get("x-egui-map-view-crate-version")
            .and_then(|v| v.as_str()),
    ) {
        if name == env!("CARGO_PKG_NAME") && version != env!("CARGO_PKG_VERSION") {
            log::warn!(
                "GeoJSON feature was created with a different version of {}. File version: {}, current version: {}. This might lead to unexpected behavior.",
                name,
                version,
                env!("CARGO_PKG_VERSION")
            );
        }
    } else {
        log::warn!("No egui-map-view version information found in feature properties.");
    }
}

impl From<Area> for Feature {
    fn from(area: Area) -> Self {
        let mut feature = Feature::default();
        let mut properties = Map::new();
        add_version_to_properties(&mut properties);

        properties.insert(
            "stroke_color".to_string(),
            JsonValue::String(area.stroke.color.to_hex()),
        );
        properties.insert(
            "stroke_width".to_string(),
            JsonValue::from(area.stroke.width),
        );
        properties.insert(
            "fill_color".to_string(),
            JsonValue::String(area.fill.to_hex()),
        );

        match area.shape {
            AreaShape::Polygon(points) => {
                let polygon_points: Vec<Vec<Vec<f64>>> = vec![
                    points
                        .iter()
                        // GeoJSON polygons must be closed, so the first and last points must be the same.
                        .chain(points.first())
                        .map(geo_pos_to_vec)
                        .collect(),
                ];
                feature.geometry = Some(Geometry::new(Value::Polygon(polygon_points)));
            }
            AreaShape::Circle {
                center,
                radius,
                points,
            } => {
                let point = Geometry::new(Value::Point(geo_pos_to_vec(&center)));
                feature.geometry = Some(point);
                properties.insert("radius".to_string(), JsonValue::from(radius));
                if let Some(p) = points {
                    properties.insert("points".to_string(), JsonValue::from(p));
                }
            }
        }

        feature.properties = Some(properties);
        feature
    }
}

impl From<Feature> for Area {
    fn from(feature: Feature) -> Self {
        let shape = if let Some(geometry) = feature.geometry {
            match geometry.value {
                Value::Polygon(mut points) => {
                    let mut polygon_points: Vec<GeoPos> = points
                        .pop()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|pos| vec_to_geo_pos(&pos))
                        .collect();

                    // Remove the closing point, as AreaShape::Polygon doesn't expect it.
                    if polygon_points.first() == polygon_points.last() {
                        polygon_points.pop();
                    }

                    AreaShape::Polygon(polygon_points)
                }
                Value::Point(point) => {
                    let properties = feature.properties.as_ref().unwrap();
                    let center = vec_to_geo_pos(&point);
                    let radius = properties
                        .get("radius")
                        .and_then(|v| v.as_f64())
                        .unwrap_or_default();
                    let points = properties.get("points").and_then(|v| v.as_i64());

                    if radius > 0.0 {
                        AreaShape::Circle {
                            center,
                            radius,
                            points,
                        }
                    } else {
                        AreaShape::Polygon(vec![]) // Not a circle, return empty polygon.
                    }
                }
                _ => {
                    // Fallback or error. For now, create a default polygon.
                    AreaShape::Polygon(vec![])
                }
            }
        } else {
            AreaShape::Polygon(vec![])
        };

        let mut stroke = Stroke::new(1.0, Color32::RED);
        let mut fill = Color32::TRANSPARENT;

        if let Some(properties) = &feature.properties {
            check_version_from_properties(properties);
            if let Some(value) = properties.get("stroke_width") {
                if let Some(width) = value.as_f64() {
                    stroke.width = width as f32;
                }
            }
            if let Some(value) = properties.get("stroke_color") {
                if let Some(s) = value.as_str() {
                    if let Ok(color) = Color32::from_hex(s) {
                        stroke.color = color;
                    }
                }
            }
            if let Some(value) = properties.get("fill_color") {
                if let Some(s) = value.as_str() {
                    if let Ok(color) = Color32::from_hex(s) {
                        fill = color;
                    }
                }
            }
        }

        Area {
            shape,
            stroke,
            fill,
        }
    }
}

impl From<Polyline> for Feature {
    fn from(polyline: Polyline) -> Self {
        let mut feature = Feature::default();
        let mut properties = Map::new();
        add_version_to_properties(&mut properties);
        feature.properties = Some(properties);
        let line_string: Vec<Vec<f64>> = polyline.0.iter().map(geo_pos_to_vec).collect();
        feature.geometry = Some(Geometry::new(Value::LineString(line_string)));
        feature
    }
}

impl From<Feature> for Polyline {
    fn from(feature: Feature) -> Self {
        if let Some(geometry) = feature.geometry {
            if let Value::LineString(line_string) = geometry.value {
                return Polyline(line_string.iter().map(|pos| vec_to_geo_pos(pos)).collect());
            }
        }
        if let Some(properties) = &feature.properties {
            check_version_from_properties(properties);
        }
        Polyline(vec![])
    }
}

impl From<Text> for Feature {
    fn from(text: Text) -> Self {
        let mut feature = Feature::default();
        let mut properties = Map::new();
        add_version_to_properties(&mut properties);
        let point = Geometry::new(Value::Point(geo_pos_to_vec(&text.pos)));
        feature.geometry = Some(point);
        properties.insert("text".to_string(), JsonValue::String(text.text));
        properties.insert("color".to_string(), JsonValue::String(text.color.to_hex()));
        properties.insert(
            "background".to_string(),
            JsonValue::String(text.background.to_hex()),
        );

        match text.size {
            TextSize::Static(size) => {
                properties.insert(
                    "size_type".to_string(),
                    JsonValue::String("Static".to_string()),
                );
                properties.insert("size".to_string(), JsonValue::from(size));
            }
            TextSize::Relative(size) => {
                properties.insert(
                    "size_type".to_string(),
                    JsonValue::String("Relative".to_string()),
                );
                properties.insert("size".to_string(), JsonValue::from(size));
            }
        }

        feature.properties = Some(properties);
        feature
    }
}

impl From<Feature> for Text {
    fn from(feature: Feature) -> Self {
        let mut text = Text::default();
        if let Some(geometry) = feature.geometry {
            if let Value::Point(point) = geometry.value {
                text.pos = vec_to_geo_pos(&point);
            }
        }
        if let Some(properties) = feature.properties {
            check_version_from_properties(&properties);
            if let Some(value) = properties.get("text") {
                if let Some(s) = value.as_str() {
                    text.text = s.to_string();
                }
            }
            if let Some(value) = properties.get("color") {
                if let Some(s) = value.as_str() {
                    if let Ok(color) = Color32::from_hex(s) {
                        text.color = color;
                    }
                }
            }
            if let Some(value) = properties.get("background") {
                if let Some(s) = value.as_str() {
                    if let Ok(color) = Color32::from_hex(s) {
                        text.background = color;
                    }
                }
            }
            if let Some(size_type) = properties.get("size_type") {
                if let Some(size) = properties.get("size") {
                    if let Some(size_f32) = size.as_f64() {
                        if size_type == "Static" {
                            text.size = TextSize::Static(size_f32 as f32);
                        } else if size_type == "Relative" {
                            text.size = TextSize::Relative(size_f32 as f32);
                        }
                    }
                }
            }
        }
        text
    }
}
