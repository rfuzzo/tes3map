use std::collections::HashMap;

use egui::{Color32, ColorImage};
use log::info;
use tes3::esp::{Landscape, LandscapeFlags, LandscapeTexture};

use crate::{
    height_from_screen_space, overlay_colors_with_alpha, CellKey, Dimensions, LandscapeSettings,
    DEFAULT_COLOR, GRID_SIZE, VERTEX_CNT,
};

/// Compute a landscape image from the given landscape records and texture map.
pub fn compute_landscape_image(
    settings: &LandscapeSettings,
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
    ltex_records: &HashMap<u32, LandscapeTexture>,
    heights: &[f32],
    texture_map: &HashMap<String, ColorImage>,
) -> ColorImage {
    let d = dimensions;
    let width = d.pixel_width(d.cell_size());
    let height = d.pixel_height(d.cell_size());
    let size = width * height;

    info!(
        "Generating textured image with size {} (width: {}, height: {})",
        size, width, height,
    );

    let mut pixels_color = vec![Color32::TRANSPARENT; size];

    for cy in d.min_y..d.max_y + 1 {
        for cx in d.min_x..d.max_x + 1 {
            if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                if landscape
                    .landscape_flags
                    .contains(LandscapeFlags::USES_TEXTURES)
                {
                    {
                        let data = &landscape.texture_indices.data;
                        for gx in 0..GRID_SIZE {
                            for gy in 0..GRID_SIZE {
                                let dx = (4 * (gy % 4)) + (gx % 4);
                                let dy = (4 * (gy / 4)) + (gx / 4);

                                let key = data[dy][dx] as u32;
                                let mut texture_name = String::new();
                                if let Some(ltex) = ltex_records.get(&key) {
                                    texture_name.clone_from(&ltex.file_name);
                                }
                                if texture_name.is_empty() {
                                    continue;
                                }

                                if let Some(texture) = texture_map.get(&texture_name) {
                                    for x in 0..d.texture_size {
                                        for y in 0..d.texture_size {
                                            let index = (y * d.texture_size) + x;
                                            let mut color = texture.pixels[index];

                                            let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                                + gx * d.texture_size
                                                + x;
                                            let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                                + (GRID_SIZE - 1 - gy) * d.texture_size
                                                + y;

                                            // blend color when under water
                                            if settings.show_water {
                                                let screenx = tx * VERTEX_CNT / d.cell_size();
                                                let screeny = ty * VERTEX_CNT / d.cell_size();

                                                if let Some(height) = height_from_screen_space(
                                                    heights, d, screenx, screeny,
                                                ) {
                                                    if height < 0_f32 {
                                                        let a = 0.5;

                                                        if settings.remove_water {
                                                            color = Color32::TRANSPARENT;
                                                        } else {
                                                            color = overlay_colors_with_alpha(
                                                                color,
                                                                Color32::BLUE,
                                                                a,
                                                            );
                                                        }
                                                    }
                                                }
                                            }

                                            let i = (ty * d.stride(d.cell_size())) + tx;
                                            pixels_color[i] = color;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // no landscape
                for gx in 0..GRID_SIZE {
                    for gy in 0..GRID_SIZE {
                        // textures per tile
                        for x in 0..d.texture_size {
                            for y in 0..d.texture_size {
                                let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                    + gx * d.texture_size
                                    + x;
                                let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                    + gy * d.texture_size
                                    + y;

                                let i = (ty * d.stride(d.cell_size())) + tx;

                                pixels_color[i] = DEFAULT_COLOR;
                            }
                        }
                    }
                }
            }
        }
    }

    ColorImage {
        size: d.pixel_size_tuple(d.cell_size()),
        pixels: pixels_color,
    }
}
