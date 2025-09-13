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

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    const EPSILON: f32 = 1e-6;

    #[test]
    fn test_dist_sq_to_segment() {
        let a = pos2(0.0, 0.0);
        let b = pos2(10.0, 0.0);

        // Point on the segment
        let p1 = pos2(5.0, 0.0);
        assert!((dist_sq_to_segment(p1, a, b) - 0.0).abs() < EPSILON);

        // Point off the segment, projection is on the segment
        let p2 = pos2(5.0, 5.0);
        assert!((dist_sq_to_segment(p2, a, b) - 25.0).abs() < EPSILON); // 5*5

        // Point off the segment, projection is before 'a'
        let p3 = pos2(-5.0, 5.0);
        assert!((dist_sq_to_segment(p3, a, b) - 50.0).abs() < EPSILON); // dist^2 from (-5,5) to (0,0) is 25+25 = 50

        // Point off the segment, projection is after 'b'
        let p4 = pos2(15.0, 5.0);
        assert!((dist_sq_to_segment(p4, a, b) - 50.0).abs() < EPSILON); // dist^2 from (15,5) to (10,0) is 25+25 = 50

        // Zero-length segment
        let c = pos2(5.0, 5.0);
        let p5 = pos2(10.0, 10.0);
        assert!((dist_sq_to_segment(p5, c, c) - 50.0).abs() < EPSILON); // dist^2 from (10,10) to (5,5) is 25+25 = 50
    }

    #[test]
    fn test_projection_factor() {
        let a = pos2(0.0, 0.0);
        let b = pos2(10.0, 0.0);

        // Point is 'a'
        assert!((projection_factor(a, a, b) - 0.0).abs() < EPSILON);

        // Point is 'b'
        assert!((projection_factor(b, a, b) - 1.0).abs() < EPSILON);

        // Point is midpoint
        let p1 = pos2(5.0, 0.0);
        assert!((projection_factor(p1, a, b) - 0.5).abs() < EPSILON);

        // Point projects to midpoint
        let p2 = pos2(5.0, 5.0);
        assert!((projection_factor(p2, a, b) - 0.5).abs() < EPSILON);

        // Point projects before 'a' (clamped)
        let p3 = pos2(-5.0, 5.0);
        assert!((projection_factor(p3, a, b) - 0.0).abs() < EPSILON);

        // Point projects after 'b' (clamped)
        let p4 = pos2(15.0, 5.0);
        assert!((projection_factor(p4, a, b) - 1.0).abs() < EPSILON);

        // Zero-length segment
        let c = pos2(5.0, 5.0);
        let p5 = pos2(10.0, 10.0);
        assert!((projection_factor(p5, c, c) - 0.0).abs() < EPSILON);
    }
}
