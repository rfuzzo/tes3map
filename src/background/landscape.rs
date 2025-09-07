use std::collections::HashMap;

use egui::{Color32, ColorImage};
use log::info;
use tes3::esp::{Landscape, LandscapeFlags, LandscapeTexture};

use crate::{
    height_from_screen_space, overlay_colors_with_alpha, CellKey, Dimensions, ImageBuffer,
    LandscapeSettings, DEFAULT_COLOR, GRID_SIZE, VERTEX_CNT,
};

/// Compute a landscape image from the given landscape records and texture map.
pub fn compute_landscape_image(
    settings: &LandscapeSettings,
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
    ltex_records: &HashMap<u32, LandscapeTexture>,
    heights: &[f32],
    texture_map: &HashMap<String, ImageBuffer>,
) -> ColorImage {
    let d = dimensions;
    let texture_size = settings.texture_size;
    let cell_size = settings.cell_size();
    let width = d.pixel_width(cell_size);
    let height = d.pixel_height(cell_size);
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
                                    for x in 0..texture_size {
                                        for y in 0..texture_size {
                                            // let index = (y * texture_size) + x;
                                            // let mut color = texture.pixels[index];
                                            let pixel = texture.get_pixel(x as u32, y as u32);
                                            let mut color = Color32::from_rgba_premultiplied(
                                                pixel[0], pixel[1], pixel[2], pixel[3],
                                            );

                                            let tx = d.cell_to_canvas_x(cx) * cell_size
                                                + gx * texture_size
                                                + x;
                                            let ty = d.cell_to_canvas_y(cy) * cell_size
                                                + (GRID_SIZE - 1 - gy) * texture_size
                                                + y;

                                            // blend color when under water
                                            if settings.show_water {
                                                let screenx = tx * VERTEX_CNT / cell_size;
                                                let screeny = ty * VERTEX_CNT / cell_size;

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

                                            let i = (ty * d.stride(cell_size)) + tx;
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
                        for x in 0..texture_size {
                            for y in 0..texture_size {
                                let tx = d.cell_to_canvas_x(cx) * cell_size + gx * texture_size + x;
                                let ty = d.cell_to_canvas_y(cy) * cell_size + gy * texture_size + y;

                                let i = (ty * d.stride(cell_size)) + tx;

                                pixels_color[i] = DEFAULT_COLOR;
                            }
                        }
                    }
                }
            }
        }
    }

    ColorImage::new(d.pixel_size_tuple(cell_size), pixels_color)
}
