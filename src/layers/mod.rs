//! Layers for the map view that can handle input, and draw on top of the map view different kinds of data.
//!
use egui::{Painter, Pos2, Response};
use std::any::Any;

use crate::projection::MapProjection;

/// Drawing layer
#[cfg(feature = "drawing-layer")]
pub mod drawing;

/// Text layer
#[cfg(feature = "text-layer")]
pub mod text;

/// Area layer
#[cfg(feature = "area-layer")]
pub mod area;

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

/// Calculates the squared distance from a point to a line segment.
pub fn dist_sq_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let l2 = ab.length_sq();

    if l2 == 0.0 {
        // The segment is a point.
        return ap.length_sq();
    }

    // Project point p onto the line defined by a and b.
    // `t` is the normalized distance from a to the projection.
    let t = (ap.dot(ab) / l2).clamp(0.0, 1.0);

    // The closest point on the line segment.
    let closest_point = a + t * ab;

    p.distance_sq(closest_point)
}

/// Calculates the projection factor of a point onto a line segment.
/// Returns a value `t` from 0.0 to 1.0.
pub fn projection_factor(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = p - a;
    let l2 = ab.length_sq();

    if l2 == 0.0 {
        return 0.0;
    }

    // Project point p onto the line defined by a and b.
    (ap.dot(ab) / l2).clamp(0.0, 1.0)
}
