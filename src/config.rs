//! Configuration for different map providers.

use crate::TileId;

/// Configuration for a map provider.
pub trait MapConfig {
    /// Returns the URL for a given tile.
    fn tile_url(&self, tile: &TileId) -> String;

    /// Returns the attribution text to be displayed on the map. If returns `None`, no attribution is shown.
    fn attribution(&self) -> Option<&String>;

    /// Returns the attribution URL to be linked from the attribution text.
    fn attribution_url(&self) -> Option<&String>;

    /// The default geographical center of the map. (longitude, latitude)
    fn default_center(&self) -> (f64, f64);

    /// The default zoom level of the map.
    fn default_zoom(&self) -> u8;

    /// Returns the minimum zoom level allowed by this provider.
    fn min_zoom(&self) -> u8 {
        0
    }

    /// Returns the maximum zoom level allowed by this provider.
    fn max_zoom(&self) -> u8 {
        19
    }
}

/// Configuration for the OpenStreetMap tile server.
///
/// # Example
///
/// ```
/// use egui_map_view::config::OpenStreetMapConfig;
/// let config = OpenStreetMapConfig::default();
/// ```
#[cfg(feature = "openstreetmap")]
pub struct OpenStreetMapConfig {
    base_url: String,
    attribution: String,
    attribution_url: String,
    default_center: (f64, f64),
    default_zoom: u8,
    min_zoom: u8,
    max_zoom: u8,
}

#[cfg(feature = "openstreetmap")]
impl Default for OpenStreetMapConfig {
    fn default() -> Self {
        Self {
            base_url: "https://tile.openstreetmap.org".to_string(),
            attribution: "© OpenStreetMap contributors".to_string(),
            attribution_url: "https://www.openstreetmap.org".to_string(),
            default_center: (24.93545, 60.16952), // Helsinki, Finland
            default_zoom: 5,
            min_zoom: 0,
            max_zoom: 19,
        }
    }
}

#[cfg(feature = "openstreetmap")]
impl MapConfig for OpenStreetMapConfig {
    fn tile_url(&self, tile: &TileId) -> String {
        format!("{}/{}/{}/{}.png", self.base_url, tile.z, tile.x, tile.y)
    }

    fn attribution(&self) -> Option<&String> {
        Some(&self.attribution)
    }

    fn attribution_url(&self) -> Option<&String> {
        Some(&self.attribution_url)
    }

    fn default_center(&self) -> (f64, f64) {
        self.default_center
    }

    fn default_zoom(&self) -> u8 {
        self.default_zoom
    }

    fn min_zoom(&self) -> u8 {
        self.min_zoom
    }

    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }
}

#[cfg(feature = "openstreetmap")]
impl OpenStreetMapConfig {
    /// Sets the minimum zoom level.
    pub fn min_zoom(mut self, min_zoom: u8) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    /// Sets the maximum zoom level.
    pub fn max_zoom(mut self, max_zoom: u8) -> Self {
        self.max_zoom = max_zoom;
        self
    }
}

/// Configuration for the Karttapaikka tile server.
///
/// # Example
///
/// ```
/// use egui_map_view::config::KarttapaikkaMapConfig;
/// let config = KarttapaikkaMapConfig::new("my-api-key".to_string());
/// ```
#[cfg(feature = "karttapaikka")]
pub struct KarttapaikkaMapConfig {
    base_url: String,
    attribution: String,
    attribution_url: String,
    default_center: (f64, f64),
    default_zoom: u8,
    api_key: String,
    min_zoom: u8,
    max_zoom: u8,
}

#[cfg(feature = "karttapaikka")]
impl Default for KarttapaikkaMapConfig {
    fn default() -> Self {
        Self {
            base_url: "https://avoin-karttakuva.maanmittauslaitos.fi/avoin/wmts/1.0.0/maastokartta/default/WGS84_Pseudo-Mercator".to_string(),
            attribution: "© Maanmittauslaitos".to_string(),
            attribution_url: "https://www.maanmittauslaitos.fi/asioi-verkossa/karttapaikka".to_string(),
            default_center: (24.93545, 60.16952), // Helsinki, Finland
            default_zoom: 15,
            api_key: "your-key-here".to_string(),
            min_zoom: 0,
            max_zoom: 15,
        }
    }
}

#[cfg(feature = "karttapaikka")]
impl MapConfig for KarttapaikkaMapConfig {
    fn tile_url(&self, tile: &TileId) -> String {
        format!(
            "{}/{}/{}/{}.png?api-key={}",
            self.base_url, tile.z, tile.y, tile.x, self.api_key
        )
    }

    fn attribution(&self) -> Option<&String> {
        Some(&self.attribution)
    }

    fn attribution_url(&self) -> Option<&String> {
        Some(&self.attribution_url)
    }

    fn default_center(&self) -> (f64, f64) {
        self.default_center
    }

    fn default_zoom(&self) -> u8 {
        self.default_zoom
    }

    fn min_zoom(&self) -> u8 {
        self.min_zoom
    }

    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }
}

#[cfg(feature = "karttapaikka")]
impl KarttapaikkaMapConfig {
    /// Creates a new `KarttapaikkaMapConfig` with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            ..Self::default()
        }
    }

    /// Sets the minimum zoom level.
    pub fn min_zoom(mut self, min: u8) -> Self {
        self.min_zoom = min;
        self
    }

    /// Sets the maximum zoom level.
    pub fn max_zoom(mut self, max: u8) -> Self {
        self.max_zoom = max;
        self
    }
}

/// A dynamic map configuration that allows defining a custom tile URL function at runtime.
///
/// # Example
///
/// ```
/// use egui_map_view::config::DynMapConfig;
/// let config = DynMapConfig::new(|tile| format!("https://my-tile-server/{}/{}/{}.png", tile.z, tile.x, tile.y));
/// ```
pub struct DynMapConfig {
    tile_url: Box<dyn Fn(&TileId) -> String>,
    min_zoom: u8,
    max_zoom: u8,
}

impl DynMapConfig {
    /// Creates a new `DynMapConfig` with a custom tile URL function.
    pub fn new(tile_url: impl Fn(&TileId) -> String + 'static) -> Self {
        Self {
            tile_url: Box::new(tile_url),
            min_zoom: 0,
            max_zoom: 19,
        }
    }

    /// Sets the minimum zoom level.
    pub fn min_zoom(mut self, min_zoom: u8) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    /// Sets the maximum zoom level.
    pub fn max_zoom(mut self, max_zoom: u8) -> Self {
        self.max_zoom = max_zoom;
        self
    }
}

impl MapConfig for DynMapConfig {
    fn tile_url(&self, tile: &TileId) -> String {
        (self.tile_url)(tile)
    }

    fn attribution(&self) -> Option<&String> {
        None
    }

    fn attribution_url(&self) -> Option<&String> {
        None
    }

    fn default_center(&self) -> (f64, f64) {
        (24.93545, 60.16952)
    }

    fn default_zoom(&self) -> u8 {
        2
    }

    fn min_zoom(&self) -> u8 {
        self.min_zoom
    }

    fn max_zoom(&self) -> u8 {
        self.max_zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TileId;

    #[test]
    #[cfg(feature = "openstreetmap")]
    fn openstreetmap_config_default() {
        let config = OpenStreetMapConfig::default();
        assert_eq!(config.base_url, "https://tile.openstreetmap.org");
        assert_eq!(config.attribution, "© OpenStreetMap contributors");
        assert_eq!(config.default_center, (24.93545, 60.16952));
        assert_eq!(config.default_zoom, 5);
    }

    #[test]
    #[cfg(feature = "openstreetmap")]
    fn openstreetmap_config_tile_url() {
        let config = OpenStreetMapConfig::default();
        let tile_id = TileId { z: 10, x: 1, y: 2 };
        let url = config.tile_url(&tile_id);
        assert_eq!(url, "https://tile.openstreetmap.org/10/1/2.png");
    }

    #[test]
    #[cfg(feature = "karttapaikka")]
    fn karttapaikka_config_new() {
        let api_key = "test-api-key".to_string();
        let config = KarttapaikkaMapConfig::new(api_key.clone());
        assert_eq!(config.api_key, api_key);
        assert_eq!(
            config.base_url,
            "https://avoin-karttakuva.maanmittauslaitos.fi/avoin/wmts/1.0.0/maastokartta/default/WGS84_Pseudo-Mercator"
        );
        assert_eq!(config.attribution, "© Maanmittauslaitos");
        assert_eq!(config.default_center, (24.93545, 60.16952));
        assert_eq!(config.default_zoom, 15);
    }

    #[test]
    #[cfg(feature = "karttapaikka")]
    fn karttapaikka_config_tile_url() {
        let api_key = "test-api-key".to_string();
        let config = KarttapaikkaMapConfig::new(api_key.clone());
        let tile_id = TileId { z: 10, x: 1, y: 2 };
        let url = config.tile_url(&tile_id);
        assert_eq!(
            url,
            "https://avoin-karttakuva.maanmittauslaitos.fi/avoin/wmts/1.0.0/maastokartta/default/WGS84_Pseudo-Mercator/10/2/1.png?api-key=test-api-key"
        );
    }

    #[test]
    #[cfg(feature = "openstreetmap")]
    fn test_openstreetmap_zoom_limits() {
        let config = OpenStreetMapConfig::default();
        assert_eq!(MapConfig::min_zoom(&config), 0);
        assert_eq!(MapConfig::max_zoom(&config), 19);

        let customized = config.min_zoom(2).max_zoom(18);
        assert_eq!(MapConfig::min_zoom(&customized), 2);
        assert_eq!(MapConfig::max_zoom(&customized), 18);
    }

    #[test]
    #[cfg(feature = "karttapaikka")]
    fn test_karttapaikka_zoom_limits() {
        let config = KarttapaikkaMapConfig::default();
        assert_eq!(MapConfig::min_zoom(&config), 0);
        assert_eq!(MapConfig::max_zoom(&config), 15);

        let customized = config.min_zoom(5).max_zoom(12);
        assert_eq!(MapConfig::min_zoom(&customized), 5);
        assert_eq!(MapConfig::max_zoom(&customized), 12);
    }

    #[test]
    fn test_dyn_zoom_limits() {
        let config = DynMapConfig::new(|tile| format!("url/{}", tile.z));
        assert_eq!(MapConfig::min_zoom(&config), 0);
        assert_eq!(MapConfig::max_zoom(&config), 19);

        let customized = config.min_zoom(3).max_zoom(17);
        assert_eq!(MapConfig::min_zoom(&customized), 3);
        assert_eq!(MapConfig::max_zoom(&customized), 17);
    }
}
