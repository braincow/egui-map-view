#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::drawing::{DrawMode, DrawingLayer},
};
use std::collections::HashSet;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Draw on multiple map layers (GeoJSON)",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    map: Map,
    selected_layer_key: Option<String>,
    layer_counter: usize,
    error_message: Option<String>,
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
            error_message: None,
        }
    }
}

impl MyApp {
    fn save_geojson(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GeoJSON", &["geojson", "json"])
            .save_file()
        {
            let mut all_features = Vec::new();

            for key in self.map.layers().keys() {
                if let Some(layer) = self.map.layer::<DrawingLayer>(key) {
                    if let Ok(json_str) = layer.to_geojson_str(key) {
                        if let Ok(collection) =
                            serde_json::from_str::<geojson::FeatureCollection>(&json_str)
                        {
                            all_features.extend(collection.features);
                        }
                    }
                }
            }

            let feature_collection = geojson::FeatureCollection {
                bbox: None,
                features: all_features,
                foreign_members: None,
            };

            match serde_json::to_string_pretty(&feature_collection) {
                Ok(json_str) => {
                    if let Err(err) = std::fs::write(&path, json_str) {
                        self.error_message = Some(format!("Failed to write file: {}", err));
                    } else {
                        self.error_message = None;
                    }
                }
                Err(err) => {
                    self.error_message = Some(format!("Failed to serialize: {}", err));
                }
            }
        }
    }

    fn load_geojson(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("GeoJSON", &["geojson", "json"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(json_str) => {
                    // First, parse to find all layer IDs
                    match serde_json::from_str::<geojson::FeatureCollection>(&json_str) {
                        Ok(collection) => {
                            let mut layer_ids = HashSet::new();
                            for feature in &collection.features {
                                if let Some(properties) = &feature.properties {
                                    if let Some(id_val) = properties.get("layer_id") {
                                        if let Some(id) = id_val.as_str() {
                                            layer_ids.insert(id.to_string());
                                        }
                                    }
                                }
                            }

                            // Create layers if they don't exist
                            for id in &layer_ids {
                                if self.map.layer::<DrawingLayer>(id).is_none() {
                                    self.map.add_layer(id.clone(), DrawingLayer::default());
                                }
                            }

                            // Load data into layers
                            for id in layer_ids {
                                if let Some(layer) = self.map.layer_mut::<DrawingLayer>(&id) {
                                    if let Err(err) = layer.from_geojson_str(&json_str, Some(&id)) {
                                        self.error_message =
                                            Some(format!("Error loading layer {}: {}", id, err));
                                    }
                                }
                            }
                            self.error_message = None;
                        }
                        Err(err) => {
                            self.error_message = Some(format!("Failed to parse GeoJSON: {}", err));
                        }
                    }
                }
                Err(err) => {
                    self.error_message = Some(format!("Failed to read file: {}", err));
                }
            }
        }
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
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        self.save_geojson();
                    }
                    if ui.button("Load").clicked() {
                        self.load_geojson();
                    }
                });
                if let Some(msg) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, msg);
                }

                ui.separator();

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
