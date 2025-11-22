# Changelog

All notable changes to this project will be documented in this file.

## [0.3.1] - 2025-11-22

### Fixed

- Fixed regression where double-clicking a polygon line segment failed to insert a new vertex. (#32)

## [0.3.0] - 2025-11-19

### Added

- Added GeoJSON serialization and deserialization support for `AreaLayer`, `DrawingLayer`, and `TextLayer` (#28).
- Added `to_geojson_str` and `from_geojson_str` methods to supported layers.
- Added `draw_many_layers_geojson` example demonstrating loading and saving layers.
- Added `geojson` feature flag (enabled by default).
- Added polygon draw layer support with `AreaLayer` (#29).

## [0.2.3] - 2024-11-15

### Added

- Add contributors to `README.md` (#26, #18)
- Added contribution from @Niki123456123456 that adds overlayd tile layers (#17)

### Changed

- Bump `thiserror` crate version (#11)
- Bump `serde_json` crate version (#13)
- Bump `serde` crate version (#14)
- Bump `egui` and `eframe` crate versions; fix changed API calls (#21)
- Bump `reqwest` crate version (#23)
