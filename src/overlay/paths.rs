use std::collections::HashMap;

use egui::Color32;
use tes3::esp::{Landscape, LandscapeFlags};

use crate::{CellKey, Dimensions, DEFAULT_COLOR, VERTEX_CNT};

pub fn color_map_to_pixels(
    dimensions: Dimensions,
    color_map: HashMap<CellKey, [[Color32; 65]; 65]>,
) -> Vec<Color32> {
    // dimensions
    let max_x = dimensions.max_x;
    let min_x = dimensions.min_x;
    let max_y = dimensions.max_y;
    let min_y = dimensions.min_y;

    let nx = dimensions.width() * VERTEX_CNT;
    let ny = dimensions.height() * VERTEX_CNT;
    let size = nx * ny;
    let mut pixels_color = vec![Color32::WHITE; size];

    for cy in min_y..max_y + 1 {
        for cx in min_x..max_x + 1 {
            let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx);
            let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy);

            if let Some(colors) = color_map.get(&(cx, cy)) {
                for (y, row) in colors.iter().rev().enumerate() {
                    for (x, value) in row.iter().enumerate() {
                        let tx = tx + x;
                        let ty = ty + y;

                        let i = (ty * nx) + tx;
                        pixels_color[i] = *value;
                    }
                }
            } else {
                for y in 0..VERTEX_CNT {
                    for x in 0..VERTEX_CNT {
                        let tx = tx + x;
                        let ty = ty + y;

                        let i = (ty * nx) + tx;

                        pixels_color[i] = DEFAULT_COLOR;
                    }
                }
            }
        }
    }

    pixels_color
}

pub fn color_pixels_reload(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
    alpha: u8,
) -> Vec<Color32> {
    let mut color_map: HashMap<CellKey, [[Color32; 65]; 65]> = HashMap::default();
    let d = dimensions.clone();

    for cy in d.min_y..d.max_y + 1 {
        for cx in d.min_x..d.max_x + 1 {
            if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                // get color data
                let mut colors: [[Color32; 65]; 65] =
                    [[Color32::TRANSPARENT; VERTEX_CNT]; VERTEX_CNT];

                if landscape
                    .landscape_flags
                    .contains(LandscapeFlags::USES_VERTEX_COLORS)
                {
                    let data = &landscape.vertex_colors.data.clone();

                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            let r = data[y][x][0];
                            let g = data[y][x][1];
                            let b = data[y][x][2];

                            let ratio = (r as f32 + g as f32 + b as f32) / (3_f32 * 255_f32);
                            let temp = (1_f32 - ratio).clamp(0.0, 1.0);

                            let c = alpha as f32 / 100_f32;
                            let alpha = if temp < c { temp / c } else { 1_f32 };

                            let rgb =
                                Color32::from_rgba_unmultiplied(r, g, b, (255_f32 * alpha) as u8);
                            colors[y][x] = rgb;
                        }
                    }
                }
                color_map.insert((cx, cy), colors);
            }
        }
    }

    color_map_to_pixels(d, color_map)
}
