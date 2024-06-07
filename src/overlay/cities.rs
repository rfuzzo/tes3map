use std::collections::HashMap;

use eframe::epaint::Stroke;
use egui::{emath::RectTransform, Color32, Pos2, Rect, Rounding, Shape};
use tes3::esp::Cell;

use crate::{dimensions::Dimensions, CellKey};

pub fn get_cities_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    cell_size: usize,
    cell_records: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let color = Color32::from_rgb(202, 165, 96);
    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            // get region
            let key = (cx, cy);
            if let Some(cell) = cell_records.get(&key) {
                if cell.name.is_empty() || cell.is_interior() {
                    continue;
                }

                let p00x = cell_size * dimensions.tranform_to_canvas_x(cx);
                let p00y = cell_size * dimensions.tranform_to_canvas_y(cy);
                let p00 = Pos2::new(p00x as f32, p00y as f32);

                let p11x = p00x + cell_size;
                let p11y = p00y + cell_size;
                let p11 = Pos2::new(p11x as f32, p11y as f32);

                let rect = Rect::from_two_pos(to_screen * p00, to_screen * p11);
                let shape = Shape::rect_stroke(rect, Rounding::default(), Stroke::new(4.0, color));
                shapes.push(shape);
            }
        }
    }
    shapes
}
