use std::cmp::max;

use egui::Pos2;

use crate::{CellKey, GRID_SIZE};

#[derive(Debug, Clone, Default)]
pub struct Dimensions {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,

    pub min_z: f32,
    pub max_z: f32,
}

impl Dimensions {
    pub fn width(&self) -> usize {
        (1 + self.max_x - self.min_x).max(0) as usize
    }
    pub fn height(&self) -> usize {
        (1 + self.max_y - self.min_y).max(0) as usize
    }
    fn size(&self) -> usize {
        self.width() * self.height()
    }

    pub fn pixel_width(&self, pixel_per_cell: usize) -> usize {
        self.width() * pixel_per_cell
    }
    pub fn pixel_height(&self, pixel_per_cell: usize) -> usize {
        self.height() * pixel_per_cell
    }

    pub fn pixel_size(&self, pixel_per_cell: usize) -> usize {
        self.size() * pixel_per_cell * pixel_per_cell
    }

    pub fn pixel_size_tuple(&self, pixel_per_cell: usize) -> [usize; 2] {
        [
            self.width() * pixel_per_cell,
            self.height() * pixel_per_cell,
        ]
    }

    fn tranform_to_cell_x(&self, x: i32) -> i32 {
        x + self.min_x
    }

    fn tranform_to_cell_y(&self, y: i32) -> i32 {
        self.max_y - y
    }

    pub fn tranform_to_cell(&self, pos: Pos2) -> CellKey {
        (
            self.tranform_to_cell_x(pos.x as i32),
            self.tranform_to_cell_y(pos.y as i32),
        )
    }

    pub fn tranform_to_canvas_x(&self, x: i32) -> usize {
        (x - self.min_x).max(0) as usize
    }

    pub fn tranform_to_canvas_y(&self, y: i32) -> usize {
        (self.max_y - y).max(0) as usize
    }

    pub fn tranform_to_canvas(&self, cell_key: CellKey) -> Pos2 {
        Pos2::new(
            self.tranform_to_canvas_x(cell_key.0) as f32,
            self.tranform_to_canvas_y(cell_key.1) as f32,
        )
    }

    pub fn stride(&self, pixel_per_cell: usize) -> usize {
        self.width() * pixel_per_cell
    }

    pub fn get_max_texture_resolution(&self, max_texture_side: usize) -> usize {
        let max = max_texture_side / (GRID_SIZE * max(self.width(), self.height()));
        max.min(256)
    }
}
