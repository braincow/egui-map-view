use crate::layers::{default_opacity, Layer};
use crate::projection::MapProjection;
use egui::{Color32, Painter, Response};
use serde::{Deserialize, Serialize};
use std::any::Any;

use super::types::{Area, AreaMode, DraggedObject};

/// Layer implementation that allows the user to draw polygons on the map.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaLayer {
    pub(crate) areas: Vec<Area>,

    #[serde(skip)]
    /// The radius of the nodes.
    pub node_radius: f32,

    #[serde(skip)]
    /// The fill color of the nodes.
    pub node_fill: Color32,

    #[serde(skip)]
    /// The current drawing mode.
    pub mode: AreaMode,

    #[serde(skip)]
    pub(crate) dragged_object: Option<DraggedObject>,

    #[serde(skip)]
    pub(crate) hovered_object: Option<DraggedObject>,

    /// The opacity of the layer.
    #[serde(default = "default_opacity")]
    pub opacity: f32,

    #[serde(skip)]
    /// The unique identifier of the currently selected area. Only used when in `AreaMode::ModifySelected`.
    pub selected_area: Option<uuid::Uuid>,
}

impl Default for AreaLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AreaLayer {
    /// Creates a new `AreaLayer`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            areas: Vec::new(),
            node_radius: 5.0,
            node_fill: Color32::from_rgb(0, 128, 0),
            mode: AreaMode::default(),
            dragged_object: None,
            hovered_object: None,
            opacity: 1.0,
            selected_area: None,
        }
    }

    /// Adds a new area to the layer.
    pub fn add_area(&mut self, area: Area) {
        self.areas.push(area);
    }

    /// Returns a reference to the areas in the layer.
    #[must_use]
    pub fn areas(&self) -> &Vec<Area> {
        &self.areas
    }

    /// Returns a mutable reference to the areas in the layer.
    pub fn areas_mut(&mut self) -> &mut Vec<Area> {
        &mut self.areas
    }

    /// Finds an area in the layer by its ID.
    pub fn find_area(&self, id: uuid::Uuid) -> Option<&Area> {
        self.areas.iter().find(|a| a.id == id)
    }

    /// Finds an area in the layer by its ID and returns a mutable reference.
    pub fn find_area_mut(&mut self, id: uuid::Uuid) -> Option<&mut Area> {
        self.areas.iter_mut().find(|a| a.id == id)
    }

    /// Removes an area from the layer by its ID and returns it.
    pub fn remove_area(&mut self, id: uuid::Uuid) -> Option<Area> {
        if let Some(pos) = self.areas.iter().position(|a| a.id == id) {
            Some(self.areas.remove(pos))
        } else {
            None
        }
    }
}

impl Layer for AreaLayer {
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
            AreaMode::Disabled => {
                self.hovered_object = None;
                false
            }
            AreaMode::Modify => self.handle_modify_input(response, projection, None),
            AreaMode::ModifySelected => {
                if response.clicked()
                    && let Some(pointer_pos) = response.interact_pointer_pos()
                {
                    // Find if any area was clicked to select it.
                    let clicked_area_id =
                        self.areas.iter().enumerate().rev().find_map(|(idx, area)| {
                            let contains_fill = area.contains(pointer_pos, projection);
                            let over_handle = self.find_object_at(pointer_pos, projection, Some(idx)).is_some();
                            let over_segment = self.find_line_segment_at(pointer_pos, projection, Some(idx)).is_some();

                            if contains_fill || over_handle || over_segment {
                                Some(area.id)
                            } else {
                                None
                            }
                        });

                    if clicked_area_id != self.selected_area {
                        self.selected_area = clicked_area_id;
                        return true;
                    }
                }

                if let Some(selected_id) = self.selected_area {
                    if let Some(selected_idx) = self.areas.iter().position(|a| a.id == selected_id) {
                        self.handle_modify_input(response, projection, Some(selected_idx))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }

    fn draw(&self, painter: &Painter, projection: &MapProjection) {
        self.draw_layer(painter, projection);
    }
}
