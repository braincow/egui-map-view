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
        "Draw on multiple map layers",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    map: Map,
    selected_layer_key: Option<String>,
    layer_counter: usize,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut map = Map::new(OpenStreetMapConfig::default());
        let initial_layer_key = "Layer 1".to_string();
        map.add_layer(initial_layer_key.clone(), DrawingLayer::default());

        Self {
            map,
            selected_layer_key: Some(initial_layer_key),
            layer_counter: 1,
        }
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
                let layer_management_enabled = if let Some(selected_key) = &self.selected_layer_key
                {
                    self.map
                        .layer::<DrawingLayer>(selected_key)
                        .map_or(true, |l| l.draw_mode == DrawMode::Disabled)
                } else {
                    true
                };

                ui.add_enabled_ui(layer_management_enabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Layers");
                        ui.separator();

                        if ui.button("+").clicked() {
                            self.layer_counter += 1;
                            let new_layer_key = format!("Layer {}", self.layer_counter);
                            self.map
                                .add_layer(new_layer_key.clone(), DrawingLayer::default());
                            self.selected_layer_key = Some(new_layer_key);
                        }

                        let can_remove = self.map.layers().len() > 1;
                        if ui.add_enabled(can_remove, egui::Button::new("-")).clicked() {
                            if let Some(selected_key) = self.selected_layer_key.take() {
                                self.map.remove_layer(&selected_key);
                            }
                            self.selected_layer_key = self.map.layers().keys().next().cloned();
                        }
                    });

                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let layer_keys: Vec<String> = self.map.layers().keys().cloned().collect();
                        for key in layer_keys {
                            let is_selected = self.selected_layer_key.as_ref() == Some(&key);
                            if ui.selectable_label(is_selected, &key).clicked() {
                                self.selected_layer_key = Some(key);
                            }
                        }
                    });
                });

                ui.separator();

                if let Some(selected_key) = self.selected_layer_key.clone() {
                    if let Some(drawing_layer) = self.map.layer_mut::<DrawingLayer>(&selected_key) {
                        ui.label(format!("Controls for '{}'", selected_key));
                        ui.label("Mode");
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut drawing_layer.draw_mode,
                                DrawMode::Disabled,
                                "Disabled",
                            );
                            ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Draw, "Draw");
                            ui.radio_value(&mut drawing_layer.draw_mode, DrawMode::Erase, "Erase");
                        });

                        ui.add(
                            egui::Slider::new(&mut drawing_layer.stroke.width, 0.1..=10.0)
                                .text("Stroke width"),
                        );
                        ui.horizontal(|ui| {
                            ui.label("Stroke color:");
                            ui.color_edit_button_srgba(&mut drawing_layer.stroke.color);
                        });
                    }
                }
            });
    }
}
