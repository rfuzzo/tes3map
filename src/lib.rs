#![warn(clippy::all, rust_2018_idioms)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use egui::{emath::RectTransform, Color32, ColorImage, Pos2, Rect};
use image::{
    error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind},
    DynamicImage, ImageError, RgbaImage,
};
use log::{info, warn};
use seahash::hash;
use serde::{Deserialize, Serialize};
use tes3::esp::{
    Cell, EditorId, Landscape, LandscapeFlags, LandscapeTexture, TES3Object, TypeInfo,
};

pub use app::TemplateApp;
use dimensions::Dimensions;

use crate::app::TooltipInfo;

mod app;
mod background;
mod dimensions;
mod eframe_app;
mod overlay;
mod views;

const GRID_SIZE: usize = 16;
const VERTEX_CNT: usize = 65;
const DEFAULT_COLOR: Color32 = Color32::TRANSPARENT;
const CELL_WIDTH: f32 = 8192_f32;

type CellKey = (i32, i32);
type ImageBuffer = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EBackground {
    None,
    Landscape,
    HeightMap,
    #[default]
    GameMap,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct LandscapeSettings {
    pub texture_size: usize, // landscape
    pub show_water: bool,
    pub remove_water: bool,
}

impl LandscapeSettings {
    pub fn cell_size(&self) -> usize {
        self.texture_size * GRID_SIZE
    }
}

impl Default for LandscapeSettings {
    fn default() -> Self {
        Self {
            // landscape
            texture_size: 16,
            show_water: true,
            remove_water: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HeightmapSettings {
    pub depth_spectrum: i32,  // heightmap
    pub depth_base: Color32,  // heightmap
    pub height_spectrum: i32, // heightmap
    pub height_base: Color32, // heightmap
}

impl Default for HeightmapSettings {
    fn default() -> Self {
        Self {
            // heightmap
            height_spectrum: -120,
            height_base: Color32::from_rgb(0, 204, 0), // HSV(120,100,80)

            depth_spectrum: 70,
            depth_base: Color32::from_rgb(0, 204, 204), // HSV(180,100,80)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct SavedData {
    // background
    // you can only have one background
    pub background: EBackground,

    // overlays
    // you can have multiple overlays
    pub overlay_paths: bool,
    pub overlay_region: bool,
    pub overlay_grid: bool,
    pub overlay_cities: bool,
    pub overlay_conflicts: bool,
    pub overlay_travel: HashMap<String, bool>, // travel class

    pub show_tooltips: bool,

    pub realtime_update: bool,

    // settings
    pub landscape_settings: LandscapeSettings,
    pub heightmap_settings: HeightmapSettings,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeData {
    pub plugin_filter: String,
    pub cell_filter: String,

    pub info: TooltipInfo,

    pub selected_ids: Vec<CellKey>,
    pub pivot_id: Option<CellKey>,
    pub hover_pos: CellKey,
}

#[derive(Debug, Clone)]
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

/// Overlay two colors with alpha.
fn overlay_colors_with_alpha(color1: Color32, color2: Color32, alpha1: f32) -> Color32 {
    let alpha2 = 1_f32 - alpha1;
    let r = (alpha1 * color1.r() as f32 + alpha2 * color2.r() as f32) as u8;
    let g = (alpha1 * color1.g() as f32 + alpha2 * color2.g() as f32) as u8;
    let b = (alpha1 * color1.b() as f32 + alpha2 * color2.b() as f32) as u8;

    Color32::from_rgba_premultiplied(r, g, b, 255)
}

fn color_image_to_dynamic_image(color_image: &ColorImage) -> Result<DynamicImage, ImageError> {
    let pixels = color_image.as_raw();

    // Create an RgbaImage from the raw pixel data
    if let Some(img) = RgbaImage::from_raw(
        color_image.width() as u32,
        color_image.height() as u32,
        pixels.to_vec(),
    ) {
        // Convert the RgbaImage to a DynamicImage (required for saving as PNG)
        Ok(DynamicImage::ImageRgba8(img))
    } else {
        let e = ImageError::Unsupported(UnsupportedError::from_format_and_kind(
            ImageFormatHint::Name("".to_owned()),
            UnsupportedErrorKind::GenericFeature("".to_owned()),
        ));
        Err(e)
    }
}

fn calculate_dimensions(
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
) -> Option<Dimensions> {
    let keys = landscape_records.keys();

    let min_x = keys.clone().map(|k| k.0).min()?;
    let min_y = keys.clone().map(|k| k.1).min()?;
    let max_x = keys.clone().map(|k| k.0).max()?;
    let max_y = keys.clone().map(|k| k.1).max()?;

    Some(Dimensions {
        min_x,
        min_y,
        max_x,
        max_y,
        min_z: dimensions.min_z,
        max_z: dimensions.max_z,
    })
}

fn load_texture(
    data_files: &Option<PathBuf>,
    ltex: &LandscapeTexture,
) -> Result<DynamicImage, ImageError> {
    let data_files = match data_files {
        Some(p) => p,
        None => {
            warn!("Data files not found");
            return Err(ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Data files not found",
            )));
        }
    };

    let _tex_path = data_files.join("Textures").join(ltex.file_name.clone());

    let tga_path = _tex_path.with_extension("tga");
    let dds_path = _tex_path.with_extension("dds");
    let bmp_path = _tex_path.with_extension("bmp");

    if !tga_path.exists() && !dds_path.exists() && !bmp_path.exists() {
        return Err(ImageError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Texture not found",
        )));
    }

    // decode image
    if dds_path.exists() {
        decode_image(dds_path)
    } else if tga_path.exists() {
        decode_image(tga_path)
    } else if bmp_path.exists() {
        decode_image(bmp_path)
    } else {
        Err(ImageError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Texture not found",
        )))
    }
}

fn decode_image(tex_path: PathBuf) -> Result<DynamicImage, ImageError> {
    let mut reader = image::ImageReader::open(&tex_path)?;
    let ext = tex_path
        .extension()
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    if ext.contains("tga") {
        reader.set_format(image::ImageFormat::Tga);
    } else if ext.contains("dds") {
        reader.set_format(image::ImageFormat::Dds);
    } else if ext.contains("bmp") {
        reader.set_format(image::ImageFormat::Bmp);
    } else {
        return Err(ImageError::Unsupported(
            UnsupportedError::from_format_and_kind(
                ImageFormatHint::Name(ext.clone()),
                UnsupportedErrorKind::Format(ImageFormatHint::Name(ext)),
            ),
        ));
    }

    reader.decode()
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

pub fn calculate_heights(
    landscape_records: &HashMap<CellKey, Landscape>,
    dimensions: &mut Dimensions,
) -> Option<Vec<f32>> {
    let mut min_z: Option<f32> = None;
    let mut max_z: Option<f32> = None;
    let mut heights_map: HashMap<CellKey, [[f32; 65]; 65]> = HashMap::default();

    for cy in dimensions.min_y..dimensions.max_y + 1 {
        for cx in dimensions.min_x..dimensions.max_x + 1 {
            if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                if landscape
                    .landscape_flags
                    .contains(LandscapeFlags::USES_VERTEX_HEIGHTS_AND_NORMALS)
                {
                    // get vertex data
                    // get data
                    let data = &landscape.vertex_heights.data;
                    let mut heights: [[f32; 65]; 65] = [[0.0; VERTEX_CNT]; VERTEX_CNT];
                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            heights[y][x] = data[y][x] as f32;
                        }
                    }

                    // decode
                    let mut offset: f32 = landscape.vertex_heights.offset;
                    for row in heights.iter_mut().take(VERTEX_CNT) {
                        for x in row.iter_mut().take(VERTEX_CNT) {
                            offset += *x;
                            *x = offset;
                        }
                        offset = row[0];
                    }

                    for row in &mut heights {
                        for height in row {
                            *height *= 8.0;

                            let z = *height;
                            if let Some(minz) = min_z {
                                if z < minz {
                                    min_z = Some(z);
                                }
                            } else {
                                min_z = Some(z);
                            }
                            if let Some(maxz) = max_z {
                                if z > maxz {
                                    max_z = Some(z);
                                }
                            } else {
                                max_z = Some(z);
                            }
                        }
                    }

                    heights_map.insert((cx, cy), heights);
                }
            }
        }
    }

    let min_z = min_z?;
    let max_z = max_z?;
    dimensions.min_z = min_z;
    dimensions.max_z = max_z;

    let heights = height_map_to_pixel_heights(dimensions, heights_map);

    Some(heights)
}

fn height_map_to_pixel_heights(
    dimensions: &Dimensions,
    heights_map: HashMap<CellKey, [[f32; 65]; 65]>,
) -> Vec<f32> {
    // dimensions
    let max_x = dimensions.max_x;
    let min_x = dimensions.min_x;
    let max_y = dimensions.max_y;
    let min_y = dimensions.min_y;
    let min_z = dimensions.min_z;

    let size = dimensions.pixel_size(VERTEX_CNT);
    // hack to paint unset tiles
    let mut pixels = vec![min_z - 1_f32; size];

    for cy in min_y..max_y + 1 {
        for cx in min_x..max_x + 1 {
            if let Some(heights) = heights_map.get(&(cx, cy)) {
                // look up heightmap
                for (y, row) in heights.iter().rev().enumerate() {
                    for (x, value) in row.iter().enumerate() {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let i = (ty * dimensions.stride(VERTEX_CNT)) + tx;
                        pixels[i] = *value;
                    }
                }
            } else {
                for y in 0..VERTEX_CNT {
                    for x in 0..VERTEX_CNT {
                        let tx = VERTEX_CNT * dimensions.tranform_to_canvas_x(cx) + x;
                        let ty = VERTEX_CNT * dimensions.tranform_to_canvas_y(cy) + y;

                        let i = (ty * dimensions.stride(VERTEX_CNT)) + tx;
                        pixels[i] = min_z - 1_f32;
                    }
                }
            }
        }
    }

    pixels
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

fn get_rect_at_cell(dimensions: &Dimensions, to_screen: RectTransform, key: CellKey) -> Rect {
    let p00 = dimensions.tranform_to_canvas(key);
    let p11 = Pos2::new(p00.x + 1.0, p00.y + 1.0);
    Rect::from_two_pos(to_screen * p00, to_screen * p11)
}

//////////////////////////////////////////
// TES3

/// creates a unique id from a record
/// we take the record tag + the record id
pub fn get_unique_id(record: &TES3Object) -> String {
    format!("{},{}", record.tag_str(), record.editor_id())
}
