#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod eframe_app;

pub use app::TemplateApp;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use egui::{Color32, ColorImage, Pos2};
use image::{
    error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind},
    DynamicImage, ImageError, RgbaImage,
};
use palette::{convert::FromColorUnclamped, Hsv, IntoColor, LinSrgb};
use serde::{Deserialize, Serialize};

const TEXTURE_SIZE: usize = 64; //256;
const GRID_SIZE: usize = 16;

const CELL_SIZE: usize = TEXTURE_SIZE * GRID_SIZE;

const VERTEX_CNT: usize = 65;
const DEFAULT_COLOR: Color32 = Color32::TRANSPARENT;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SavedUiData {
    pub depth_spectrum: i32,
    pub depth_base: Color32,

    pub height_spectrum: i32,
    pub height_base: Color32,

    pub alpha: u8,

    pub overlay_terrain: bool,
    pub overlay_paths: bool,
    pub overlay_textures: bool,
    pub show_tooltips: bool,
}

impl Default for SavedUiData {
    fn default() -> Self {
        Self {
            // map color settings
            height_spectrum: -120,
            height_base: Color32::from_rgb(0, 204, 0), // HSV(120,100,80)

            depth_spectrum: 70,
            depth_base: Color32::from_rgb(0, 204, 204), // HSV(180,100,80)

            alpha: 100,

            // overlays
            overlay_terrain: true,
            overlay_paths: true,
            overlay_textures: false,
            show_tooltips: false,
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

impl Dimensions {
    fn width(&self) -> usize {
        (1 + self.max_x - self.min_x).max(0) as usize
    }
    fn height(&self) -> usize {
        (1 + self.max_y - self.min_y).max(0) as usize
    }
    fn size(&self) -> usize {
        self.width() * self.height()
    }

    fn pixel_size(&self, pixel_per_cell: usize) -> usize {
        self.size() * pixel_per_cell * pixel_per_cell
    }

    fn pixel_size_tuple(&self, pixel_per_cell: usize) -> [usize; 2] {
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

    fn tranform_to_canvas_x(&self, x: i32) -> usize {
        (x - self.min_x).max(0) as usize
    }

    fn tranform_to_canvas_y(&self, y: i32) -> usize {
        (self.max_y - y).max(0) as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ZoomData {
    drag_start: Pos2,
    drag_delta: Option<Pos2>,
    drag_offset: Pos2,

    zoom: f32,
    zoom_delta: Option<f32>,
}

impl Default for ZoomData {
    fn default() -> Self {
        Self {
            drag_start: Default::default(),
            drag_delta: Default::default(),
            drag_offset: Default::default(),
            zoom: 1.0,
            zoom_delta: Default::default(),
        }
    }
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

fn get_color_for_height(value: f32, dimensions: Dimensions, ui_data: SavedUiData) -> Color32 {
    if value < dimensions.min_z {
        return Color32::TRANSPARENT;
    }

    if value < 0.0 {
        depth_to_color(value, dimensions, ui_data)
    } else {
        height_to_color(value, dimensions, ui_data)
    }
}

fn height_to_color(height: f32, dimensions: Dimensions, ui_data: SavedUiData) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        ui_data.height_base.r(),
        ui_data.height_base.g(),
        ui_data.height_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the height to the range [0.0, 1.0]
    let normalized_height = height / dimensions.max_z;

    let hue = base.hue + normalized_height * ui_data.height_spectrum as f32;
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

fn depth_to_color(depth: f32, dimensions: Dimensions, ui_data: SavedUiData) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        ui_data.depth_base.r(),
        ui_data.depth_base.g(),
        ui_data.depth_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the depth to the range [0.0, 1.0]
    let normalized_depth = depth / dimensions.min_z;

    let hue = base.hue + normalized_depth * ui_data.depth_spectrum as f32;
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

fn color_map_to_pixels(
    dimensions: Dimensions,
    color_map: HashMap<(i32, i32), [[Color32; 65]; 65]>,
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

fn height_map_to_pixel_heights(
    dimensions: Dimensions,
    heights_map: HashMap<(i32, i32), [[f32; 65]; 65]>,
) -> Vec<f32> {
    // dimensions
    let max_x = dimensions.max_x;
    let min_x = dimensions.min_x;
    let max_y = dimensions.max_y;
    let min_y = dimensions.min_y;

    let size = dimensions.pixel_size(VERTEX_CNT);
    // TODO hack to paint unset tiles
    let mut pixels = vec![dimensions.min_z - 1.0; size];

    for cy in min_y..max_y + 1 {
        for cx in min_x..max_x + 1 {
            if let Some(heights) = heights_map.get(&(cx, cy)) {
                // look up heightmap
                for (y, row) in heights.iter().rev().enumerate() {
                    for (x, value) in row.iter().enumerate() {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let stride = dimensions.width() * VERTEX_CNT;
                        let i = (ty * stride) + tx;
                        pixels[i] = *value;
                    }
                }
            } else {
                for y in 0..VERTEX_CNT {
                    for x in 0..VERTEX_CNT {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let stride = dimensions.width() * VERTEX_CNT;
                        let i = (ty * stride) + tx;
                        pixels[i] = dimensions.min_z - 1.0;
                    }
                }
            }
        }
    }

    pixels
}

fn create_image(
    pixels: &[f32],
    size: [usize; 2],
    dimensions: Dimensions,
    ui_data: SavedUiData,
) -> ColorImage {
    let mut img = ColorImage::new(size, Color32::WHITE);
    let p = pixels
        .iter()
        .map(|f| get_color_for_height(*f, dimensions, ui_data))
        .collect::<Vec<_>>();
    img.pixels = p;
    img
}

fn overlay_colors(
    color1: (u8, u8, u8),
    alpha1: f32,
    color2: (u8, u8, u8),
    alpha2: f32,
) -> (u8, u8, u8, u8) {
    let r = ((1.0 - alpha2) * (alpha1 * color1.0 as f32 + alpha2 * color2.0 as f32)) as u8;
    let g = ((1.0 - alpha2) * (alpha1 * color1.1 as f32 + alpha2 * color2.1 as f32)) as u8;
    let b = ((1.0 - alpha2) * (alpha1 * color1.2 as f32 + alpha2 * color2.2 as f32)) as u8;
    let a = alpha1 * 255.0; // TODO HACK

    (r, g, b, a as u8)
}

fn append_number_to_filename(path: &Path, number: usize) -> PathBuf {
    // Get the stem (filename without extension) and extension from the original path
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let extension = path.extension().map_or("", |ext| ext.to_str().unwrap());

    // Append a number to the stem (filename)
    let new_stem = format!("{}_{}", stem, number);

    // Create a new PathBuf with the modified stem and the same extension
    PathBuf::from(path.parent().unwrap()).join(format!("{}.{}", new_stem, extension))
}

fn save_image(path: PathBuf, color_image: &ColorImage) -> Result<(), image::ImageError> {
    // get image

    let pixels = color_image.as_raw();

    // Create an RgbaImage from the raw pixel data
    if let Some(img) = RgbaImage::from_raw(
        color_image.width() as u32,
        color_image.height() as u32,
        pixels.to_vec(),
    ) {
        // Convert the RgbaImage to a DynamicImage (required for saving as PNG)
        let dynamic_img = DynamicImage::ImageRgba8(img);
        dynamic_img.save(path)?;
        Ok(())
    } else {
        let e = ImageError::Unsupported(UnsupportedError::from_format_and_kind(
            ImageFormatHint::Name("".to_owned()),
            UnsupportedErrorKind::GenericFeature("".to_owned()),
        ));
        Err(e)
    }
}
