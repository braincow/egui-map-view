#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(rustdoc::missing_crate_level_docs)]

use eframe::egui;
use egui_map_view::{Map, config::OpenStreetMapConfig};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Map Window Repro",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    map: Map,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            map: Map::new(OpenStreetMapConfig::default()),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Map inside a Window Test");
            ui.label("Drag the map or zoom.");

            egui::Window::new("Map Window")
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    ui.add_sized(ui.available_size(), &mut self.map);
                });
        });
    }
}
