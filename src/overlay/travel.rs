use std::collections::HashMap;

use eframe::emath::RectTransform;
use eframe::epaint::{Color32, Shape};
use egui::epaint::PathStroke;
use egui::Vec2;

use crate::dimensions::Dimensions;
use crate::CellKey;

fn get_color_for_class(class: &str) -> Color32 {
    match class {
        "Shipmaster" => Color32::BLUE,
        "Caravaner" => Color32::GOLD,
        "Gondolier" => Color32::GRAY,
        "T_Mw_RiverstriderService" => Color32::LIGHT_BLUE,
        _ => Color32::RED,
    }
}

pub fn get_travel_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    edges: &HashMap<String, Vec<(CellKey, CellKey)>>,
) -> Vec<Shape> {
    let shapes_len = edges
        .iter()
        .fold(0, |acc, (_, destinations)| acc + destinations.len());
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len);

    for (class, destinations) in edges.iter() {
        // get class color
        let color = get_color_for_class(class);

        for (key, value) in destinations {
            let p00 = dimensions.tranform_to_canvas(*key) + Vec2::new(0.5, 0.5);
            let p11 = dimensions.tranform_to_canvas(*value) + Vec2::new(0.5, 0.5);

            let line = Shape::LineSegment {
                points: [to_screen * p00, to_screen * p11],
                stroke: PathStroke::new(2.0, color),
            };
            shapes.push(line);
        }
    }

    shapes
}
