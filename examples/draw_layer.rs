#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::drawing::{DrawMode, DrawingLayer},
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My draw on a map test",
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
        map.add_layer("drawing", DrawingLayer::new());
        Self { map }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.add(&mut self.map).clicked();
            });

        egui::Window::new("Drawing")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                if let Some(layer) = self.map.layers_mut().get_mut("drawing") {
                    if let Some(drawing_layer) = layer.as_any_mut().downcast_mut::<DrawingLayer>() {
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut drawing_layer.draw_mode,
                                DrawMode::Disabled,
                                "Disabled",
                            );
                            ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Draw, "Draw");
                            ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Erase, "Erase");
                        });
                    }
                }
            });
    }
}
