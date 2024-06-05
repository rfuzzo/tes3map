use egui::Pos2;

use crate::{CellKey, GRID_SIZE};

#[derive(Debug, Clone)]
pub struct Dimensions {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,

    pub texture_size: usize,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            min_x: Default::default(),
            min_y: Default::default(),
            max_x: Default::default(),
            max_y: Default::default(),
            texture_size: 16,
        }
    }
}

impl Dimensions {
    pub fn cell_size(&self) -> usize {
        self.texture_size * GRID_SIZE
    }

    pub fn width(&self) -> usize {
        (1 + self.max_x - self.min_x).max(0) as usize
    }
    pub fn height(&self) -> usize {
        (1 + self.max_y - self.min_y).max(0) as usize
    }
    fn size(&self) -> usize {
        self.width() * self.height()
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

    pub fn tranform_to_cell_x(&self, x: i32) -> i32 {
        x + self.min_x
    }

    pub fn tranform_to_cell_y(&self, y: i32) -> i32 {
        self.max_y - y
    }

    pub fn tranform_to_canvas_x(&self, x: i32) -> usize {
        (x - self.min_x).max(0) as usize
    }

    pub fn tranform_to_canvas(&self, cell_key: CellKey) -> Pos2 {
        Pos2::new(
            self.tranform_to_canvas_x(cell_key.0) as f32,
            self.tranform_to_canvas_y(cell_key.1) as f32,
        )
    }

    pub fn tranform_to_canvas_y(&self, y: i32) -> usize {
        (self.max_y - y).max(0) as usize
    }

    pub fn stride(&self, pixel_per_cell: usize) -> usize {
        self.width() * pixel_per_cell
    }
}
