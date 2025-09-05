//! A layer for placing text on the map.

use crate::layers::Layer;
use crate::projection::{GeoPos, MapProjection};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Response};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A helper module for serializing `egui::Color32`.
mod ser_color {
    use egui::Color32;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = color.to_hex();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if !s.starts_with('#') {
            return Err(serde::de::Error::custom("hex color must start with '#'"));
        }
        let s = &s[1..];
        let (r, g, b, a) = match s.len() {
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(serde::de::Error::custom)?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(serde::de::Error::custom)?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(serde::de::Error::custom)?;
                (r, g, b, 255)
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).map_err(serde::de::Error::custom)?;
                let g = u8::from_str_radix(&s[2..4], 16).map_err(serde::de::Error::custom)?;
                let b = u8::from_str_radix(&s[4..6], 16).map_err(serde::de::Error::custom)?;
                let a = u8::from_str_radix(&s[6..8], 16).map_err(serde::de::Error::custom)?;
                (r, g, b, a)
            }
            _ => {
                return Err(serde::de::Error::custom("invalid hex color length"));
            }
        };
        Ok(Color32::from_rgba_unmultiplied(r, g, b, a))
    }
}

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
    /// The text to display.
    pub text: String,

    /// The geographical position of the text.
    pub pos: GeoPos,

    /// The size of the text.
    pub size: TextSize,

    /// The color of the text.
    #[serde(with = "ser_color")]
    pub color: Color32,

    /// The color of the background.
    #[serde(with = "ser_color")]
    pub background: Color32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
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
    /// The index of the text being edited, if it's an existing one.
    pub index: Option<usize>,
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
}

impl Default for TextLayer {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            mode: TextLayerMode::default(),
            new_text_properties: Text::default(),
            editing: None,
            dragged_text_index: None,
        }
    }
}

impl TextLayer {
    /// Starts editing an existing text element.
    pub fn start_editing(&mut self, index: usize) {
        if let Some(text) = self.texts.get(index) {
            self.editing = Some(EditingText {
                index: Some(index),
                properties: text.clone(),
            });
        }
    }

    /// Deletes a text element.
    pub fn delete(&mut self, index: usize) {
        if index < self.texts.len() {
            self.texts.remove(index);
        }
    }

    /// Saves the changes made in the editing dialog.
    pub fn commit_edit(&mut self) {
        if let Some(editing) = self.editing.take() {
            if let Some(index) = editing.index {
                // It's an existing text.
                if let Some(text) = self.texts.get_mut(index) {
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

    fn handle_modify_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        if self.editing.is_some() {
            // While editing in a dialog, we don't want to interact with the map.
            // We consume all hover events to prevent panning and zooming.
            return response.hovered();
        }

        if response.drag_started() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.dragged_text_index = self.find_text_at(pointer_pos, projection, &response.ctx);
            }
        }

        if response.dragged() {
            if let Some(text_index) = self.dragged_text_index {
                if let Some(text) = self.texts.get_mut(text_index) {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        text.pos = projection.unproject(pointer_pos);
                    }
                }
            }
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
                    self.start_editing(index);
                } else {
                    // Clicked on an empty spot, start adding a new text.
                    let geo_pos = projection.unproject(pointer_pos);
                    let mut properties = self.new_text_properties.clone();
                    properties.pos = geo_pos;
                    self.editing = Some(EditingText {
                        index: None,
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
                text.color,
            );

            let rect =
                Align2::CENTER_CENTER.anchor_rect(Rect::from_min_size(screen_pos, galley.size()));

            painter.rect_filled(rect.expand(2.0), 3.0, text.background);
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
                        + (size_in_meters as f64 / (111_320.0 * text.pos.lat.to_radians().cos())),
                    lat: text.pos.lat,
                });
                (p2.x - projection.project(text.pos).x).abs()
            }
        }
    }

    fn get_text_rect(&self, text: &Text, projection: &MapProjection, ctx: &egui::Context) -> Rect {
        let font_size = self.get_font_size(text, projection);
        let galley = ctx.fonts(|f| {
            f.layout_no_wrap(
                text.text.clone(),
                FontId::proportional(font_size),
                text.color,
            )
        });
        let screen_pos = projection.project(text.pos);
        Align2::CENTER_CENTER.anchor_rect(Rect::from_min_size(screen_pos, galley.size()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_layer_serde() {
        let mut layer = TextLayer::default();
        layer.mode = TextLayerMode::Modify; // This should not be serialized.
        layer.texts.push(Text {
            text: "Hello".to_string(),
            pos: GeoPos { lon: 1.0, lat: 2.0 },
            size: TextSize::Static(14.0),
            color: Color32::from_rgb(0, 0, 255),
            background: Color32::from_rgba_unmultiplied(255, 0, 0, 128),
        });

        let json = serde_json::to_string(&layer).unwrap();

        // The serialized string should only contain texts.
        assert!(json.contains(r##""texts":[{"text":"Hello","pos":{"lon":1.0,"lat":2.0},"size":{"Static":14.0},"color":"#0000ffff","background":"#ff000080""##));

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
        assert_eq!(
            deserialized.new_text_properties,
            default_layer.new_text_properties
        );
        assert!(deserialized.editing.is_none());
        assert!(deserialized.dragged_text_index.is_none());
    }
}
