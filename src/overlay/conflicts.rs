use std::collections::HashMap;

use egui::{emath::RectTransform, Color32, Rounding, Shape};

use crate::dimensions::Dimensions;
use crate::{get_rect_at_cell, CellKey};

pub fn get_conflict_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    cell_conflicts: &HashMap<CellKey, Vec<u64>>,
) -> Vec<Shape> {
    let shapes_len = cell_conflicts.keys().len() as u32;
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for key in cell_conflicts.keys() {
        let rect = get_rect_at_cell(dimensions, to_screen, *key);
        let shape = Shape::rect_filled(
            rect,
            Rounding::default(),
            Color32::from_rgba_unmultiplied(0, 255, 0, 10),
        );
        shapes.push(shape);
    }

    shapes
}
