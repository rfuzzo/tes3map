use std::cmp::max;

use egui::Pos2;

use crate::{CellKey, CELL_WIDTH, GRID_SIZE};

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

    pub fn stride(&self, pixel_per_cell: usize) -> usize {
        self.width() * pixel_per_cell
    }

    pub fn get_max_texture_resolution(&self, max_texture_side: usize) -> usize {
        let max = max_texture_side / (GRID_SIZE * max(self.width(), self.height()));
        max.min(256)
    }

    pub fn canvas_to_cell(&self, pos: Pos2) -> CellKey {
        (pos.x as i32 + self.min_x, self.max_y - pos.y as i32)
    }

    // from cell to canvas
    pub fn cell_to_canvas_x(&self, x: i32) -> usize {
        (x - self.min_x).max(0) as usize
    }
    pub fn cell_to_canvas_y(&self, y: i32) -> usize {
        (self.max_y - y).max(0) as usize
    }
    pub fn cell_to_canvas(&self, cell_key: CellKey) -> Pos2 {
        Pos2::new(
            (cell_key.0 - self.min_x).max(0) as f32,
            (self.max_y - cell_key.1).max(0) as f32,
        )
    }

    // engine to canvas
    pub fn engine_to_canvas_x(&self, x: f32) -> f32 {
        (x - (self.min_x as f32)).max(0_f32)
    }

    pub fn engine_to_canvas_y(&self, y: f32) -> f32 {
        ((self.max_y as f32) + 1_f32 - y).max(0_f32)
    }

    pub fn engine_to_canvas(&self, p: Pos2) -> Pos2 {
        let x = self.engine_to_canvas_x(p.x / CELL_WIDTH);
        let y = self.engine_to_canvas_y(p.y / CELL_WIDTH);

        Pos2::new(x, y)
    }

    // canvas to engine
    pub fn canvas_to_engine_x(&self, x: f32) -> f32 {
        (x + self.min_x as f32) * CELL_WIDTH
    }
    pub fn canvas_to_engine_y(&self, y: f32) -> f32 {
        ((self.max_y as f32) + 1_f32 - y) * CELL_WIDTH
    }
    pub fn canvas_to_engine(&self, p: Pos2) -> Pos2 {
        let x = self.canvas_to_engine_x(p.x);
        let y = self.canvas_to_engine_y(p.y);

        Pos2::new(x, y)
    }
}
