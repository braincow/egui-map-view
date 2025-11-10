#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{Map, config::OpenStreetMapConfig, layers::area::AreaLayer};

fn main() -> eframe::Result {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Load GeoJSON shapes on a map",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    map: Map,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut map = Map::new(OpenStreetMapConfig::default());
        let (center_lon, center_lat) = map.center.into();

        let mut area_layer = AreaLayer::default();
        // Define a GeoJSON string with a FeatureCollection.
        // This one contains a polygon and a circle.
        let geojson_str = format!(
            r##"
        {{
            "type": "FeatureCollection",
            "features": [
                {{
                    "type": "Feature",
                    "geometry": {{
                        "type": "Polygon",
                        "coordinates": [[ [{}, {}], [{}, {}], [{}, {}], [{}, {}] ]]
                    }},
                    "properties": {{
                        "stroke_color": "#ff0000ff",
                        "stroke_width": 2.0,
                        "fill_color": "#ff000080"
                    }}
                }},
                {{
                    "type": "Feature",
                    "geometry": {{
                        "type": "Point",
                        "coordinates": [{}, {}]
                    }},
                    "properties": {{
                        "radius": 150000.0,
                        "stroke_color": "#0066ffff",
                        "stroke_width": 2.0,
                        "fill_color": "#0066ff80"
                    }}
                }}
            ]
        }}"##,
            // Polygon coordinates
            center_lon - 1.5,
            center_lat - 0.5,
            center_lon + 1.5,
            center_lat - 0.5,
            center_lon,
            center_lat + 1.0,
            center_lon - 1.5,
            center_lat - 0.5,
            // Circle coordinates
            center_lon - 3.5,
            center_lat
        );

        // Deserialize the GeoJSON into the AreaLayer.
        if let Err(e) = area_layer.from_geojson_str(&geojson_str) {
            log::error!("Failed to deserialize GeoJSON: {}", e);
        }

        map.add_layer("areas", area_layer);
        Self { map }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.add_sized(ui.available_size_before_wrap(), &mut self.map)
                    .clicked();
            });
    }
}
