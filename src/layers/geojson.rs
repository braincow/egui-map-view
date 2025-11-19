//! GeoJSON serialization and deserialization for layers.

use super::area::{Area, AreaShape};
use super::drawing::Polyline;
use super::text::{Text, TextSize};
use crate::projection::GeoPos;
use egui::{Color32, Stroke};
use geojson::{Feature, Geometry, Value};
use serde_json::{Map, Value as JsonValue};

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
                        .map(|gp| (*gp).into())
                        .collect(),
                ];
                feature.geometry = Some(Geometry::new(Value::Polygon(polygon_points)));
            }
            AreaShape::Circle {
                center,
                radius,
                points,
            } => {
                let point = Geometry::new(Value::Point(center.into()));
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

impl TryFrom<Feature> for Area {
    type Error = String;

    fn try_from(feature: Feature) -> Result<Self, Self::Error> {
        let shape = if let Some(geometry) = &feature.geometry {
            match &geometry.value {
                Value::Polygon(points) => {
                    let mut polygon_points: Vec<GeoPos> = points
                        .first()
                        .ok_or("Polygon has no rings")?
                        .iter()
                        .map(|pos| pos.clone().into())
                        .collect();

                    // Remove the closing point, as AreaShape::Polygon doesn't expect it.
                    if polygon_points.first() == polygon_points.last() {
                        polygon_points.pop();
                    }

                    Some(AreaShape::Polygon(polygon_points))
                }
                Value::Point(point) => {
                    let properties = feature
                        .properties
                        .as_ref()
                        .ok_or("Feature has no properties")?;
                    let center: GeoPos = point.clone().into();
                    let radius = properties
                        .get("radius")
                        .and_then(|v| v.as_f64())
                        .unwrap_or_default();
                    let points = properties.get("points").and_then(|v| v.as_i64());

                    if radius <= 0.0 {
                        return Err("Radius must be greater than 0".to_string());
                    }

                    Some(AreaShape::Circle {
                        center,
                        radius,
                        points,
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        let shape = shape.ok_or("Unsupported geometry or missing shape data")?;

        // default stroke and fill settings to use if not present in the feature properties
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

        Ok(Area {
            shape,
            stroke,
            fill,
        })
    }
}

impl From<Polyline> for Feature {
    fn from(polyline: Polyline) -> Self {
        let mut feature = Feature::default();
        let mut properties = Map::new();
        add_version_to_properties(&mut properties);
        feature.properties = Some(properties);
        let line_string: Vec<Vec<f64>> = polyline.0.iter().map(|gp| (*gp).into()).collect();
        feature.geometry = Some(Geometry::new(Value::LineString(line_string)));
        feature
    }
}

impl TryFrom<Feature> for Polyline {
    type Error = String;

    fn try_from(feature: Feature) -> Result<Self, Self::Error> {
        if let Some(geometry) = feature.geometry {
            if let Value::LineString(line_string) = geometry.value {
                return Ok(Polyline(
                    line_string.iter().map(|pos| pos.clone().into()).collect(),
                ));
            }
        }
        if let Some(properties) = &feature.properties {
            check_version_from_properties(properties);
        }
        Err("Feature is not a LineString".to_string())
    }
}

impl From<Text> for Feature {
    fn from(text: Text) -> Self {
        let mut feature = Feature::default();
        let mut properties = Map::new();
        add_version_to_properties(&mut properties);
        let point = Geometry::new(Value::Point(text.pos.into()));
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

impl TryFrom<Feature> for Text {
    type Error = String;

    fn try_from(feature: Feature) -> Result<Self, Self::Error> {
        let mut text = Text::default();
        if let Some(geometry) = feature.geometry {
            if let Value::Point(point) = geometry.value {
                text.pos = point.into();
            } else {
                return Err("Feature is not a Point".to_string());
            }
        } else {
            return Err("Feature has no geometry".to_string());
        }

        if let Some(properties) = feature.properties {
            check_version_from_properties(&properties);
            if let Some(value) = properties.get("text") {
                if let Some(s) = value.as_str() {
                    text.text = s.to_string();
                } else {
                    return Err("Property 'text' is not a string".to_string());
                }
            } else {
                return Err("Feature has no 'text' property".to_string());
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
        Ok(text)
    }
}
