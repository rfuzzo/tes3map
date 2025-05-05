use egui::{emath::RectTransform, Color32, CornerRadius, Shape, StrokeKind};

use crate::{dimensions::Dimensions, get_rect_at_cell};

pub fn get_grid_shapes(to_screen: RectTransform, dimensions: &Dimensions) -> Vec<Shape> {
    let grid_color = Color32::BLACK;

    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            let key = (cx, cy);
            let rect = get_rect_at_cell(dimensions, to_screen, key);
            let stroke = egui::Stroke::new(1.0, grid_color);
            let shape =
                Shape::rect_stroke(rect, CornerRadius::default(), stroke, StrokeKind::Middle);
            shapes.push(shape);
        }
    }

    shapes
}
