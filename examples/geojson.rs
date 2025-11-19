#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::{area::AreaLayer, text::TextLayer},
    projection::GeoPos,
};

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
        // Read the GeoJSON data from the file.
        let geojson_str =
            std::fs::read_to_string("examples/data.geojson").expect("Failed to read data.geojson");

        let mut map = Map::new(OpenStreetMapConfig::default());
        map.center = GeoPos::from((10.0, 55.0));

        // Deserialize the GeoJSON into the AreaLayer.
        let mut area_layer = AreaLayer::default();
        if let Err(e) = area_layer.from_geojson_str(&geojson_str) {
            log::error!("Failed to deserialize shapes from GeoJSON: {}", e);
        }
        map.add_layer("areas", area_layer);

        // Deserialize the GeoJSON into the TextLayer.
        let mut text_layer = TextLayer::default();
        if let Err(e) = text_layer.from_geojson_str(&geojson_str) {
            log::error!("Failed to deserialize text from GeoJSON: {}", e);
        }
        map.add_layer("text", text_layer);

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
