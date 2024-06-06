use std::collections::HashMap;

use egui::{Color32, emath::RectTransform, Pos2, Rect, Rounding, Shape};

use crate::CellKey;
use crate::dimensions::Dimensions;

pub fn get_conflict_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    cell_conflicts: &HashMap<CellKey, Vec<u64>>,
) -> Vec<Shape> {
    let grid_size = dimensions.cell_size();

    let shapes_len = cell_conflicts.keys().len() as u32;
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for (cx, cy) in cell_conflicts.keys() {
        let p00x = grid_size * dimensions.tranform_to_canvas_x(*cx);
        let p00y = grid_size * dimensions.tranform_to_canvas_y(*cy);
        let p00 = Pos2::new(p00x as f32, p00y as f32);

        let p11x = p00x + grid_size;
        let p11y = p00y + grid_size;
        let p11 = Pos2::new(p11x as f32, p11y as f32);

        let rect = Rect::from_two_pos(to_screen * p00, to_screen * p11);
        let shape = Shape::rect_filled(
            rect,
            Rounding::default(),
            Color32::from_rgba_unmultiplied(0, 255, 0, 10),
        );
        shapes.push(shape);
    }

    shapes
}
