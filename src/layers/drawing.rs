//! A layer for freeform drawing on the map.
//!
//! # Example
//!
//! ```no_run
//! use eframe::egui;
//! use egui_map_view::{layers::drawing::DrawingLayer, layers::drawing::DrawMode, Map, config::OpenStreetMapConfig};
//!
//! struct MyApp {
//!     map: Map,
//! }
//!
//! impl Default for MyApp {
//!   fn default() -> Self {
//!     let mut map = Map::new(OpenStreetMapConfig::default());
//!      map.add_layer("drawing", DrawingLayer::default());
//!      if let Some(drawing_layer) = map.layer_mut::<DrawingLayer>("drawing") {
//!        drawing_layer.draw_mode = DrawMode::Draw;
//!      }
//!      Self { map }
//!    }
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
use crate::layers::{Layer, dist_sq_to_segment, projection_factor, serde_stroke};
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Painter, Pos2, Response, Stroke};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A polyline on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Polyline(pub Vec<GeoPos>);

/// The mode of the `DrawingLayer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawMode {
    /// The layer is not interactive.
    #[default]
    Disabled,
    /// The user can draw on the map.
    Draw,
    /// The user can erase drawings.
    Erase,
}

/// Layer implementation that allows the user to draw polylines on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DrawingLayer {
    polylines: Vec<Polyline>,

    /// The stroke style for drawing aka line width and color.
    #[serde(with = "serde_stroke")]
    pub stroke: Stroke,

    /// The current drawing mode.
    #[serde(skip)]
    pub draw_mode: DrawMode,
}

impl DrawingLayer {
    /// Serializes the layer to a GeoJSON `FeatureCollection`.
    #[cfg(feature = "geojson")]
    /// Serializes the layer to a GeoJSON `FeatureCollection`.
    #[cfg(feature = "geojson")]
    pub fn to_geojson_str(&self, layer_id: &str) -> Result<String, serde_json::Error> {
        let features: Vec<geojson::Feature> = self
            .polylines
            .clone()
            .into_iter()
            .map(|p| {
                let mut feature = geojson::Feature::from(p);
                if let Some(properties) = &mut feature.properties {
                    properties.insert(
                        "stroke_width".to_string(),
                        serde_json::Value::from(self.stroke.width),
                    );
                    properties.insert(
                        "stroke_color".to_string(),
                        serde_json::Value::String(self.stroke.color.to_hex()),
                    );
                    properties.insert(
                        "layer_id".to_string(),
                        serde_json::Value::String(layer_id.to_string()),
                    );
                }
                feature
            })
            .collect();

        let mut foreign_members = serde_json::Map::new();
        foreign_members.insert(
            "stroke_width".to_string(),
            serde_json::Value::from(self.stroke.width),
        );
        foreign_members.insert(
            "stroke_color".to_string(),
            serde_json::Value::String(self.stroke.color.to_hex()),
        );
        foreign_members.insert(
            "layer_id".to_string(),
            serde_json::Value::String(layer_id.to_string()),
        );

        let feature_collection = geojson::FeatureCollection {
            bbox: None,
            features,
            foreign_members: Some(foreign_members),
        };
        serde_json::to_string(&feature_collection)
    }

    /// Deserializes a GeoJSON `FeatureCollection` and adds the features to the layer.
    ///
    /// If `layer_id` is provided, only features with a matching `layer_id` property will be added.
    /// If `layer_id` is `None`, all valid features will be added.
    #[cfg(feature = "geojson")]
    pub fn from_geojson_str(
        &mut self,
        s: &str,
        layer_id: Option<&str>,
    ) -> Result<(), serde_json::Error> {
        let feature_collection: geojson::FeatureCollection = serde_json::from_str(s)?;
        let new_polylines: Vec<Polyline> = feature_collection
            .features
            .iter()
            .filter_map(|f| {
                // Filter by layer_id if provided
                if let Some(target_id) = layer_id {
                    if let Some(properties) = &f.properties {
                        if let Some(val) = properties.get("layer_id") {
                            if let Some(id) = val.as_str() {
                                if id != target_id {
                                    return None;
                                }
                            } else {
                                // layer_id property exists but is not a string, treat as mismatch
                                return None;
                            }
                        } else {
                            // layer_id property missing, treat as mismatch
                            return None;
                        }
                    } else {
                        // No properties, treat as mismatch
                        return None;
                    }
                }

                let polyline = Polyline::try_from(f.clone()).ok();
                if polyline.is_some() {
                    if let Some(properties) = &f.properties {
                        if let Some(value) = properties.get("stroke_width") {
                            if let Some(width) = value.as_f64() {
                                self.stroke.width = width as f32;
                            }
                        }
                        if let Some(value) = properties.get("stroke_color") {
                            if let Some(s) = value.as_str() {
                                if let Ok(color) = Color32::from_hex(s) {
                                    self.stroke.color = color;
                                }
                            }
                        }
                    }
                }
                polyline
            })
            .collect();
        self.polylines.extend(new_polylines);

        if let Some(foreign_members) = feature_collection.foreign_members {
            // Check layer_id in foreign members if filtering is enabled?
            // The requirement says "only those Polylines are deserialized into the the layer that have that layer ID in place".
            // This implies filtering individual features.
            // However, we might want to respect global properties if they match.
            // But for now, let's just apply global properties if we processed any features OR if we are not filtering?
            // Or maybe just always apply them if they exist?
            // Let's stick to applying them always for now, as it was before, but maybe we should check layer_id here too?
            // The prompt specifically talks about Polylines (features).
            // "only those Polylines are deserialized into the the layer that have that layer ID in place"

            if let Some(value) = foreign_members.get("stroke_width") {
                if let Some(width) = value.as_f64() {
                    self.stroke.width = width as f32;
                }
            }
            if let Some(value) = foreign_members.get("stroke_color") {
                if let Some(s) = value.as_str() {
                    if let Ok(color) = Color32::from_hex(s) {
                        self.stroke.color = color;
                    }
                }
            }
        }

        Ok(())
    }

    /// Creates a new `DrawingLayer`.
    pub fn new(stroke: Stroke) -> Self {
        Self {
            polylines: Vec::new(),
            stroke,
            draw_mode: DrawMode::default(),
        }
    }
}

impl Default for DrawingLayer {
    fn default() -> Self {
        Self {
            polylines: Vec::new(),
            stroke: Stroke::new(2.0, Color32::RED),
            draw_mode: DrawMode::default(),
        }
    }
}

impl DrawingLayer {
    fn handle_draw_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.hovered() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let geo_pos = projection.unproject(pointer_pos);
                if let Some(last_line) = self.polylines.last_mut()
                    && response.ctx.input(|i| i.modifiers.shift)
                {
                    last_line.0.push(geo_pos);
                } else {
                    // No polylines exist yet, so create a new one.
                    let geo_pos2 = projection.unproject(pointer_pos + egui::vec2(1.0, 0.0));
                    self.polylines.push(Polyline(vec![geo_pos, geo_pos2]));
                }
            }
        }

        if response.drag_started() {
            self.polylines.push(Polyline(Vec::new()));
        }

        if response.dragged() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(last_line) = self.polylines.last_mut() {
                    let geo_pos = projection.unproject(pointer_pos);
                    last_line.0.push(geo_pos);
                }
            }
        }

        // When drawing, we consume all interactions over the map,
        // so that the map does not pan or zoom.
        response.hovered()
    }

    fn handle_erase_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if response.hovered() {
            response.ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
        }

        if response.dragged() || response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.erase_at(pointer_pos, projection);
            }
        }
        response.hovered()
    }

    fn erase_at(&mut self, pointer_pos: Pos2, projection: &MapProjection) {
        let erase_radius_screen = self.stroke.width;
        let erase_radius_sq = erase_radius_screen * erase_radius_screen;

        let old_polylines = std::mem::take(&mut self.polylines);
        self.polylines = old_polylines
            .into_iter()
            .flat_map(|polyline| {
                split_polyline_by_erase_circle(
                    &polyline.0,
                    pointer_pos,
                    erase_radius_sq,
                    projection,
                )
                .into_iter()
                .map(Polyline)
            })
            .collect();
    }
}

impl Layer for DrawingLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        match self.draw_mode {
            DrawMode::Disabled => false,
            DrawMode::Draw => self.handle_draw_input(response, projection),
            DrawMode::Erase => self.handle_erase_input(response, projection),
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for polyline in &self.polylines {
            if polyline.0.len() > 1 {
                let screen_points: Vec<egui::Pos2> =
                    polyline.0.iter().map(|p| projection.project(*p)).collect();
                painter.add(egui::Shape::line(screen_points, self.stroke));
            }
        }
    }
}

/// Splits a polyline into multiple polylines based on whether segments are within the erase radius.
fn split_polyline_by_erase_circle(
    polyline: &[GeoPos],
    pointer_pos: Pos2,
    erase_radius_sq: f32,
    projection: &MapProjection,
) -> Vec<Vec<GeoPos>> {
    if polyline.len() < 2 {
        return vec![];
    }

    let screen_points: Vec<Pos2> = polyline.iter().map(|p| projection.project(*p)).collect();

    let mut new_polylines = Vec::new();
    let mut current_line = Vec::new();
    let mut in_visible_part = true;

    // Check if the first segment is erased to correctly set initial state.
    if dist_sq_to_segment(pointer_pos, screen_points[0], screen_points[1]) < erase_radius_sq {
        in_visible_part = false;
    } else {
        current_line.push(polyline[0]);
    }

    for i in 0..(polyline.len() - 1) {
        let p2_geo = polyline[i + 1];
        let p1_screen = screen_points[i];
        let p2_screen = screen_points[i + 1];

        let segment_is_erased =
            dist_sq_to_segment(pointer_pos, p1_screen, p2_screen) < erase_radius_sq;

        if in_visible_part {
            if segment_is_erased {
                // Transition from visible to erased.
                let t = projection_factor(pointer_pos, p1_screen, p2_screen);
                let split_point_screen = p1_screen.lerp(p2_screen, t);
                let split_point_geo = projection.unproject(split_point_screen);
                current_line.push(split_point_geo);

                if current_line.len() > 1 {
                    new_polylines.push(std::mem::take(&mut current_line));
                }
                in_visible_part = false;
            } else {
                // Continue visible part.
                current_line.push(p2_geo);
            }
        } else {
            // In erased part
            if !segment_is_erased {
                // Transition from erased to visible.
                let t = projection_factor(pointer_pos, p1_screen, p2_screen);
                let split_point_screen = p1_screen.lerp(p2_screen, t);
                let split_point_geo = projection.unproject(split_point_screen);

                // Start new line.
                current_line.push(split_point_geo);
                current_line.push(p2_geo);
                in_visible_part = true;
            }
            // Continue in erased part, do nothing.
        }
    }

    if current_line.len() > 1 {
        new_polylines.push(current_line);
    }

    new_polylines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_layer_new() {
        let layer = DrawingLayer::default();
        assert_eq!(layer.draw_mode, DrawMode::Disabled);
        assert!(layer.polylines.is_empty());
    }

    #[test]
    fn drawing_layer_as_any() {
        let layer = DrawingLayer::default();
        assert!(layer.as_any().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_as_any_mut() {
        let mut layer = DrawingLayer::default();
        assert!(layer.as_any_mut().is::<DrawingLayer>());
    }

    #[test]
    fn drawing_layer_serde() {
        let mut layer = DrawingLayer::default();
        layer.draw_mode = DrawMode::Draw; // This should not be serialized.
        layer.polylines.push(Polyline(vec![
            GeoPos { lon: 1.0, lat: 2.0 },
            GeoPos { lon: 3.0, lat: 4.0 },
        ]));
        layer.stroke = Stroke::new(5.0, Color32::BLUE); // This should not be serialized.

        let json = serde_json::to_string(&layer).unwrap();

        // The serialized string should only contain polylines.
        assert!(json.contains(r##""polylines":[[{"lon":1.0,"lat":2.0},{"lon":3.0,"lat":4.0}]],"stroke":{"width":5.0,"color":"#0000ffff"}"##));
        assert!(!json.contains("draw_mode"));

        let deserialized: DrawingLayer = serde_json::from_str(&json).unwrap();

        // Check that polylines are restored correctly.
        assert_eq!(deserialized.polylines, layer.polylines);

        // Check that the stroke information is correct
        assert_eq!(deserialized.stroke.width, 5.0);
        assert_eq!(deserialized.stroke.color, Color32::BLUE);

        // Default is drawmode disabled and its not serializable
        assert_eq!(deserialized.draw_mode, DrawMode::Disabled);
    }

    #[cfg(feature = "geojson")]
    mod geojson_tests {
        use super::*;

        #[test]
        fn drawing_layer_geojson() {
            let mut layer = DrawingLayer::default();
            layer.polylines.push(Polyline(vec![
                (10.0, 20.0).into(),
                (30.0, 40.0).into(),
                (50.0, 60.0).into(),
            ]));
            layer.stroke = Stroke::new(5.0, Color32::BLUE);

            let geojson_str = layer.to_geojson_str("my_layer").unwrap();

            // Test deserialization with matching ID
            let mut new_layer = DrawingLayer::default();
            new_layer
                .from_geojson_str(&geojson_str, Some("my_layer"))
                .unwrap();

            assert_eq!(new_layer.polylines.len(), 1);
            assert_eq!(layer.polylines[0], new_layer.polylines[0]);
            assert_eq!(layer.stroke, new_layer.stroke);

            // Test deserialization with non-matching ID
            let mut other_layer = DrawingLayer::default();
            other_layer
                .from_geojson_str(&geojson_str, Some("other_layer"))
                .unwrap();
            assert_eq!(other_layer.polylines.len(), 0);

            // Test deserialization with None ID (should include all)
            let mut all_layer = DrawingLayer::default();
            all_layer.from_geojson_str(&geojson_str, None).unwrap();
            assert_eq!(all_layer.polylines.len(), 1);
        }
    }
}
