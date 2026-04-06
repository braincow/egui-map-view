//! An example of using the SVG layer to display SVG images on the map.

use eframe::egui;
use egui_map_view::{
    Map,
    config::OpenStreetMapConfig,
    layers::svg::{SvgElement, SvgLayer},
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SVG Layer Example",
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

        // Create a new SVG layer
        let mut svg_layer = SvgLayer::default();

        // A simple red circle SVG
        let red_circle = r#"<svg height="100" width="100" xmlns="http://www.w3.org/2000/svg">
            <circle r="40" cx="50" cy="50" fill="red" />
        </svg>"#;

        // A simple blue square SVG
        let blue_square = r#"<svg height="100" width="100" xmlns="http://www.w3.org/2000/svg">
            <rect width="80" height="80" x="10" y="10" fill="blue" />
        </svg>"#;

        // A simple green triangle SVG
        let green_triangle = r#"<svg height="100" width="100" xmlns="http://www.w3.org/2000/svg">
            <polygon points="50,15 90,85 10,85" fill="green" />
        </svg>"#;

        // Add some SVG elements to the layer
        // Helsinki (approx 24.94, 60.17)
        svg_layer.add_element(
            SvgElement::from_xy(24.94, 60.17, red_circle, "Helsinki Red Circle (Clickable)")
                .with_scalable(true),
        );

        // London (approx -0.12, 51.50)
        svg_layer.add_element(
            SvgElement::from_xy(
                -0.12,
                51.50,
                blue_square,
                "London Blue Square (Non-clickable)",
            )
            .with_clickable(false),
        );

        // Stockholm (approx 18.06, 59.32)
        svg_layer.add_element(
            SvgElement::from_xy(
                18.06,
                59.32,
                green_triangle,
                "Stockholm Green Triangle (Draggable)",
            )
            .with_draggable(true),
        );

        // Add the layer to the map
        map.add_layer("svg_icons", svg_layer);

        Self { map }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.add_sized(ui.available_size_before_wrap(), &mut self.map);
            });

        // Check for click events on the SVG layer
        if let Some(svg_layer) = self.map.layer_mut::<SvgLayer>("svg_icons") {
            for event in svg_layer.take_events() {
                println!(
                    "SVG clicked: {} with {:?} button at geo: {:?}, screen: {:?}",
                    event.metadata, event.button, event.world_pos, event.screen_pos
                );
            }
        }
    }
}
