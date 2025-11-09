#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui::{Color32, Stroke};
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::area::{Area, AreaLayer, AreaMode, AreaShape::*},
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Place polygons on a map test",
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
        let mut area_layer = AreaLayer::default();
        let (center_lon, center_lat) = map.center.into();

        area_layer.add_area(Area {
            shape: Polygon(vec![
                // Create GeoPos points relative to the maps default center
                (center_lon - 1.5, center_lat - 0.5).into(),
                (center_lon + 1.5, center_lat - 0.5).into(),
                (center_lon, center_lat + 1.0).into(),
            ]),
            stroke: Stroke::new(2.0, Color32::from_rgb(255, 0, 0)),
            fill: Color32::from_rgba_unmultiplied(255, 0, 0, 50),
        });

        // Add a circle
        let circle_center_lon = center_lon - 3.5;
        let circle_center_lat = center_lat;
        let radius = 150000.0; // In meters

        area_layer.add_area(Area {
            shape: Circle {
                center: (circle_center_lon, circle_center_lat).into(),
                radius,
                points: None, // calculate reasonable amount of points on the circle polygon based on its radius
            },
            stroke: Stroke::new(2.0, Color32::from_rgb(0, 102, 255)),
            fill: Color32::from_rgba_unmultiplied(0, 102, 255, 50),
        });

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

        egui::Window::new("Areas")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                if let Some(area_layer) = self.map.layer_mut::<AreaLayer>("areas") {
                    ui.label("Mode");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut area_layer.mode, AreaMode::Disabled, "Disabled");
                        ui.radio_value(&mut area_layer.mode, AreaMode::Modify, "Modify");
                    });
                }
            });
    }
}
