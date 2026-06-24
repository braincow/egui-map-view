use super::layer::AreaLayer;
use super::types::Area;

#[cfg(feature = "geojson")]
use ::geojson::{Feature, FeatureCollection};

impl AreaLayer {
    /// Serializes the layer to a `GeoJSON` `FeatureCollection`.
    #[cfg(feature = "geojson")]
    pub fn to_geojson_str(&self) -> Result<String, serde_json::Error> {
        let features: Vec<Feature> = self
            .areas
            .clone()
            .into_iter()
            .map(Feature::from)
            .collect();
        let mut foreign_members = serde_json::Map::new();
        foreign_members.insert(
            "opacity".to_string(),
            serde_json::Value::from(f64::from(self.opacity)),
        );

        let feature_collection = FeatureCollection {
            bbox: None,
            features,
            foreign_members: Some(foreign_members),
        };
        serde_json::to_string(&feature_collection)
    }

    /// Deserializes a `GeoJSON` `FeatureCollection` and adds the features to the layer.
    #[cfg(feature = "geojson")]
    pub fn from_geojson_str(&mut self, s: &str) -> Result<(), serde_json::Error> {
        let feature_collection: FeatureCollection = serde_json::from_str(s)?;
        let new_areas: Vec<Area> = feature_collection
            .features
            .into_iter()
            .filter_map(|f| Area::try_from(f).ok())
            .collect();
        self.areas.extend(new_areas);

        if let Some(foreign_members) = feature_collection.foreign_members
            && let Some(value) = foreign_members.get("opacity")
            && let Some(opacity) = value.as_f64()
        {
            self.opacity = opacity as f32;
        }
        Ok(())
    }
}
