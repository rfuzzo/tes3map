use std::collections::HashMap;

use egui::{Color32, ColorImage};
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

// adapted from https://github.com/GNOME/gimp/blob/b0d16685940a5c17689166b7d42c8aa61f39ebb4/app/operations/layer-modes/gimpoperationlayermode-blend.c#L1180
fn color_erase_white(color: Color32) -> Color32 {
    let epsilon = 0.000001;
    let mut alpha = 0.0f32;
    let bgcolor = Color32::WHITE;

    for c in 0..3 {
        let r = color[c] as f32 / 255.0;
        let bgr = bgcolor[c] as f32 / 255.0;

        let col = r.clamp(0.0, 1.0);
        let bgcol = bgr.clamp(0.0, 1.0);

        if (col - bgcol).abs() > epsilon {
            let a = if col > bgcol {
                (col - bgcol) / (1.0 - bgcol)
            } else {
                (bgcol - col) / bgcol
            };

            alpha = alpha.max(a);
        }
    }
    let mut comp = [0.0f32; 4];

    if alpha > epsilon {
        let alpha_inv = 1.0 / alpha;

        for c in 0..3 {
            let color_c = color[c] as f32 / 255.0;
            let bgcolor_c = bgcolor[c] as f32 / 255.0;
            comp[c] = (color_c - bgcolor_c) * alpha_inv + bgcolor_c;
        }
    } else {
        comp[0] = 0.0;
        comp[1] = 0.0;
        comp[2] = 0.0;
    }

    comp[3] = alpha;

    // to color32
    let r = (comp[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g = (comp[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b = (comp[2] * 255.0).clamp(0.0, 255.0) as u8;
    let a = (comp[3] * 255.0).clamp(0.0, 255.0) as u8;

    Color32::from_rgba_unmultiplied(r, g, b, a)
}

pub fn get_overlay_path_image(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
) -> ColorImage {
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

                            let mut rgb = Color32::from_rgba_unmultiplied(r, g, b, 255);
                            // subtract white from rgb
                            rgb = color_erase_white(rgb);

                            colors[y][x] = rgb;
                        }
                    }
                }
                color_map.insert((cx, cy), colors);
            }
        }
    }

    ColorImage {
        pixels: color_map_to_pixels(d, color_map),
        size: dimensions.pixel_size_tuple(VERTEX_CNT),
    }
}
