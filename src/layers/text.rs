//! A layer for placing text on the map.

use crate::layers::{Layer, default_opacity, serde_color32};
use crate::projection::{GeoPos, MapProjection};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Response};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// The size of the text.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TextSize {
    /// Size is in screen points, and does not scale with zoom.
    Static(f32),

    /// Size is in meters at the equator, and scales with zoom.
    Relative(f32),
}

impl Default for TextSize {
    fn default() -> Self {
        // A reasonable default.
        Self::Static(12.0)
    }
}

/// A piece of text on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text {
    /// The unique identifier of the text.
    #[serde(default = "uuid::Uuid::new_v4")]
    pub id: uuid::Uuid,

    /// The text to display.
    pub text: String,

    /// The geographical position of the text.
    pub pos: GeoPos,

    /// The size of the text.
    pub size: TextSize,

    /// The color of the text.
    #[serde(with = "serde_color32")]
    pub color: Color32,

    /// The color of the background.
    #[serde(with = "serde_color32")]
    pub background: Color32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            text: "New Text".to_string(),
            pos: GeoPos { lon: 0.0, lat: 0.0 }, // This will be updated on click.
            size: TextSize::default(),
            color: Color32::BLACK,
            background: Color32::from_rgba_unmultiplied(255, 255, 255, 180),
        }
    }
}

/// The state of the text currently being edited or added.
#[derive(Clone, Debug)]
pub struct EditingText {
    /// The unique identifier of the text being edited, if it's an existing one.
    pub id: Option<uuid::Uuid>,
    /// The properties of the text being edited.
    pub properties: Text,
}

/// The mode of the `TextLayer`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextLayerMode {
    /// The layer is not interactive.
    #[default]
    Disabled,
    /// The user can add, remove, and modify text elements.
    Modify,
}

/// Layer implementation that allows placing text on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextLayer {
    texts: Vec<Text>,

    /// The current mode.
    #[serde(skip)]
    pub mode: TextLayerMode,

    /// The properties for the next text to be added.
    #[serde(skip)]
    pub new_text_properties: Text,

    /// The state of the text currently being edited or added.
    #[serde(skip)]
    pub editing: Option<EditingText>,

    #[serde(skip)]
    dragged_text_index: Option<usize>,

    /// The opacity of the layer.
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

impl Default for TextLayer {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            mode: TextLayerMode::default(),
            new_text_properties: Text::default(),
            editing: None,
            dragged_text_index: None,
            opacity: 1.0,
        }
    }
}

impl TextLayer {
    /// Starts editing an existing text element.
    pub fn start_editing(&mut self, id: uuid::Uuid) {
        if let Some(text) = self.texts.iter().find(|t| t.id == id) {
            self.editing = Some(EditingText {
                id: Some(id),
                properties: text.clone(),
            });
        }
    }

    /// Deletes a text element.
    pub fn delete(&mut self, id: uuid::Uuid) {
        if let Some(pos) = self.texts.iter().position(|t| t.id == id) {
            self.texts.remove(pos);
        }
    }

    /// Saves the changes made in the editing dialog.
    pub fn commit_edit(&mut self) {
        if let Some(editing) = self.editing.take() {
            if let Some(id) = editing.id {
                // It's an existing text.
                if let Some(text) = self.texts.iter_mut().find(|t| t.id == id) {
                    *text = editing.properties;
                }
            } else {
                // It's a new text.
                self.texts.push(editing.properties);
            }
        }
    }

    /// Discards the changes made in the editing dialog.
    pub fn cancel_edit(&mut self) {
        self.editing = None;
    }

    /// Finds a text element by its ID.
    pub fn find_text(&self, id: uuid::Uuid) -> Option<&Text> {
        self.texts.iter().find(|t| t.id == id)
    }

    /// Finds a text element by its ID and returns a mutable reference.
    pub fn find_text_mut(&mut self, id: uuid::Uuid) -> Option<&mut Text> {
        self.texts.iter_mut().find(|t| t.id == id)
    }

    /// Removes a text element from the layer by its ID and returns it.
    pub fn remove_text(&mut self, id: uuid::Uuid) -> Option<Text> {
        if let Some(pos) = self.texts.iter().position(|t| t.id == id) {
            Some(self.texts.remove(pos))
        } else {
            None
        }
    }

    /// Serializes the layer to a `GeoJSON` `FeatureCollection`.
    #[cfg(feature = "geojson")]
    pub fn to_geojson_str(&self) -> Result<String, serde_json::Error> {
        let features: Vec<geojson::Feature> = self
            .texts
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
        let new_texts: Vec<Text> = feature_collection
            .features
            .into_iter()
            .filter_map(|f| Text::try_from(f).ok())
            .collect();
        self.texts.extend(new_texts);

        if let Some(foreign_members) = feature_collection.foreign_members
            && let Some(value) = foreign_members.get("opacity")
            && let Some(opacity) = value.as_f64()
        {
            self.opacity = opacity as f32;
        }
        Ok(())
    }

    fn handle_modify_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if self.editing.is_some() {
            // While editing in a dialog, we don't want to interact with the map.
            // We consume all hover events to prevent panning and zooming.
            return response.hovered();
        }

        if response.drag_started()
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            self.dragged_text_index = self.find_text_at(pointer_pos, projection, &response.ctx);
        }

        if response.dragged()
            && let Some(text_index) = self.dragged_text_index
            && let Some(text) = self.texts.get_mut(text_index)
            && let Some(pointer_pos) = response.interact_pointer_pos()
        {
            text.pos = projection.unproject(pointer_pos);
        }

        if response.drag_stopped() {
            self.dragged_text_index = None;
        }

        // Change cursor on hover
        if self.dragged_text_index.is_some() {
            response.ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if let Some(hover_pos) = response.hover_pos() {
            if self
                .find_text_at(hover_pos, projection, &response.ctx)
                .is_some()
            {
                response.ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
            } else {
                response.ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }

        if !response.dragged() && response.clicked() {
            // Left-click to add or edit a text element
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(index) = self.find_text_at(pointer_pos, projection, &response.ctx) {
                    // Clicked on an existing text, start editing it.
                    if let Some(text) = self.texts.get(index) {
                        self.start_editing(text.id);
                    }
                } else {
                    // Clicked on an empty spot, start adding a new text.
                    let geo_pos = projection.unproject(pointer_pos);
                    let mut properties = self.new_text_properties.clone();
                    properties.id = uuid::Uuid::new_v4();
                    properties.pos = geo_pos;
                    self.editing = Some(EditingText {
                        id: None,
                        properties,
                    });
                }
            }
        }

        response.hovered()
    }

    /// A more robust check that considers the text's bounding box.
    fn find_text_at(
        &self,
        screen_pos: Pos2,
        projection: &MapProjection,
        ctx: &egui::Context,
    ) -> Option<usize> {
        self.texts.iter().enumerate().rev().find_map(|(i, text)| {
            let text_rect = self.get_text_rect(text, projection, ctx);
            if text_rect.expand(5.0).contains(screen_pos) {
                // Add some tolerance
                Some(i)
            } else {
                None
            }
        })
    }
}

impl Layer for TextLayer {
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
            TextLayerMode::Disabled => false,
            TextLayerMode::Modify => self.handle_modify_input(response, projection),
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for text in &self.texts {
            let screen_pos = projection.project(text.pos);

            let galley = painter.layout_no_wrap(
                // We use the painter's layout function here for drawing.
                text.text.clone(),
                FontId::proportional(self.get_font_size(text, projection)),
                text.color.gamma_multiply(self.opacity),
            );

            let rect =
                Align2::CENTER_CENTER.anchor_rect(Rect::from_min_size(screen_pos, galley.size()));

            painter.rect_filled(
                rect.expand(2.0),
                3.0,
                text.background.gamma_multiply(self.opacity),
            );
            painter.galley(rect.min, galley, Color32::TRANSPARENT);
        }
    }
}

impl TextLayer {
    fn get_font_size(&self, text: &Text, projection: &MapProjection) -> f32 {
        match text.size {
            TextSize::Static(size) => size,
            TextSize::Relative(size_in_meters) => {
                let p2 = projection.project(GeoPos {
                    lon: text.pos.lon
                        + (f64::from(size_in_meters)
                            / (111_320.0 * text.pos.lat.to_radians().cos())),
                    lat: text.pos.lat,
                });
                (p2.x - projection.project(text.pos).x).abs()
            }
        }
    }

    fn get_text_rect(&self, text: &Text, projection: &MapProjection, ctx: &egui::Context) -> Rect {
        let font_size = self.get_font_size(text, projection);
        let galley = ctx
            .debug_painter()
            .layout_job(egui::text::LayoutJob::simple(
                text.text.clone(),
                FontId::proportional(font_size),
                text.color,
                f32::INFINITY,
            ));
        let screen_pos = projection.project(text.pos);
        Align2::CENTER_CENTER.anchor_rect(Rect::from_min_size(screen_pos, galley.size()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_layer_serde() {
        let mut layer = TextLayer {
            mode: TextLayerMode::Modify,
            ..TextLayer::default()
        };
        layer.texts.push(Text {
            text: "Hello".to_string(),
            pos: GeoPos { lon: 1.0, lat: 2.0 },
            size: TextSize::Static(14.0),
            color: Color32::from_rgb(0, 0, 255),
            background: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
            ..Default::default()
        });

        let json = serde_json::to_string(&layer).unwrap();

        // The serialized string should contain the text properties.
        assert!(json.contains(r##""text":"Hello","pos":{"lon":1.0,"lat":2.0},"size":{"Static":14.0},"color":"#0000ffff","background":"#ff000080""##));

        // it should not contain skipped fields
        assert!(!json.contains("mode"));
        assert!(!json.contains("new_text_properties"));
        assert!(!json.contains("editing"));
        assert!(!json.contains("dragged_text_index"));

        let deserialized: TextLayer = serde_json::from_str(&json).unwrap();

        // Check that texts are restored correctly.
        assert_eq!(deserialized.texts.len(), 1);
        assert_eq!(deserialized.texts[0].text, "Hello");
        assert_eq!(deserialized.texts[0].pos, GeoPos { lon: 1.0, lat: 2.0 });
        assert_eq!(deserialized.texts[0].size, TextSize::Static(14.0));
        assert_eq!(deserialized.texts[0].color, Color32::from_rgb(0, 0, 255));
        assert_eq!(
            deserialized.texts[0].background,
            Color32::from_rgba_unmultiplied(255, 0, 0, 128)
        );

        // Check that skipped fields have their values from the `default()` implementation.
        let default_layer = TextLayer::default();
        assert_eq!(deserialized.mode, default_layer.mode);
        let mut expected_properties = default_layer.new_text_properties.clone();
        expected_properties.id = deserialized.new_text_properties.id;
        assert_eq!(deserialized.new_text_properties, expected_properties);
        assert!(deserialized.editing.is_none());
        assert!(deserialized.dragged_text_index.is_none());
    }

    #[cfg(feature = "geojson")]
    mod geojson_tests {
        use super::*;

        #[test]
        fn text_layer_geojson() {
            let mut layer = TextLayer::default();
            layer.texts.push(Text {
                text: "Hello".to_string(),
                pos: (10.0, 20.0).into(),
                size: TextSize::Static(14.0),
                color: Color32::from_rgb(0, 0, 255),
                background: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
                ..Default::default()
            });

            let geojson_str = layer.to_geojson_str().unwrap();
            let mut new_layer = TextLayer::default();
            new_layer.from_geojson_str(&geojson_str).unwrap();

            assert_eq!(new_layer.texts.len(), 1);
            assert_eq!(layer.texts[0], new_layer.texts[0]);
        }
    }
}
