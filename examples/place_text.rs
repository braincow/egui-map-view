#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::text::{TextLayer, TextLayerMode, TextSize},
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Place text on map",
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
        map.add_layer("text_layer", TextLayer::default());

        Self { map }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let map_response = egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.add(&mut self.map);
            })
            .response;

        // Handle context menu from right-clicking on a text element.
        map_response.context_menu(|ui| {
            if let Some(layer) = self.map.layer_mut::<TextLayer>("text_layer") {
                if let Some(index) = layer.last_right_clicked_index.take() {
                    ui.label("Text Options");
                    if ui.button("Edit").clicked() {
                        layer.start_editing(index);
                        ui.close();
                    }
                    if ui.button("Delete").clicked() {
                        layer.delete(index);
                        ui.close();
                    }
                } else {
                    // If no text was clicked, you could add other options here.
                    ui.label("Map");
                }
            }
        });

        // Show the main controls window.
        egui::Window::new("Controls")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                if let Some(layer) = self.map.layer_mut::<TextLayer>("text_layer") {
                    ui.heading("Mode");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut layer.mode, TextLayerMode::Disabled, "Disabled");
                        ui.radio_value(&mut layer.mode, TextLayerMode::Modify, "Modify");
                    });
                }
            });

        // Show the edit window if we are adding or modifying a text element.
        if let Some(layer) = self.map.layer_mut::<TextLayer>("text_layer") {
            // Take the editing state out of the layer to modify it.
            if let Some(mut editing) = layer.editing.take() {
                let mut open = true;
                let mut should_commit = false;
                let title = if editing.index.is_some() {
                    "Edit Text"
                } else {
                    "Add Text"
                };
                egui::Window::new(title)
                    .open(&mut open)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.text_edit_singleline(&mut editing.properties.text);

                        ui.horizontal(|ui| {
                            ui.label("Text Color:");
                            ui.color_edit_button_srgba(&mut editing.properties.color);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Background Color:");
                            ui.color_edit_button_srgba(&mut editing.properties.background);
                        });

                        ui.label("Text Size");
                        let is_static = matches!(editing.properties.size, TextSize::Static(_));
                        if ui.radio(is_static, "Static").clicked() {
                            editing.properties.size = TextSize::Static(12.0);
                        }
                        if let TextSize::Static(size) = &mut editing.properties.size {
                            ui.add_enabled(is_static, egui::Slider::new(size, 4.0..=50.0));
                        }

                        let is_relative = matches!(editing.properties.size, TextSize::Relative(_));
                        if ui.radio(is_relative, "Relative").clicked() {
                            editing.properties.size = TextSize::Relative(5000.0);
                        }
                        if let TextSize::Relative(size) = &mut editing.properties.size {
                            ui.add_enabled(
                                is_relative,
                                egui::Slider::new(size, 100.0..=10000.0)
                                    .logarithmic(true)
                                    .text("meters"),
                            );
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Ok").clicked() {
                                should_commit = true;
                                open = false; // This will close the window.
                            }
                            if ui.button("Cancel").clicked() {
                                open = false; // This will close the window.
                            }
                        });
                    });

                if should_commit {
                    layer.editing = Some(editing);
                    layer.commit_edit();
                } else if open {
                    // If the window is still open, put the editing state back for the next frame.
                    layer.editing = Some(editing);
                }
            }
        }
    }
}
