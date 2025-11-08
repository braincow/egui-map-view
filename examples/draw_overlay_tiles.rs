#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui::{Color32, Slider};
use egui_map_view::{
    Map,
    config::{DynMapConfig, OpenStreetMapConfig},
    layers::tile::TileLayer,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "My draw on a overlay tiles test",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    map: Map,
    overlay_names: Vec<&'static str>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut map = Map::new(OpenStreetMapConfig::default());

        let overlays: Vec<(&'static str, DynMapConfig)> = vec![
            (
                "railway",
                DynMapConfig::new(|tile| {
                    format!(
                        "https://tiles.openrailwaymap.org/standard/{}/{}/{}.png",
                        tile.z, tile.x, tile.y
                    )
                }),
            ),
            (
                "hiking",
                DynMapConfig::new(|tile| {
                    format!(
                        "https://tile.waymarkedtrails.org/hiking/{}/{}/{}.png",
                        tile.z, tile.x, tile.y
                    )
                }),
            ),
        ];

        let overlay_names: Vec<_> = overlays.iter().map(|o| o.0).collect();

        for overlay in overlays.into_iter() {
            map.add_layer(overlay.0, TileLayer::new(overlay.1));
        }

        Self { map, overlay_names }
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

        egui::Window::new("Drawing")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                for overlay in self.overlay_names.iter() {
                    if let Some(drawing_layer) = self.map.layer_mut::<TileLayer>(overlay) {
                        let mut a = drawing_layer.tint.a();
                        ui.horizontal(|ui| {
                            ui.label(*overlay);
                            ui.add(Slider::new(&mut a, 0u8..=255u8));
                        });

                        drawing_layer.tint = Color32::from_rgba_unmultiplied(255, 255, 255, a);
                    }
                }
            });
    }
}
