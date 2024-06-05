#![warn(clippy::all, rust_2018_idioms)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use egui::{Color32, ColorImage, Pos2};
use image::{
    DynamicImage,
    error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind}, ImageError, RgbaImage,
};
use palette::{convert::FromColorUnclamped, Hsv, IntoColor, LinSrgb};
use seahash::hash;
use serde::{Deserialize, Serialize};
use tes3::esp::{Cell, EditorId, Landscape, LandscapeTexture, TES3Object, TypeInfo};

pub use app::TemplateApp;
use dimensions::Dimensions;

mod app;
mod background;
mod dimensions;
mod eframe_app;
mod overlay;
mod views;

const TEXTURE_MAX_SIZE: usize = 256;
const GRID_SIZE: usize = 16;
const VERTEX_CNT: usize = 65;
const DEFAULT_COLOR: Color32 = Color32::TRANSPARENT;

type CellKey = (i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EBackground {
    None,
    Landscape,
    #[default]
    HeightMap,
    GameMap,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SavedUiData {
    pub depth_spectrum: i32,
    pub depth_base: Color32,

    pub height_spectrum: i32,
    pub height_base: Color32,

    pub alpha: u8,

    // background
    // you can only have one background
    pub background: EBackground,

    // overlays
    // you can have multiple overlays
    pub overlay_paths: bool,
    pub overlay_region: bool,
    pub overlay_grid: bool,
    pub overlay_cities: bool,
    pub overlay_travel: bool,

    pub show_tooltips: bool,
    pub texture_size: usize,
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
            background: EBackground::default(),
            overlay_paths: false,
            overlay_region: false,
            overlay_grid: false,
            overlay_cities: false,
            overlay_travel: false,

            show_tooltips: false,

            texture_size: 16,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DimensionsZ {
    pub min_z: f32,
    pub max_z: f32,
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

#[derive(Default, Clone)]
pub struct PluginViewModel {
    pub hash: u64,
    pub path: PathBuf,
    pub enabled: bool,
}
impl PluginViewModel {
    pub fn get_name(&self) -> String {
        self.path.file_name().unwrap().to_string_lossy().to_string()
    }
    // from path
    pub fn from_path(path: PathBuf) -> Self {
        let hash = hash(path.to_str().unwrap_or_default().as_bytes());
        Self {
            hash,
            path,
            enabled: false,
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
    if let Ok(plugins) = fs::read_dir(path) {
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

fn get_color_for_height(value: f32, dimensions: DimensionsZ, ui_data: SavedUiData) -> Color32 {
    if value < dimensions.min_z {
        return Color32::TRANSPARENT;
    }

    if value < 0.0 {
        depth_to_color(value, dimensions, ui_data)
    } else {
        height_to_color(value, dimensions, ui_data)
    }
}

fn height_to_color(height: f32, dimensions: DimensionsZ, ui_data: SavedUiData) -> Color32 {
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

fn depth_to_color(depth: f32, dimensions: DimensionsZ, ui_data: SavedUiData) -> Color32 {
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

/// Get the height from the screen space.
pub fn height_from_screen_space(
    heights: &[f32],
    dimensions: &Dimensions,
    x: usize,
    y: usize,
) -> Option<f32> {
    let i = (y * dimensions.stride(VERTEX_CNT)) + x;
    heights.get(i).copied()
}

/// Overlay two colors with alpha.
fn overlay_colors_with_alpha(color1: Color32, color2: Color32, alpha1: f32) -> Color32 {
    let alpha2 = 1_f32 - alpha1;
    let r = (alpha1 * color1.r() as f32 + alpha2 * color2.r() as f32) as u8;
    let g = (alpha1 * color1.g() as f32 + alpha2 * color2.g() as f32) as u8;
    let b = (alpha1 * color1.b() as f32 + alpha2 * color2.b() as f32) as u8;

    Color32::from_rgba_premultiplied(r, g, b, 255)
}

fn create_image(
    pixels: &[f32],
    size: [usize; 2],
    dimensions_z: DimensionsZ,
    ui_data: SavedUiData,
) -> ColorImage {
    let mut img = ColorImage::new(size, Color32::WHITE);
    let p = pixels
        .iter()
        .map(|f| get_color_for_height(*f, dimensions_z, ui_data))
        .collect::<Vec<_>>();
    img.pixels = p;
    img
}

fn overlay_colors(color1: Color32, color2: Color32) -> Color32 {
    let alpha1 = color1.a() as f32 / 255.0;
    let alpha2 = color2.a() as f32 / 255.0;

    let r = ((1.0 - alpha2) * (alpha1 * color1.r() as f32 + alpha2 * color2.r() as f32)) as u8;
    let g = ((1.0 - alpha2) * (alpha1 * color1.g() as f32 + alpha2 * color2.g() as f32)) as u8;
    let b = ((1.0 - alpha2) * (alpha1 * color1.b() as f32 + alpha2 * color2.b() as f32)) as u8;
    let a = alpha1 * 255.0; // TODO HACK

    Color32::from_rgba_premultiplied(r, g, b, a as u8)
}

fn append_to_filename(path: &Path, suffix: &str) -> PathBuf {
    // Get the stem (filename without extension) and extension from the original path
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let extension = path.extension().map_or("", |ext| ext.to_str().unwrap());

    // Append a number to the stem (filename)
    let new_stem = format!("{}_{}", stem, suffix);

    // Create a new PathBuf with the modified stem and the same extension
    PathBuf::from(path.parent().unwrap()).join(format!("{}.{}", new_stem, extension))
}

fn save_image(path: &Path, color_image: &ColorImage) -> Result<(), ImageError> {
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

fn calculate_dimensions(
    landscape_records: &HashMap<CellKey, Landscape>,
    texture_size: usize,
) -> Option<Dimensions> {
    let mut min_x: Option<i32> = None;
    let mut min_y: Option<i32> = None;
    let mut max_x: Option<i32> = None;
    let mut max_y: Option<i32> = None;

    for key in landscape_records.keys() {
        // get grid dimensions
        let x = key.0;
        let y = key.1;

        if let Some(minx) = min_x {
            if x < minx {
                min_x = Some(x);
            }
        } else {
            min_x = Some(x);
        }
        if let Some(maxx) = max_x {
            if x > maxx {
                max_x = Some(x);
            }
        } else {
            max_x = Some(x);
        }
        if let Some(miny) = min_y {
            if y < miny {
                min_y = Some(y);
            }
        } else {
            min_y = Some(y);
        }
        if let Some(maxy) = max_y {
            if y > maxy {
                max_y = Some(y);
            }
        } else {
            max_y = Some(y);
        }
    }

    let min_y = min_y?;
    let max_y = max_y?;
    let min_x = min_x?;
    let max_x = max_x?;
    let dimensions = Dimensions {
        min_x,
        min_y,
        max_x,
        max_y,
        texture_size,
    };
    Some(dimensions)
}

pub fn get_layered_image(dimensions: &Dimensions, img: ColorImage, img2: ColorImage) -> ColorImage {
    // base image
    let mut layered = img.pixels.clone();

    // overlay second image
    for (i, color1) in img.pixels.into_iter().enumerate() {
        let color2 = img2.pixels[i];
        layered[i] = overlay_colors(color1, color2);
    }

    // create new colorImage
    let mut layered_img = ColorImage::new(
        dimensions.pixel_size_tuple(VERTEX_CNT),
        Color32::TRANSPARENT,
    );
    layered_img.pixels = layered;
    layered_img
}

fn load_texture(data_files: &Option<PathBuf>, ltex: &LandscapeTexture) -> Option<ColorImage> {
    // data files
    let data_files = data_files.as_ref()?;

    let texture = ltex.file_name.clone();
    let tex_path = data_files.join("Textures").join(texture);
    if !tex_path.exists() {
        return None;
    }

    // decode image
    if let Ok(mut reader) = image::io::Reader::open(&tex_path) {
        let ext = tex_path.extension().unwrap().to_string_lossy();
        if ext.contains("tga") {
            reader.set_format(image::ImageFormat::Tga);
        } else if ext.contains("dds") {
            reader.set_format(image::ImageFormat::Dds);
        } else {
            // not supported
            return None;
        }

        let Ok(image) = reader.decode() else {
            return None;
        };

        let size = [image.width() as _, image.height() as _];
        let image_buffer = image.to_rgba8();
        let pixels = image_buffer.as_flat_samples();
        return Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
    }

    None
}

pub fn get_cell_name(cells: &HashMap<CellKey, Cell>, pos: CellKey) -> String {
    let mut name = "".to_owned();
    if let Some(cell) = cells.get(&pos) {
        name.clone_from(&cell.name);
        if name.is_empty() {
            if let Some(region) = cell.region.clone() {
                name = region;
            }
        }
    }
    format!("{} ({},{})", name, pos.0, pos.1)
}

//////////////////////////////////////////
// TES3

/// creates a unique id from a record
/// we take the record tag + the record id
pub fn get_unique_id(record: &TES3Object) -> String {
    format!("{},{}", record.tag_str(), record.editor_id())
}
