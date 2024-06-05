use egui::{emath::RectTransform, Color32, Pos2, Rect, Rounding, Shape};

use crate::dimensions::Dimensions;

pub fn get_grid_shapes(to_screen: RectTransform, dimensions: &Dimensions) -> Vec<Shape> {
    let grid_color = Color32::BLACK;
    let grid_size = dimensions.cell_size();

    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            let p00x = grid_size * dimensions.tranform_to_canvas_x(cx);
            let p00y = grid_size * dimensions.tranform_to_canvas_y(cy);
            let p00 = Pos2::new(p00x as f32, p00y as f32);

            let p11x = p00x + grid_size;
            let p11y = p00y + grid_size;
            let p11 = Pos2::new(p11x as f32, p11y as f32);

            let rect = Rect::from_two_pos(to_screen * p00, to_screen * p11);
            let stroke = egui::Stroke::new(1.0, grid_color);
            let shape = Shape::rect_stroke(rect, Rounding::default(), stroke);
            shapes.push(shape);
        }
    }

    shapes
}
