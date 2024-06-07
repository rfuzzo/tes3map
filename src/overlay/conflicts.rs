use std::collections::HashMap;

use egui::{emath::RectTransform, Color32, Pos2, Rect, Rounding, Shape};

use crate::dimensions::Dimensions;
use crate::CellKey;

pub fn get_conflict_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    cell_conflicts: &HashMap<CellKey, Vec<u64>>,
) -> Vec<Shape> {
    let shapes_len = cell_conflicts.keys().len() as u32;
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for (cx, cy) in cell_conflicts.keys() {
        let p00x = dimensions.tranform_to_canvas_x(*cx);
        let p00y = dimensions.tranform_to_canvas_y(*cy);
        let p00 = Pos2::new(p00x as f32, p00y as f32);

        let p11x = p00x + 1;
        let p11y = p00y + 1;
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
