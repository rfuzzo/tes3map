use std::collections::HashMap;

use egui::{emath::RectTransform, Color32, CornerRadius, Shape};
use tes3::esp::{Cell, Region};

use crate::{dimensions::Dimensions, get_rect_at_cell, CellKey};

pub fn get_region_shapes(
    to_screen: RectTransform,
    dimensions: &Dimensions,
    regn_records: &HashMap<String, Region>,
    cell_records: &HashMap<CellKey, Cell>,
) -> Vec<Shape> {
    let shapes_len =
        (dimensions.max_x - dimensions.min_x + 1) * (dimensions.max_y - dimensions.min_y + 1);
    let mut shapes: Vec<Shape> = Vec::with_capacity(shapes_len as usize);

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            // get region
            let key = (cx, cy);
            if let Some(cell) = cell_records.get(&key) {
                if let Some(region_name) = &cell.region {
                    if let Some(region) = regn_records.get(region_name) {
                        let region_color = Color32::from_rgb(
                            region.map_color[0],
                            region.map_color[1],
                            region.map_color[2],
                        );

                        let rect = get_rect_at_cell(dimensions, to_screen, key);
                        let shape = Shape::rect_filled(rect, CornerRadius::default(), region_color);
                        shapes.push(shape);
                    }
                }
            }
        }
    }
    shapes
}
