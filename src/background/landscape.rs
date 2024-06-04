use std::collections::HashMap;

use egui::{Color32, ColorImage};
use log::info;
use tes3::esp::{Landscape, LandscapeFlags};

use crate::{
    height_from_screen_space, overlay_colors_with_alpha, CellKey, Dimensions, DEFAULT_COLOR,
    GRID_SIZE, TEXTURE_MAX_SIZE, VERTEX_CNT,
};

/// Compute a landscape image from the given landscape records and texture map.
pub fn compute_landscape_image(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, (u64, Landscape)>,
    texture_map: &HashMap<(u64, u32), ColorImage>,
    heights: &[f32],
) -> Option<ColorImage> {
    let d = dimensions;
    let size = d.pixel_size(d.cell_size());
    let size_tuple = d.pixel_size_tuple(d.cell_size());
    let width = size_tuple[0];
    let height = size_tuple[1];
    info!(
        "Generating textured image with size {} (width: {}, height: {})",
        size, width, height,
    );

    let mut pixels_color = vec![Color32::TRANSPARENT; size];

    for cy in d.min_y..d.max_y + 1 {
        for cx in d.min_x..d.max_x + 1 {
            if let Some((hash, landscape)) = landscape_records.get(&(cx, cy)) {
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
                                let Some(color_image) = texture_map.get(&(*hash, key)) else {
                                    continue;
                                };

                                // textures per tile
                                for x in 0..d.texture_size {
                                    for y in 0..d.texture_size {
                                        let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                            + gx * d.texture_size
                                            + x;
                                        let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                            + (GRID_SIZE - 1 - gy) * d.texture_size
                                            + y;

                                        let i = (ty * d.stride(d.cell_size())) + tx;

                                        // pick every nth pixel from the texture to downsize
                                        let sx = x * (TEXTURE_MAX_SIZE / d.texture_size);
                                        let sy = y * (TEXTURE_MAX_SIZE / d.texture_size);
                                        let index = (sy * d.texture_size) + sx;

                                        let mut color = color_image.pixels[index];

                                        // blend color when under water
                                        let screenx = tx * VERTEX_CNT / d.cell_size();
                                        let screeny = ty * VERTEX_CNT / d.cell_size();

                                        if let Some(height) = height_from_screen_space(
                                            heights, dimensions, screenx, screeny,
                                        ) {
                                            if height < 0_f32 {
                                                let a = 0.5;

                                                color = overlay_colors_with_alpha(
                                                    color,
                                                    Color32::BLUE,
                                                    a,
                                                );
                                            }
                                        }

                                        pixels_color[i] = color;
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

    let mut img = ColorImage::new(d.pixel_size_tuple(d.cell_size()), Color32::GOLD);
    img.pixels = pixels_color;
    Some(img)
}
