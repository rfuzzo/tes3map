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
use log::info;
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

const TEXTURE_MAX_SIZE: usize = 256;
const GRID_SIZE: usize = 16;
const VERTEX_CNT: usize = 65;
const DEFAULT_COLOR: Color32 = Color32::TRANSPARENT;

type CellKey = (i32, i32);

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
}

impl Default for LandscapeSettings {
    fn default() -> Self {
        Self {
            // landscape
            texture_size: 16,
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
    pub overlay_travel: bool,
    pub overlay_conflicts: bool,

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

    pub selected_id: Option<CellKey>,
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

fn overlay_colors(color1: Color32, color2: Color32) -> Color32 {
    let alpha1 = color1.a() as f32 / 255.0;
    let alpha2 = color2.a() as f32 / 255.0;

    let r = ((1.0 - alpha2) * (alpha1 * color1.r() as f32 + alpha2 * color2.r() as f32)) as u8;
    let g = ((1.0 - alpha2) * (alpha1 * color1.g() as f32 + alpha2 * color2.g() as f32)) as u8;
    let b = ((1.0 - alpha2) * (alpha1 * color1.b() as f32 + alpha2 * color2.b() as f32)) as u8;
    let a = alpha1 * 255.0; // TODO HACK

    Color32::from_rgba_premultiplied(r, g, b, a as u8)
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
    dimensions: &Dimensions,
    landscape_records: &HashMap<CellKey, Landscape>,
    texture_size: usize,
) -> Dimensions {
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

    let min_y = min_y.unwrap();
    let max_y = max_y.unwrap();
    let min_x = min_x.unwrap();
    let max_x = max_x.unwrap();

    Dimensions {
        min_x,
        min_y,
        max_x,
        max_y,
        texture_size,
        min_z: dimensions.min_z,
        max_z: dimensions.max_z,
    }
}

pub fn get_layered_image(dimensions: &Dimensions, img: ColorImage, img2: ColorImage) -> ColorImage {
    // log size
    let size1 = img.size;
    let size2 = img2.size;
    info!("size 1 {:?} size 2 {:?}", size1, size2);

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

//////////////////////////////////////////
// TES3

/// creates a unique id from a record
/// we take the record tag + the record id
pub fn get_unique_id(record: &TES3Object) -> String {
    format!("{},{}", record.tag_str(), record.editor_id())
}
