//! Map projection.

use egui::Rect;

use crate::{TILE_SIZE, lat_to_y, lon_to_x, x_to_lon, y_to_lat};

/// A helper for converting between geographical and screen coordinates.
pub struct MapProjection {
    zoom: u8,
    center_lon: f64,
    center_lat: f64,
    widget_rect: Rect,
}

impl MapProjection {
    /// Creates a new `MapProjection`.
    pub(crate) fn new(zoom: u8, center: (f64, f64), widget_rect: Rect) -> Self {
        Self {
            zoom,
            center_lon: center.0,
            center_lat: center.1,
            widget_rect,
        }
    }

    /// Projects a geographical coordinate to a screen coordinate.
    pub fn project(&self, geo_pos: (f64, f64)) -> egui::Pos2 {
        let center_x = lon_to_x(self.center_lon, self.zoom);
        let center_y = lat_to_y(self.center_lat, self.zoom);

        let tile_x = lon_to_x(geo_pos.0, self.zoom);
        let tile_y = lat_to_y(geo_pos.1, self.zoom);

        let dx = (tile_x - center_x) * TILE_SIZE as f64;
        let dy = (tile_y - center_y) * TILE_SIZE as f64;

        let widget_center = self.widget_rect.center();
        widget_center + egui::vec2(dx as f32, dy as f32)
    }

    /// Un-projects a screen coordinate to a geographical coordinate.
    pub fn unproject(&self, screen_pos: egui::Pos2) -> (f64, f64) {
        let rel_pos = screen_pos - self.widget_rect.min;
        let widget_center_x = self.widget_rect.width() as f64 / 2.0;
        let widget_center_y = self.widget_rect.height() as f64 / 2.0;

        let center_x = lon_to_x(self.center_lon, self.zoom);
        let center_y = lat_to_y(self.center_lat, self.zoom);

        let target_x = center_x + (rel_pos.x as f64 - widget_center_x) / TILE_SIZE as f64;
        let target_y = center_y + (rel_pos.y as f64 - widget_center_y) / TILE_SIZE as f64;

        (x_to_lon(target_x, self.zoom), y_to_lat(target_y, self.zoom))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{pos2, vec2};

    const EPSILON: f64 = 1e-9;

    fn create_projection() -> MapProjection {
        MapProjection::new(
            10,
            (24.93545, 60.16952), // Helsinki
            Rect::from_min_size(pos2(100.0, 200.0), vec2(800.0, 600.0)),
        )
    }

    #[test]
    fn project_center() {
        let projection = create_projection();
        let center_geo = (projection.center_lon, projection.center_lat);
        let projected_center = projection.project(center_geo);
        assert_eq!(projected_center, projection.widget_rect.center());
    }

    #[test]
    fn unproject_center() {
        let projection = create_projection();
        let center_screen = projection.widget_rect.center();
        let (lon, lat) = projection.unproject(center_screen);
        assert!((lon - projection.center_lon).abs() < EPSILON);
        assert!((lat - projection.center_lat).abs() < EPSILON);
    }

    #[test]
    fn project_unproject_roundtrip() {
        let projection = create_projection();
        let geo_pos_in = (24.93545, 60.16952); // Some point near Helsinki

        let screen_pos = projection.project(geo_pos_in);
        let geo_pos_out = projection.unproject(screen_pos);

        println!("{}", EPSILON);
        println!("Input Geo: {:?}", geo_pos_in);
        println!("Output Geo: {:?}", geo_pos_out);
        println!("{}", (geo_pos_in.0 - geo_pos_out.0).abs());
        assert!((geo_pos_in.0 - geo_pos_out.0).abs() < EPSILON);
        println!("{}", (geo_pos_in.1 - geo_pos_out.1).abs());
        assert!((geo_pos_in.1 - geo_pos_out.1).abs() < EPSILON);
    }

    #[test]
    fn unproject_project_roundtrip() {
        let projection = create_projection();
        let screen_pos_in = pos2(150.0, 250.0); // Some point on the widget

        let geo_pos = projection.unproject(screen_pos_in);
        let screen_pos_out = projection.project(geo_pos);

        assert!((screen_pos_in.x - screen_pos_out.x).abs() < 1e-3); // f32 precision
        assert!((screen_pos_in.y - screen_pos_out.y).abs() < 1e-3);
    }
}
