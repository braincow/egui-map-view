//! Map layers.
//!
use egui::{Painter, Response};
use std::any::Any;

use crate::projection::MapProjection;

/// Drawing layer
pub mod drawing;

/// A trait for map layers.
pub trait Layer: Any {
    /// Handles user input for the layer. Returns `true` if the input was handled and should not be
    /// processed further by the map.
    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool;

    /// Draws the layer.
    fn draw(&self, painter: &Painter, projection: &MapProjection);

    /// Gets the layer as a `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Gets the layer as a mutable `dyn Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
