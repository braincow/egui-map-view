#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{Map, config::OpenStreetMapConfig};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My map test",
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
            ui.heading("EXAMPLE");
            ui.label(self.map.center.lon.to_string());
            ui.label(self.map.center.lat.to_string());
            ui.label(self.map.zoom.to_string());

            ui.horizontal(|ui| {
                if ui.add(&mut self.map).clicked() {
                    if let Some(pos) = self.map.mouse_pos {
                        println!("{},{}", pos.lon, pos.lat);
                    }
                }
                ui.label("right to map")
            });

            ui.label("below map");
        });
    }
}
