//! A layer for placing text on the map.

use crate::layers::Layer;
use crate::projection::{GeoPos, MapProjection};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Response};
use std::any::Any;

/// The size of the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextSize {
    /// Size is in screen points, and does not scale with zoom.
    Static(f32),

    /// Size is in meters at the equator, and scales with zoom.
    Relative(f32),
}

/// A piece of text on the map.
#[derive(Clone, Debug)]
pub struct Text {
    /// The text to display.
    pub text: String,

    /// The geographical position of the text.
    pub pos: GeoPos,

    /// The size of the text.
    pub size: TextSize,

    /// The color of the text.
    pub color: Color32,

    /// The color of the background.
    pub background: Color32,
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
#[derive(Clone)]
pub struct TextLayer {
    texts: Vec<Text>,

    /// The current mode.
    pub mode: TextLayerMode,

    /// The properties for the next text to be added.
    pub new_text_properties: Text,

    /// The state of the text currently being edited or added.
    pub editing: Option<EditingText>,

    /// The index of the last right-clicked text element.
    pub last_right_clicked_index: Option<usize>,

    dragged_text_index: Option<usize>,
}

impl Default for TextLayer {
    fn default() -> Self {
        Self {
            texts: Vec::new(),
            mode: TextLayerMode::default(),
            new_text_properties: Text {
                text: "New Text".to_string(),
                pos: GeoPos { lon: 0.0, lat: 0.0 }, // This will be updated on click.
                size: TextSize::Static(12.0),
                color: Color32::BLACK,
                background: Color32::from_rgba_premultiplied(255, 255, 255, 180),
            },
            editing: None,
            last_right_clicked_index: None,
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
            // But we do want to consume drags on the map to prevent panning.
            return response.dragged();
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

        if !response.dragged() {
            // Right-click to open context menu
            if response.secondary_clicked() {
                if let Some(pointer_pos) = response.interact_pointer_pos() {
                    self.last_right_clicked_index =
                        self.find_text_at(pointer_pos, projection, &response.ctx);
                }
            }

            // Left-click to add or edit a text element
            if response.clicked() {
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
