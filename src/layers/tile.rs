//! A layer for tile maps on the map.

use egui::{Color32, Painter, Response};
use std::{any::Any, collections::HashMap};

use crate::{
    Tile, TileId, config::MapConfig, draw_tile, layers::Layer, load_tile,
    projection::MapProjection, visible_tiles,
};

/// A layer that manages and renders map tiles on the map view.
pub struct TileLayer {
    tiles: HashMap<TileId, Tile>,
    visible_tiles: Vec<(TileId, egui::Pos2)>,
    /// Color tint applied to the tile images when rendering
    pub tint: Color32,
    config: Box<dyn MapConfig>,
}

impl TileLayer {
    /// Creates a new tile layer with the given map configuration.
    pub fn new(config: impl MapConfig + 'static) -> Self {
        Self {
            tiles: Default::default(),
            visible_tiles: Default::default(),
            tint: Color32::WHITE,
            config: Box::new(config),
        }
    }
}

impl Layer for TileLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn handle_input(&mut self, response: &Response, projection: &MapProjection) -> bool {
        self.visible_tiles = visible_tiles(projection).collect();
        for (tile_id, _) in &self.visible_tiles {
            load_tile(
                &mut self.tiles,
                self.config.as_ref(),
                &response.ctx,
                *tile_id,
            );
        }
        return false;
    }

    fn draw(&self, painter: &Painter, _: &MapProjection) {
        for (tile_id, tile_pos) in &self.visible_tiles {
            draw_tile(&self.tiles, painter, tile_id, *tile_pos, self.tint);
        }
    }
}
