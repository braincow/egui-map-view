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
