#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::area::{Area, AreaLayer, AreaMode},
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My place polygons on a map test",
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
            points: vec![
                // Create GeoPos points relative to the maps default center
                (center_lon - 1.5, center_lat - 0.5).into(),
                (center_lon + 1.5, center_lat - 0.5).into(),
                (center_lon, center_lat + 1.0).into(),
            ],
        });
        area_layer.mode = AreaMode::Modify;

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

        //     egui::Window::new("Drawing")
        //         .resizable(false)
        //         .default_width(280.0)
        //         .show(ctx, |ui| {
        //             if let Some(drawing_layer) = self.map.layer_mut::<DrawingLayer>("drawing") {
        //                 ui.label("Mode");
        //                 ui.horizontal(|ui| {
        //                     ui.radio_value(
        //                         &mut drawing_layer.draw_mode,
        //                         DrawMode::Disabled,
        //                         "Disabled",
        //                     );
        //                     ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Draw, "Draw");
        //                     ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Erase, "Erase");
        //                 });

        //                 ui.add(
        //                     egui::Slider::new(&mut drawing_layer.stroke.width, 0.1..=10.0)
        //                         .text("Stroke width"),
        //                 );
        //                 ui.horizontal(|ui| {
        //                     ui.label("Stroke color:");
        //                     ui.color_edit_button_srgba(&mut drawing_layer.stroke.color);
        //                 });
        //             }
        //         });
    }
}
