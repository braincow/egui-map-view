//! A layer for placing SVG elements on the map.

use crate::layers::{Layer, default_opacity};
use crate::projection::{GeoPos, MapProjection};
use egui::{Color32, Painter, PointerButton, Pos2, Response};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// An SVG element on the map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SvgElement {
    /// The geographical position (longitude, latitude) of the SVG element.
    pub pos: GeoPos,

    /// The SVG content string.
    pub text: String,

    /// Arbitrary metadata string, not rendered.
    pub metadata: String,

    /// Whether the SVG element should scale with the zoom level.
    /// If false, the image size stays the same in screen pixels.
    /// If true, the image size scales with the map.
    pub scalable: bool,

    /// Whether the SVG element is clickable.
    /// If true, click events will be emitted for this element.
    /// If false, no click events will be emitted.
    #[serde(default = "default_true")]
    pub clickable: bool,

    /// Whether the SVG element is draggable.
    /// If true, the element can be moved on the map by dragging it with the mouse.
    #[serde(default)]
    pub draggable: bool,

    /// The anchor point of the SVG element, relative to its size.
    /// (0.5, 0.5) is the center (default).
    /// (0.0, 0.0) is the top-left.
    /// (1.0, 1.0) is the bottom-right.
    #[serde(default = "default_anchor")]
    pub anchor: Pos2,
}

fn default_anchor() -> Pos2 {
    Pos2::new(0.5, 0.5)
}

fn default_true() -> bool {
    true
}

impl SvgElement {
    /// Creates a new SVG element.
    pub fn new(pos: GeoPos, text: impl Into<String>, metadata: impl Into<String>) -> Self {
        Self {
            pos,
            text: text.into(),
            metadata: metadata.into(),
            scalable: false,
            clickable: true,
            draggable: false,
            anchor: default_anchor(),
        }
    }

    /// Creates a new SVG element from x (longitude) and y (latitude) coordinates.
    pub fn from_xy(
        lon: f64,
        lat: f64,
        text: impl Into<String>,
        metadata: impl Into<String>,
    ) -> Self {
        Self {
            pos: GeoPos { lon, lat },
            text: text.into(),
            metadata: metadata.into(),
            scalable: false,
            clickable: true,
            draggable: false,
            anchor: default_anchor(),
        }
    }

    /// Sets whether the SVG element is scalable.
    #[must_use]
    pub fn with_scalable(mut self, scalable: bool) -> Self {
        self.scalable = scalable;
        self
    }

    /// Sets whether the SVG element is clickable.
    #[must_use]
    pub fn with_clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    /// Sets whether the SVG element is draggable.
    #[must_use]
    pub fn with_draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Sets the anchor point of the SVG element.
    #[must_use]
    pub fn with_anchor(mut self, anchor: Pos2) -> Self {
        self.anchor = anchor;
        self
    }
}

/// Information about a click on an SVG element.
#[derive(Clone, Debug)]
pub struct SvgClickEvent {
    /// The button that was clicked.
    pub button: PointerButton,
    /// The metadata of the clicked SVG element.
    pub metadata: String,
    /// The geographical position where the click occurred.
    pub world_pos: GeoPos,
    /// The screen position where the click occurred.
    pub screen_pos: Pos2,
}

/// Layer implementation that allows placing multiple SVG elements on the map.
#[derive(Clone, Serialize, Deserialize)]
pub struct SvgLayer {
    /// The list of SVG elements.
    pub elements: Vec<SvgElement>,

    /// Click events that have occurred on the SVG elements.
    #[serde(skip)]
    pub events: Vec<SvgClickEvent>,

    /// The index of the element currently being dragged.
    #[serde(skip)]
    pub dragging_index: Option<usize>,

    /// The opacity of the layer.
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

impl Default for SvgLayer {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            events: Vec::new(),
            dragging_index: None,
            opacity: 1.0,
        }
    }
}

impl SvgLayer {
    /// Adds an SVG element to the layer.
    pub fn add_element(&mut self, element: SvgElement) {
        self.elements.push(element);
    }

    /// Clears all SVG elements from the layer.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Takes all click events from the layer, leaving it empty.
    pub fn take_events(&mut self) -> Vec<SvgClickEvent> {
        std::mem::take(&mut self.events)
    }
}

impl Layer for SvgLayer {
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
        // Ensure image loaders are installed
        egui_extras::install_image_loaders(&response.ctx);

        for element in &self.elements {
            let uri = format!("bytes://{}.svg", rust_hash(&element.text));
            // include_bytes ensures the data is available for the loaders
            response
                .ctx
                .include_bytes(uri, element.text.as_bytes().to_vec());
        }

        let mut handled = false;

        // Handle active dragging
        if let Some(index) = self.dragging_index {
            if response.dragged() {
                if let Some(pointer_pos) = response.interact_pointer_pos()
                    && let Some(element) = self.elements.get_mut(index)
                {
                    element.pos = projection.unproject(pointer_pos);
                    handled = true;
                    response.ctx.request_repaint();
                }
            } else {
                self.dragging_index = None;
            }
        }

        // Detect drag start or click
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            for (index, element) in self.elements.iter_mut().enumerate() {
                if !element.clickable && !element.draggable {
                    continue;
                }

                let screen_pos = projection.project(element.pos);
                let uri = format!("bytes://{}.svg", rust_hash(&element.text));

                if let Ok(egui::load::TexturePoll::Ready { texture }) = response
                    .ctx
                    .try_load_texture(&uri, egui::TextureOptions::default(), Default::default())
                {
                    let mut size = texture.size;

                    if element.scalable {
                        // Scale the size based on the zoom level.
                        let scale = 2.0_f32.powi(i32::from(projection.zoom) - 10);
                        size *= scale;
                    }

                    let rect = egui::Rect::from_min_size(
                        screen_pos - size * element.anchor.to_vec2(),
                        size,
                    );
                    if rect.contains(pointer_pos) {
                        // Check for drag start
                        if element.draggable && response.drag_started() {
                            self.dragging_index = Some(index);
                            handled = true;
                        }

                        // Check for clicks
                        if element.clickable && (response.clicked() || response.secondary_clicked())
                        {
                            let button = if response.secondary_clicked() {
                                PointerButton::Secondary
                            } else {
                                PointerButton::Primary
                            };

                            self.events.push(SvgClickEvent {
                                button,
                                metadata: element.metadata.clone(),
                                world_pos: projection.unproject(pointer_pos),
                                screen_pos: pointer_pos,
                            });
                            handled = true;
                        }
                    }
                }
            }
        }

        handled
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        for element in &self.elements {
            let screen_pos = projection.project(element.pos);
            let uri = format!("bytes://{}.svg", rust_hash(&element.text));

            match painter.ctx().try_load_texture(
                &uri,
                egui::TextureOptions::default(),
                Default::default(),
            ) {
                Ok(egui::load::TexturePoll::Ready { texture }) => {
                    let mut size = texture.size;

                    if element.scalable {
                        // Scale the size based on the zoom level.
                        // We use zoom level 10 as a reference where scale is 1.0.
                        let scale = 2.0_f32.powi(i32::from(projection.zoom) - 10);
                        size *= scale;
                    }

                    let rect = egui::Rect::from_min_size(
                        screen_pos - size * element.anchor.to_vec2(),
                        size,
                    );
                    painter.image(
                        texture.id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE.gamma_multiply(self.opacity),
                    );
                }
                _ => {
                    // Still loading or failed.
                    // We could draw a placeholder here if desired.
                    painter.ctx().request_repaint();
                }
            }
        }
    }
}

fn rust_hash(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_layer_serde() {
        let mut layer = SvgLayer::default();
        layer.add_element(SvgElement {
            pos: GeoPos { lon: 1.0, lat: 2.0 },
            text: "<svg></svg>".to_string(),
            metadata: "test metadata".to_string(),
            scalable: false,
            clickable: true,
            draggable: false,
            anchor: Pos2::new(0.5, 0.5),
        });

        let json = serde_json::to_string(&layer).unwrap();
        let deserialized: SvgLayer = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.elements.len(), 1);
        assert_eq!(deserialized.elements[0].text, "<svg></svg>");
        assert_eq!(deserialized.elements[0].metadata, "test metadata");
        assert_eq!(deserialized.elements[0].pos, GeoPos { lon: 1.0, lat: 2.0 });
        assert!(deserialized.elements[0].clickable);
        assert!(!deserialized.elements[0].draggable);
    }

    #[test]
    fn svg_layer_serde_backward_compatibility() {
        let json = r#"{
            "elements": [
                {
                    "pos": {"lon": 1.0, "lat": 2.0},
                    "text": "<svg></svg>",
                    "metadata": "test metadata",
                    "scalable": false
                }
            ]
        }"#;
        let deserialized: SvgLayer = serde_json::from_str(json).unwrap();
        assert!(deserialized.elements[0].clickable);
        assert!(!deserialized.elements[0].draggable);
    }

    #[test]
    fn svg_layer_clickable_false() {
        let mut layer = SvgLayer::default();
        layer.add_element(SvgElement {
            pos: GeoPos { lon: 1.0, lat: 2.0 },
            text: "<svg></svg>".to_string(),
            metadata: "test metadata".to_string(),
            scalable: false,
            clickable: false,
            draggable: false,
            anchor: default_anchor(),
        });

        let json = serde_json::to_string(&layer).unwrap();
        let deserialized: SvgLayer = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.elements[0].clickable);
    }

    #[test]
    fn svg_layer_draggable_true() {
        let mut layer = SvgLayer::default();
        layer.add_element(SvgElement {
            pos: GeoPos { lon: 1.0, lat: 2.0 },
            text: "<svg></svg>".to_string(),
            metadata: "test metadata".to_string(),
            scalable: false,
            clickable: false,
            draggable: true,
            anchor: default_anchor(),
        });

        let json = serde_json::to_string(&layer).unwrap();
        let deserialized: SvgLayer = serde_json::from_str(&json).unwrap();
        assert!(deserialized.elements[0].draggable);
    }
}
