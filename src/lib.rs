#![warn(clippy::all, rust_2018_idioms)]

mod app;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub use app::TemplateApp;
use egui::{Color32, ColorImage};
use palette::{convert::FromColorUnclamped, Hsv, IntoColor, LinSrgb};
use serde::{Deserialize, Serialize};

const VERTEX_CNT: usize = 65;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UiData {
    pub depth_spectrum: usize,
    pub depth_base: Color32,
    pub height_spectrum: usize,
    pub height_base: Color32,
}

impl Default for UiData {
    fn default() -> Self {
        Self {
            depth_spectrum: 20,
            depth_base: Color32::BLUE,
            height_spectrum: 120,
            height_base: Color32::DARK_GREEN,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Dimensions {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub min_z: f32,
    pub max_z: f32,
}

/// Get all plugins (esp, omwaddon, omwscripts) in a folder
fn get_plugins_in_folder<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get all plugins
    let mut results: Vec<PathBuf> = vec![];
    if let Ok(plugins) = std::fs::read_dir(path) {
        plugins.for_each(|p| {
            if let Ok(file) = p {
                let file_path = file.path();
                if file_path.is_file() {
                    if let Some(ext_os) = file_path.extension() {
                        let ext = ext_os.to_ascii_lowercase();
                        if ext == "esm"
                            || ext == "esp"
                            || (use_omw_plugins && ext == "omwaddon")
                            || (use_omw_plugins && ext == "omwscripts")
                        {
                            results.push(file_path);
                        }
                    }
                }
            }
        });
    }
    results
}

fn get_plugins_sorted<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get plugins
    let mut plugins = get_plugins_in_folder(path, use_omw_plugins);

    // sort
    plugins.sort_by(|a, b| {
        fs::metadata(a.clone())
            .expect("filetime")
            .modified()
            .unwrap()
            .cmp(
                &fs::metadata(b.clone())
                    .expect("filetime")
                    .modified()
                    .unwrap(),
            )
    });

    plugins
}

fn get_color_for_height(value: f32, dimensions: Dimensions, ui_data: UiData) -> Color32 {
    if value < 0.0 {
        depth_to_color(value, dimensions, ui_data)
    } else {
        height_to_color(value, dimensions, ui_data)
    }
}

fn height_to_color(height: f32, dimensions: Dimensions, ui_data: UiData) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        ui_data.height_base.r(),
        ui_data.height_base.g(),
        ui_data.height_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the height to the range [0.0, 1.0]
    let normalized_height = height / dimensions.max_z;

    // Map normalized height to hue in the range [120.0, 30.0] (green to brown)
    // let hue = 120.0 - normalized_height * self.height_spectrum as f32;
    // let saturation = 1.0;
    // let value = 0.65;

    let hue = base.hue - normalized_height * ui_data.height_spectrum as f32;
    let saturation = base.saturation;
    let value = 0.65;
    //base.value;

    // Create an HSV color
    let color = Hsv::new(hue, saturation, value);

    // Convert HSV to linear RGB
    let linear_rgb: LinSrgb = color.into_color();

    // Convert linear RGB to gamma-corrected RGB
    let c: LinSrgb<u8> = linear_rgb.into_format();

    Color32::from_rgb(c.red, c.green, c.blue)
}

fn depth_to_color(depth: f32, dimensions: Dimensions, ui_data: UiData) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        ui_data.depth_base.r(),
        ui_data.depth_base.g(),
        ui_data.depth_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the depth to the range [0.0, 1.0]
    let normalized_depth = -depth / dimensions.min_z;

    // Map normalized depth to hue in the range [240.0, 180.0] (blue to light blue)
    // let hue = 240.0 - normalized_depth * depth_spectrum as f32;
    // let saturation = 1.0;
    // let value = 0.8;

    let hue = base.hue - normalized_depth * ui_data.depth_spectrum as f32;
    let saturation = base.saturation;
    let value = base.value;

    // Create an HSV color
    let color = Hsv::new(hue, saturation, value);

    // Convert HSV to linear RGB
    let linear_rgb: LinSrgb = color.into_color();

    // Convert linear RGB to gamma-corrected RGB
    let c: LinSrgb<u8> = linear_rgb.into_format();
    Color32::from_rgb(c.red, c.green, c.blue)
}

impl Dimensions {
    fn nx(&self) -> i32 {
        (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32)
    }
    fn ny(&self) -> i32 {
        (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32)
    }
    fn size(&self) -> [usize; 2] {
        [self.nx() as usize, self.ny() as usize]
    }

    fn tranform_to_cell_x(&self, x: i32) -> i32 {
        x + self.min_x
    }

    fn tranform_to_cell_y(&self, y: i32) -> i32 {
        self.max_y - y
    }

    fn tranform_to_canvas_x(&self, x: i32) -> i32 {
        x - self.min_x
    }

    fn tranform_to_canvas_y(&self, y: i32) -> i32 {
        self.max_y - y
    }
}

fn generate_pixels(
    dimensions: Dimensions,
    heights_map: HashMap<(i32, i32), [[f32; 65]; 65]>,
) -> Vec<f32> {
    // dimensions
    let max_x = dimensions.max_x;
    let min_x = dimensions.min_x;
    let max_y = dimensions.max_y;
    let min_y = dimensions.min_y;

    let nx = dimensions.nx();
    let ny = dimensions.ny();
    let size = nx as usize * ny as usize;
    let mut pixels = vec![-1.0; size];

    for cy in min_y..max_y + 1 {
        for cx in min_x..max_x + 1 {
            let tx = VERTEX_CNT as i32 * dimensions.tranform_to_canvas_x(cx);
            let ty = VERTEX_CNT as i32 * dimensions.tranform_to_canvas_y(cy);

            if let Some(heights) = heights_map.get(&(cx, cy)) {
                // look up heightmap
                for (y, row) in heights.iter().rev().enumerate() {
                    for (x, value) in row.iter().enumerate() {
                        let tx = tx + x as i32;
                        let ty = ty + y as i32;

                        let i = (ty * nx) + tx;
                        if i as usize > size {
                            panic!();
                        }
                        pixels[i as usize] = *value;
                    }
                }
            } else {
                for y in 0..VERTEX_CNT {
                    for x in 0..VERTEX_CNT {
                        let tx = tx + x as i32;
                        let ty = ty + y as i32;

                        let i = (ty * nx) + tx;
                        if i as usize > size {
                            panic!();
                        }
                        pixels[i as usize] = -1.0;
                    }
                }
            }
        }
    }

    pixels
}

fn create_image(pixels: &[f32], dimensions: Dimensions, ui_data: UiData) -> ColorImage {
    let mut img = ColorImage::new(dimensions.size(), Color32::BLUE);
    let p = pixels
        .iter()
        .map(|f| get_color_for_height(*f, dimensions, ui_data))
        .collect::<Vec<_>>();
    img.pixels = p;
    img
}
