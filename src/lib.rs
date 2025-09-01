#![warn(missing_docs)]

//! A simple map view widget for `egui`.
//!
//! This crate provides a `Map` widget that can be used to display a map from a tile server.
//! It supports panning, zooming, and displaying the current mouse position in geographical coordinates.
//!
//! # Example
//!
//! ```no_run
//! use eframe::egui;
//! use egui_map_view::{Map, config::OpenStreetMapConfig};
//!
//! struct MyApp {
//!     map: Map,
//! }
//!
//! impl Default for MyApp {
//!     fn default() -> Self {
//!         Self {
//!             map: Map::new(OpenStreetMapConfig::default()),
//!         }
//!     }
//! }
//!
//! impl eframe::App for MyApp {
//!     fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
//!         egui::CentralPanel::default()
//!             .frame(egui::Frame::NONE)
//!             .show(ctx, |ui| {
//!                 ui.add(&mut self.map);
//!             });
//!     }
//! }
//! ```

/// Configuration traits and types for the map widget.
pub mod config;

use eframe::egui;
use egui::{Color32, Rect, Response, Sense, Ui, Vec2, Widget, pos2};
use eyre::{Context, Result};
use log::{debug, error};
use once_cell::sync::Lazy;
use poll_promise::Promise;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::config::MapConfig;

// The size of a map tile in pixels.
const TILE_SIZE: u32 = 256;
/// The minimum zoom level.
pub const MIN_ZOOM: u8 = 0;
/// The maximum zoom level.
pub const MAX_ZOOM: u8 = 19;

// Reuse the reqwest client for all tile downloads by making it a static variable.
static CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
    reqwest::blocking::Client::builder()
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .expect("Failed to build reqwest client")
});

/// Errors that can occur while using the map widget.
#[derive(Error, Debug)]
pub enum MapError {
    /// An error occurred while making a web request.
    #[error("Connection error")]
    ConnectionError(#[from] reqwest::Error),

    /// A map tile failed to download.
    #[error("A map tile failed to download. HTTP Status: `{0}`")]
    TileDownloadError(String),

    /// The downloaded tile bytes could not be converted to an image.
    #[error("Unable to convert downloaded map tile bytes as image")]
    TileBytesConversionError(#[from] image::ImageError),
}

/// A unique identifier for a map tile.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct TileId {
    /// The zoom level.
    pub z: u8,

    /// The x-coordinate of the tile.
    pub x: u32,

    /// The y-coordinate of the tile.
    pub y: u32,
}

impl TileId {
    fn to_url(&self, config: &dyn MapConfig) -> String {
        config.tile_url(self)
    }
}

/// The state of a tile in the cache.
enum Tile {
    /// The tile is being downloaded.
    Loading(Promise<Result<egui::ColorImage, Arc<eyre::Report>>>),

    /// The tile is in memory.
    Loaded(egui::TextureHandle),

    /// The tile failed to download.
    Failed(Arc<eyre::Report>),
}

/// The map widget.
pub struct Map {
    /// The geographical center of the map. (longitude, latitude)
    pub center: (f64, f64),

    /// The zoom level of the map.
    pub zoom: u8,

    tiles: HashMap<TileId, Tile>,

    /// The geographical position under the mouse pointer, if any. (longitude, latitude)
    pub mouse_pos: Option<(f64, f64)>,

    /// Configuration for the map, such as the tile server URL.
    config: Box<dyn MapConfig>,
}

impl Map {
    /// Creates a new `Map` widget.
    ///
    /// # Arguments
    ///
    /// * `config` - A type that implements `MapConfig`, which provides configuration for the map.
    pub fn new<C: MapConfig + 'static>(config: C) -> Self {
        let center = config.default_center();
        let zoom = config.default_zoom();
        Self {
            tiles: HashMap::new(),
            mouse_pos: None,
            config: Box::new(config),
            center,
            zoom,
        }
    }

    /// Handles user input for panning and zooming.
    fn handle_input(&mut self, ui: &Ui, rect: &Rect, response: Response) {
        // Handle panning
        if response.dragged() {
            let delta = response.drag_delta();
            let center_in_tiles_x = lon_to_x(self.center.0, self.zoom);
            let center_in_tiles_y = lat_to_y(self.center.1, self.zoom);

            let mut new_center_x = center_in_tiles_x - (delta.x as f64 / TILE_SIZE as f64);
            let mut new_center_y = center_in_tiles_y - (delta.y as f64 / TILE_SIZE as f64);

            // Clamp the new center to the map boundaries.
            let world_size_in_tiles = 2.0_f64.powi(self.zoom as i32);
            let view_size_in_tiles_x = rect.width() as f64 / TILE_SIZE as f64;
            let view_size_in_tiles_y = rect.height() as f64 / TILE_SIZE as f64;

            let min_center_x = view_size_in_tiles_x / 2.0;
            let max_center_x = world_size_in_tiles - view_size_in_tiles_x / 2.0;
            let min_center_y = view_size_in_tiles_y / 2.0;
            let max_center_y = world_size_in_tiles - view_size_in_tiles_y / 2.0;

            // If the map is smaller than the viewport, center it. Otherwise, clamp the center.
            new_center_x = if min_center_x > max_center_x {
                world_size_in_tiles / 2.0
            } else {
                new_center_x.clamp(min_center_x, max_center_x)
            };
            new_center_y = if min_center_y > max_center_y {
                world_size_in_tiles / 2.0
            } else {
                new_center_y.clamp(min_center_y, max_center_y)
            };

            self.center = (
                x_to_lon(new_center_x, self.zoom),
                y_to_lat(new_center_y, self.zoom),
            );
        }

        // Handle double-click to zoom and center
        if response.double_clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let new_zoom = (self.zoom + 1).clamp(MIN_ZOOM, MAX_ZOOM);

                if new_zoom != self.zoom {
                    // Determine the geo-coordinate under the mouse cursor before the zoom
                    let mouse_rel = pointer_pos - rect.min;
                    let center_x = lon_to_x(self.center.0, self.zoom);
                    let center_y = lat_to_y(self.center.1, self.zoom);
                    let widget_center_x = rect.width() as f64 / 2.0;
                    let widget_center_y = rect.height() as f64 / 2.0;

                    let target_x =
                        center_x + (mouse_rel.x as f64 - widget_center_x) / TILE_SIZE as f64;
                    let target_y =
                        center_y + (mouse_rel.y as f64 - widget_center_y) / TILE_SIZE as f64;

                    let new_center_lon = x_to_lon(target_x, self.zoom);
                    let new_center_lat = y_to_lat(target_y, self.zoom);

                    // Set the new zoom level and center the map on the clicked location
                    self.zoom = new_zoom;
                    self.center = (new_center_lon, new_center_lat);
                }
            }
        }

        // Handle zooming and mouse position
        if response.hovered() {
            if let Some(mouse_pos) = response.hover_pos() {
                let mouse_rel = mouse_pos - rect.min;

                // Determine the geo-coordinate under the mouse cursor.
                let center_x = lon_to_x(self.center.0, self.zoom);
                let center_y = lat_to_y(self.center.1, self.zoom);
                let widget_center_x = rect.width() as f64 / 2.0;
                let widget_center_y = rect.height() as f64 / 2.0;

                let target_x = center_x + (mouse_rel.x as f64 - widget_center_x) / TILE_SIZE as f64;
                let target_y = center_y + (mouse_rel.y as f64 - widget_center_y) / TILE_SIZE as f64;

                self.mouse_pos =
                    Some((x_to_lon(target_x, self.zoom), y_to_lat(target_y, self.zoom)));

                let scroll = ui.input(|i| i.raw_scroll_delta.y);
                if scroll != 0.0 {
                    let old_zoom = self.zoom;
                    let mut new_zoom = (self.zoom as i32 + scroll.signum() as i32)
                        .clamp(MIN_ZOOM as i32, MAX_ZOOM as i32)
                        as u8;

                    // If we are zooming out, check if the new zoom level is valid.
                    if scroll < 0.0 {
                        let world_pixel_size = 2.0_f64.powi(new_zoom as i32) * TILE_SIZE as f64;
                        // If the world size would become smaller than the widget size, reject the zoom.
                        if world_pixel_size < rect.width() as f64
                            || world_pixel_size < rect.height() as f64
                        {
                            new_zoom = old_zoom; // Effectively cancel the zoom by reverting to the old value.
                        }
                    }

                    if new_zoom != old_zoom {
                        let target_lon = x_to_lon(target_x, old_zoom);
                        let target_lat = y_to_lat(target_y, old_zoom);

                        // Set the new zoom level
                        self.zoom = new_zoom;

                        // Adjust the map center so the geo-coordinate under the mouse remains the
                        // same
                        let new_target_x = lon_to_x(target_lon, new_zoom);
                        let new_target_y = lat_to_y(target_lat, new_zoom);

                        let new_center_x = new_target_x
                            - (mouse_rel.x as f64 - widget_center_x) / TILE_SIZE as f64;
                        let new_center_y = new_target_y
                            - (mouse_rel.y as f64 - widget_center_y) / TILE_SIZE as f64;

                        self.center = (
                            x_to_lon(new_center_x, new_zoom),
                            y_to_lat(new_center_y, new_zoom),
                        );
                    }
                }
            } else {
                self.mouse_pos = None;
            }
        } else {
            self.mouse_pos = None;
        }
    }

    /// Draws the map tiles and attribution.
    fn draw_map_and_attribution(&mut self, ui: &mut Ui, rect: &Rect) {
        let painter = ui.painter_at(*rect);
        painter.rect_filled(*rect, 0.0, Color32::from_rgb(220, 220, 220)); // Background

        let visible_tiles: Vec<_> = self.visible_tiles(rect).collect();
        for (tile_id, tile_pos) in visible_tiles {
            self.draw_tile(ui, &painter, tile_id, tile_pos);
        }

        self.draw_attribution(ui, rect);
    }

    /// Returns an iterator over the visible tiles.
    fn visible_tiles(&self, rect: &Rect) -> impl Iterator<Item = (TileId, egui::Pos2)> {
        let center_x = lon_to_x(self.center.0, self.zoom);
        let center_y = lat_to_y(self.center.1, self.zoom);

        let widget_center_x = rect.width() / 2.0;
        let widget_center_y = rect.height() / 2.0;

        let x_min = (center_x - widget_center_x as f64 / TILE_SIZE as f64).floor() as i32;
        let y_min = (center_y - widget_center_y as f64 / TILE_SIZE as f64).floor() as i32;
        let x_max = (center_x + widget_center_x as f64 / TILE_SIZE as f64).ceil() as i32;
        let y_max = (center_y + widget_center_y as f64 / TILE_SIZE as f64).ceil() as i32;

        let zoom = self.zoom;
        let rect_min = rect.min;
        (x_min..=x_max).flat_map(move |x| {
            (y_min..=y_max).map(move |y| {
                let tile_id = TileId {
                    z: zoom,
                    x: x as u32,
                    y: y as u32,
                };
                let screen_x = widget_center_x + (x as f64 - center_x) as f32 * TILE_SIZE as f32;
                let screen_y = widget_center_y + (y as f64 - center_y) as f32 * TILE_SIZE as f32;
                let tile_pos = rect_min + Vec2::new(screen_x, screen_y);
                (tile_id, tile_pos)
            })
        })
    }

    /// Draws a single map tile.
    fn draw_tile(
        &mut self,
        ui: &mut Ui,
        painter: &egui::Painter,
        tile_id: TileId,
        tile_pos: egui::Pos2,
    ) {
        let tile_state = self.tiles.entry(tile_id).or_insert_with(|| {
            let url = tile_id.to_url(self.config.as_ref());
            let promise =
                Promise::spawn_thread("download_tile", move || -> Result<_, Arc<eyre::Report>> {
                    let result: Result<_, eyre::Report> = (|| {
                        debug!("Downloading tile from {}", &url);
                        let response = CLIENT.get(&url).send().map_err(MapError::from)?;

                        if !response.status().is_success() {
                            return Err(MapError::TileDownloadError(response.status().to_string()));
                        }

                        let bytes = response.bytes().map_err(MapError::from)?.to_vec();
                        let image = image::load_from_memory(&bytes)
                            .map_err(MapError::from)?
                            .to_rgba8();

                        let size = [image.width() as _, image.height() as _];
                        let pixels = image.into_raw();
                        Ok(egui::ColorImage::from_rgba_unmultiplied(size, &pixels))
                    })()
                    .with_context(|| format!("Failed to download tile from {}", &url));

                    result.map_err(Arc::new)
                });
            Tile::Loading(promise)
        });

        // If the tile is loading, check if the promise is ready and update the state.
        // This is done before matching on the state, so that we can immediately draw
        // the tile if it has just finished loading.
        if let Tile::Loading(promise) = tile_state {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(color_image) => {
                        let texture = ui.ctx().load_texture(
                            format!("tile_{}_{}_{}", tile_id.z, tile_id.x, tile_id.y),
                            color_image.clone(),
                            Default::default(),
                        );
                        *tile_state = Tile::Loaded(texture);
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        *tile_state = Tile::Failed(e.clone());
                    }
                }
            }
        }

        let tile_rect =
            Rect::from_min_size(tile_pos, Vec2::new(TILE_SIZE as f32, TILE_SIZE as f32));

        match tile_state {
            Tile::Loading(_) => {
                // Draw a gray background and a border for the placeholder.
                painter.rect_filled(tile_rect, 0.0, Color32::from_gray(220));
                painter.rect_stroke(
                    tile_rect,
                    0.0,
                    egui::Stroke::new(1.0, Color32::GRAY),
                    egui::StrokeKind::Inside,
                );

                // Draw a question mark in the center.
                painter.text(
                    tile_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "?",
                    egui::FontId::proportional(40.0),
                    Color32::ORANGE,
                );

                // The tile is still loading, so we need to tell egui to repaint.
                ui.ctx().request_repaint();
            }
            Tile::Loaded(texture) => {
                painter.image(
                    texture.id(),
                    tile_rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Tile::Failed(e) => {
                // Draw a gray background and a border for the placeholder.
                painter.rect_filled(tile_rect, 0.0, Color32::from_gray(220));
                painter.rect_stroke(
                    tile_rect,
                    0.0,
                    egui::Stroke::new(1.0, Color32::GRAY),
                    egui::StrokeKind::Inside,
                );

                // Draw a red exclamation mark in the center.
                painter.text(
                    tile_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "!",
                    egui::FontId::proportional(40.0),
                    Color32::RED,
                );

                let response = ui.interact(tile_rect, ui.id().with(tile_id), Sense::hover());
                response.on_hover_text(format!("{}", e));
            }
        }
    }

    /// Draws the attribution text.
    fn draw_attribution(&self, ui: &mut Ui, rect: &Rect) {
        if let Some(attribution) = self.config.attribution() {
            let (_text_color, bg_color) = if ui.visuals().dark_mode {
                (Color32::from_gray(230), Color32::from_black_alpha(150))
            } else {
                (Color32::from_gray(80), Color32::from_white_alpha(150))
            };

            let frame = egui::Frame::NONE
                .inner_margin(egui::Margin::same(5)) // A bit of padding
                .fill(bg_color)
                .corner_radius(3.0);

            egui::Area::new(ui.id().with("attribution"))
                .fixed_pos(rect.left_bottom())
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(5.0, -5.0))
                .show(ui.ctx(), |ui| {
                    frame.show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Small);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend); // Don't wrap attribution text.

                        if let Some(url) = self.config.attribution_url() {
                            ui.hyperlink_to(attribution, url);
                        } else {
                            ui.label(attribution);
                        }
                    });
                });
        }
    }
}

/// Converts longitude to the x-coordinate of a tile at a given zoom level.
fn lon_to_x(lon: f64, zoom: u8) -> f64 {
    (lon + 180.0) / 360.0 * (2.0_f64.powi(zoom as i32))
}

/// Converts latitude to the y-coordinate of a tile at a given zoom level.
fn lat_to_y(lat: f64, zoom: u8) -> f64 {
    (1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0
        * (2.0_f64.powi(zoom as i32))
}

/// Converts the x-coordinate of a tile to longitude at a given zoom level.
fn x_to_lon(x: f64, zoom: u8) -> f64 {
    x / (2.0_f64.powi(zoom as i32)) * 360.0 - 180.0
}

/// Converts the y-coordinate of a tile to latitude at a given zoom level.
fn y_to_lat(y: f64, zoom: u8) -> f64 {
    let n = std::f64::consts::PI - 2.0 * std::f64::consts::PI * y / (2.0_f64.powi(zoom as i32));
    n.sinh().atan().to_degrees()
}

impl Widget for &mut Map {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), Sense::drag().union(Sense::click()));
        let response_clone = response.clone();
        self.handle_input(ui, &rect, response_clone);
        self.draw_map_and_attribution(ui, &rect);

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OpenStreetMapConfig;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn test_coord_conversion_roundtrip() {
        let original_lon = 24.93545;
        let original_lat = 60.16952;
        let zoom: u8 = 10;

        let x = lon_to_x(original_lon, zoom);
        let y = lat_to_y(original_lat, zoom);

        let final_lon = x_to_lon(x, zoom);
        let final_lat = y_to_lat(y, zoom);

        assert!((original_lon - final_lon).abs() < EPSILON);
        assert!((original_lat - final_lat).abs() < EPSILON);

        let original_lon = -122.4194;
        let original_lat = 37.7749;

        let x = lon_to_x(original_lon, zoom);
        let y = lat_to_y(original_lat, zoom);

        let final_lon = x_to_lon(x, zoom);
        let final_lat = y_to_lat(y, zoom);

        assert!((original_lon - final_lon).abs() < EPSILON);
        assert!((original_lat - final_lat).abs() < EPSILON);
    }

    #[test]
    fn test_y_to_lat_conversion() {
        // y, zoom, expected_lat
        let test_cases = vec![
            // Equator
            (0.5, 0, 0.0),
            (128.0, 8, 0.0),
            // Near poles (Mercator projection limits)
            (0.0, 0, 85.0511287798),
            (1.0, 0, -85.0511287798),
            (0.0, 8, 85.0511287798),
            (256.0, 8, -85.0511287798),
            // Helsinki
            (9.262574089998255, 5, 60.16952),
            // London
            (85.12653378959828, 8, 51.5074),
        ];

        for (y, zoom, expected_lat) in test_cases {
            assert!((y_to_lat(y, zoom) - expected_lat).abs() < EPSILON);
        }
    }

    #[test]
    fn test_lat_to_y_conversion() {
        // lat, zoom, expected_y
        let test_cases = vec![
            // Equator
            (0.0, 0, 0.5),
            (0.0, 8, 128.0),
            // Near poles (Mercator projection limits)
            (85.0511287798, 0, 0.0),
            (-85.0511287798, 0, 1.0),
            (85.0511287798, 8, 0.0),
            (-85.0511287798, 8, 256.0),
            // Helsinki
            (60.16952, 5, 9.262574089998255),
            // London
            (51.5074, 8, 85.12653378959828),
        ];

        for (lat, zoom, expected_y) in test_cases {
            assert!((lat_to_y(lat, zoom) - expected_y).abs() < EPSILON);
        }
    }

    #[test]
    fn test_x_to_lon_conversion() {
        // x, zoom, expected_lon
        let test_cases = vec![
            // Center of the map
            (0.5, 0, 0.0),
            (128.0, 8, 0.0),
            // Edges of the map
            (0.0, 0, -180.0),
            (1.0, 0, 180.0),
            (0.0, 8, -180.0),
            (256.0, 8, 180.0),
            // Helsinki
            (18.216484444444444, 5, 24.93545),
        ];

        for (x, zoom, expected_lon) in test_cases {
            assert!((x_to_lon(x, zoom) - expected_lon).abs() < EPSILON);
        }
    }

    #[test]
    fn test_lon_to_x_conversion() {
        // lon, zoom, expected_x
        let test_cases = vec![
            // Center of the map
            (0.0, 0, 0.5),
            (0.0, 8, 128.0),
            // Edges of the map
            (-180.0, 0, 0.0),
            (180.0, 0, 1.0), // upper bound is exclusive for tiles, but not for coordinate space
            (-180.0, 8, 0.0),
            (180.0, 8, 256.0),
            // Helsinki
            (24.93545, 5, 18.216484444444444),
            // London
            (-0.1275, 8, 127.90933333333333),
        ];

        for (lon, zoom, expected_x) in test_cases {
            assert!((lon_to_x(lon, zoom) - expected_x).abs() < EPSILON);
        }
    }

    #[test]
    fn test_tile_id_to_url() {
        let config = OpenStreetMapConfig::default();
        let tile_id = TileId {
            z: 10,
            x: 559,
            y: 330,
        };
        let url = tile_id.to_url(&config);
        assert_eq!(url, "https://tile.openstreetmap.org/10/559/330.png");
    }

    #[test]
    fn test_map_new() {
        let config = OpenStreetMapConfig::default();
        let default_center = config.default_center();
        let default_zoom = config.default_zoom();

        let map = Map::new(config);

        assert_eq!(map.center, default_center);
        assert_eq!(map.zoom, default_zoom);
        assert!(map.mouse_pos.is_none());
        assert!(map.tiles.is_empty());
    }
}
