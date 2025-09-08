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
                // By using a right-to-left layout, we can have a widget on the right
                // that takes its preferred size, and then have the map fill the rest.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("right to map");
                    // The map widget will take up all the remaining space.
                    if ui.add_sized(ui.available_size(), &mut self.map).clicked() {
                        if let Some(pos) = self.map.mouse_pos {
                            println!("{},{}", pos.lon, pos.lat);
                        }
                    }
                });
            });

            ui.label("below map will eat whats left from the horizontal layout");
        });
    }
}
