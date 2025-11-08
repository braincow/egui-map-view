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
}

#[cfg(feature = "karttapaikka")]
impl KarttapaikkaMapConfig {
    /// Creates a new `KarttapaikkaMapConfig` with the given API key.
    pub fn new(api_key: String) -> Self {
        let mut config = Self::default();
        config.api_key = api_key;
        config
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
}

impl DynMapConfig {
    /// Creates a new `DynMapConfig` with a custom tile URL function.
    pub fn new(tile_url: impl Fn(&TileId) -> String + 'static) -> Self {
        Self {
            tile_url: Box::new(tile_url),
        }
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
}