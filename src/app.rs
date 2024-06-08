use std::{collections::HashMap, path::PathBuf};

use egui::{pos2, ColorImage, Pos2, Shape};
use image::{imageops, ImageError};
use log::{debug, error};
use overlay::{
    paths::{self, get_overlay_path_image},
    regions::get_region_shapes,
};
use tes3::esp::{Landscape, Region};

use background::{
    gamemap::generate_map, heightmap::generate_heightmap, landscape::compute_landscape_image,
};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ESidePanelView {
    #[default]
    Plugins,
    Cells,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TooltipInfo {
    pub key: CellKey,
    pub height: f32,
    pub region: String,
    pub cell_name: String,
    pub conflicts: Vec<u64>,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TemplateApp {
    pub data_files: Option<PathBuf>,
    pub ui_data: SavedData,

    // ui
    #[serde(skip)]
    pub zoom_data: ZoomData,
    #[serde(skip)]
    pub dimensions: Dimensions,

    // tes3
    #[serde(skip)]
    pub plugins: Option<Vec<PluginViewModel>>,

    // runtime data
    #[serde(skip)]
    pub land_records: HashMap<CellKey, Landscape>,
    #[serde(skip)]
    pub ltex_records: HashMap<u32, LandscapeTexture>,
    #[serde(skip)]
    pub regn_records: HashMap<String, Region>,
    #[serde(skip)]
    pub cell_records: HashMap<CellKey, Cell>,

    // overlays
    #[serde(skip)]
    pub travel_edges: HashMap<String, Vec<(CellKey, CellKey)>>,
    #[serde(skip)]
    pub cell_conflicts: HashMap<CellKey, Vec<u64>>,
    // textures in memory
    #[serde(skip)]
    pub background_handle: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub paths_handle: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub heights: Vec<f32>,
    #[serde(skip)]
    pub texture_map_resolution: usize,
    #[serde(skip)]
    pub texture_map: HashMap<String, ImageBuffer>,

    // runtime data
    #[serde(skip)]
    pub side_panel_view: ESidePanelView,
    #[serde(skip)]
    pub runtime_data: RuntimeData,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    pub fn reload_paths(&mut self, ctx: &egui::Context) {
        let image = get_overlay_path_image(&self.dimensions, &self.land_records);
        self.paths_handle = Some(ctx.load_texture("paths", image, Default::default()));
    }

    pub fn populate_texture_map(&mut self, max_texture_side: usize) {
        // if the resolution is the same, no need to reload
        let max_texture_resolution = self.dimensions.get_max_texture_resolution(max_texture_side);
        if max_texture_resolution > self.texture_map_resolution
            && self.texture_map_resolution == self.ui_data.landscape_settings.texture_size
        {
            debug!("Texture resolution is the same, no need to reload");
            return;
        }

        // otherwise check if possible
        let cell_size = self.ui_data.landscape_settings.cell_size();
        let width = self.dimensions.pixel_width(cell_size);
        let height = self.dimensions.pixel_height(cell_size);
        if width > max_texture_side || height > max_texture_side {
            error!(
                "Texture size too large: (width: {}, height: {}), supported side: {}, max_texture_side: {}",
                width, height, max_texture_side, max_texture_resolution
            );

            debug!(
                "texture_size {}",
                self.ui_data.landscape_settings.texture_size
            );
            debug!("Resetting texture size to 16");

            self.ui_data.landscape_settings.texture_size = 16.min(max_texture_resolution);

            // rfd messagebox
            let msg = format!(
                "Texture size too large, supported side: {}",
                max_texture_resolution
            );
            rfd::MessageDialog::new()
                .set_title("Error")
                .set_description(msg)
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }

        self.texture_map.clear();
        let texture_size = self.ui_data.landscape_settings.texture_size;
        self.texture_map_resolution = texture_size;

        debug!("Populating texture map with resolution: {}", texture_size);

        for cy in self.dimensions.min_y..self.dimensions.max_y + 1 {
            for cx in self.dimensions.min_x..self.dimensions.max_x + 1 {
                if let Some(landscape) = self.land_records.get(&(cx, cy)) {
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

                                    // load texture
                                    if let Some(ltex) = self.ltex_records.get(&key) {
                                        // texture name
                                        let texture_name = ltex.file_name.clone();
                                        if self.texture_map.contains_key(&texture_name) {
                                            continue;
                                        }

                                        if let Ok(tex) = load_texture(&self.data_files, ltex) {
                                            // resize the image
                                            let image = image::imageops::resize(
                                                &tex,
                                                texture_size as u32,
                                                texture_size as u32,
                                                image::imageops::FilterType::CatmullRom,
                                            );

                                            // let size = [image.width() as _, image.height() as _];
                                            // let image_buffer = image.to_rgba8();
                                            // let pixels = image_buffer.as_flat_samples();
                                            // return Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));

                                            info!("Loaded texture: {}", ltex.file_name);
                                            self.texture_map.insert(texture_name, image);
                                        } else {
                                            error!("Failed to load texture: {}", ltex.file_name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Assigns landscape_records, dimensions and pixels
    pub fn reload_background(
        &mut self,
        ctx: &egui::Context,
        new_dimensions: Option<Dimensions>,
        recalculate_dimensions: bool,
        recalculate_heights: bool,
    ) {
        // calculate dimensions
        if let Some(dimensions) = new_dimensions {
            self.dimensions = dimensions.clone();
        } else if recalculate_dimensions {
            self.dimensions = calculate_dimensions(&self.dimensions, &self.land_records);
        }

        // calculate heights
        if recalculate_heights {
            if let Some(heights) = calculate_heights(&self.land_records, &mut self.dimensions) {
                self.heights = heights;
            }
        }

        let image = match self.ui_data.background {
            EBackground::None => None,
            EBackground::Landscape => {
                let max_texture_side = ctx.input(|i| i.max_texture_side);
                self.populate_texture_map(max_texture_side);

                Some(self.get_landscape_image())
            }
            EBackground::HeightMap => Some(self.get_heightmap_image()),
            EBackground::GameMap => Some(self.get_gamemap_image()),
        };

        if let Some(image) = image {
            self.background_handle =
                Some(ctx.load_texture("background", image, Default::default()));
        } else {
            self.background_handle = None;
        }
    }

    // Shortcuts

    pub fn get_heightmap_image(&mut self) -> ColorImage {
        generate_heightmap(
            &self.heights,
            &self.dimensions,
            &self.ui_data.heightmap_settings,
        )
    }

    pub fn get_gamemap_image(&mut self) -> ColorImage {
        generate_map(&self.dimensions, &self.land_records)
    }

    pub fn get_landscape_image(&mut self) -> ColorImage {
        compute_landscape_image(
            &self.ui_data.landscape_settings,
            &self.dimensions,
            &self.land_records,
            &self.ltex_records,
            &self.heights,
            &self.texture_map,
        )
    }

    // UI methods
    pub fn reset_zoom(&mut self) {
        self.zoom_data.zoom = 1.0;
    }

    pub fn reset_pan(&mut self) {
        self.zoom_data.drag_delta = None;
        self.zoom_data.drag_offset = Pos2::default();
        self.zoom_data.drag_start = Pos2::default();
    }

    pub(crate) fn texture_size(&self) -> f32 {
        self.ui_data.landscape_settings.texture_size as f32
    }

    pub fn save_image(&mut self, ctx: &egui::Context) -> Result<(), ImageError> {
        // construct default name from the first plugin name then the background type abbreviated
        let background_name = match self.ui_data.background {
            EBackground::None => "",
            EBackground::Landscape => "l",
            EBackground::HeightMap => "h",
            EBackground::GameMap => "g",
        };
        let first_plugin = self
            .plugins
            .as_ref()
            .unwrap()
            .iter()
            .filter(|p| p.enabled)
            .nth(0)
            .unwrap();
        let plugin_name = first_plugin.get_name();
        let defaultname = format!("{}_{}.png", plugin_name, background_name);

        let file_option = rfd::FileDialog::new()
            .add_filter("png", &["png"])
            .set_file_name(defaultname)
            .save_file();

        if let Some(original_path) = file_option {
            let mut background = None;
            match self.ui_data.background {
                EBackground::None => {}
                EBackground::Landscape => {
                    let max_texture_side = ctx.input(|i| i.max_texture_side);
                    self.populate_texture_map(max_texture_side);
                    background = Some(self.get_landscape_image());
                }
                EBackground::HeightMap => {
                    background = Some(self.get_heightmap_image());
                }
                EBackground::GameMap => {
                    background = Some(self.get_gamemap_image());
                }
            }

            if let Some(bg) = background {
                let mut bg_image = color_image_to_dynamic_image(&bg)?;

                // overlay paths
                if self.ui_data.overlay_paths {
                    let fg = paths::get_overlay_path_image(&self.dimensions, &self.land_records);
                    let mut fg_image = color_image_to_dynamic_image(&fg)?;

                    #[allow(clippy::comparison_chain)]
                    if bg.size < fg.size {
                        // resize the smaller image to the larger image
                        bg_image = image::imageops::resize(
                            &bg_image,
                            fg.size[0] as u32,
                            fg.size[1] as u32,
                            image::imageops::FilterType::CatmullRom,
                        )
                        .into();
                    } else if bg.size > fg.size {
                        // resize the fg image to the bg image
                        fg_image = image::imageops::resize(
                            &fg_image,
                            bg.size[0] as u32,
                            bg.size[1] as u32,
                            image::imageops::FilterType::CatmullRom,
                        )
                        .into();
                    }

                    // overlay the images
                    imageops::overlay(&mut bg_image, &fg_image, 0, 0);
                }

                // other overlays
                let any_overlay = self.ui_data.overlay_region
                    || self.ui_data.overlay_grid
                    || self.ui_data.overlay_cities
                    || self.ui_data.overlay_travel
                    || self.ui_data.overlay_conflicts;

                if any_overlay {
                    let real_width = self.dimensions.width() as f32;
                    let real_height = self.dimensions.height() as f32;
                    let from: Rect =
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(real_width, real_height));

                    let canvas_width = bg_image.width() as f32;
                    let canvas_height = bg_image.height() as f32;
                    //let r = real_height / real_width;
                    //let canvas_height = r * canvas_width as f32;
                    let transform = RectTransform::from_to(
                        from,
                        Rect::from_min_max(pos2(0.0, 0.0), pos2(canvas_width, canvas_height)),
                    );

                    let mut all_shapes = vec![];

                    // order is: paths, regions, grid, cities, travel, conflicts
                    // regions
                    if self.ui_data.overlay_region {
                        let shapes = get_region_shapes(
                            transform,
                            &self.dimensions,
                            &self.regn_records,
                            &self.cell_records,
                        );
                        all_shapes.extend(shapes);
                    }
                    // grid
                    if self.ui_data.overlay_grid {
                        let shapes = overlay::grid::get_grid_shapes(transform, &self.dimensions);
                        all_shapes.extend(shapes);
                    }
                    // cities
                    if self.ui_data.overlay_cities {
                        let shapes = overlay::cities::get_cities_shapes(
                            transform,
                            &self.dimensions,
                            &self.cell_records,
                        );
                        all_shapes.extend(shapes);
                    }
                    // travel
                    if self.ui_data.overlay_travel {
                        let shapes = overlay::travel::get_travel_shapes(
                            transform,
                            &self.dimensions,
                            &self.travel_edges,
                        );
                        all_shapes.extend(shapes);
                    }
                    // conflicts
                    if self.ui_data.overlay_conflicts {
                        let shapes = overlay::conflicts::get_conflict_shapes(
                            transform,
                            &self.dimensions,
                            &self.cell_conflicts,
                        );
                        all_shapes.extend(shapes);
                    }

                    // draw the shapes
                    for shape in all_shapes {
                        match shape {
                            Shape::Rect(shape) => {
                                // stroke
                                if shape.stroke != Default::default() {
                                    let color = shape.stroke.color;
                                    let stroke_width = shape.stroke.width;
                                    let rect = shape.rect;

                                    // image buffer here is just the border of the rectangle with the stroke color and width
                                    let img = image::ImageBuffer::from_fn(
                                        rect.width() as u32,
                                        rect.height() as u32,
                                        |x, y| {
                                            if x < stroke_width as u32
                                                || x >= rect.width() as u32 - stroke_width as u32
                                                || y < stroke_width as u32
                                                || y >= rect.height() as u32 - stroke_width as u32
                                            {
                                                image::Rgba([
                                                    color.r(),
                                                    color.g(),
                                                    color.b(),
                                                    color.a(),
                                                ])
                                            } else {
                                                image::Rgba([0, 0, 0, 0])
                                            }
                                        },
                                    );

                                    imageops::overlay(
                                        &mut bg_image,
                                        &img,
                                        rect.min.x as i64,
                                        rect.min.y as i64,
                                    );
                                } else if shape.fill != Default::default() {
                                    // filled
                                    let color = shape.fill;
                                    let rect = shape.rect;

                                    let img = image::ImageBuffer::from_pixel(
                                        rect.width() as u32,
                                        rect.height() as u32,
                                        image::Rgba([color.r(), color.g(), color.b(), color.a()]),
                                    );
                                    imageops::overlay(
                                        &mut bg_image,
                                        &img,
                                        rect.min.x as i64,
                                        rect.min.y as i64,
                                    );
                                }
                            }
                            // TODO fix lines
                            Shape::LineSegment { points, stroke } => {
                                let color = stroke.color;
                                let stroke_width = stroke.width;

                                for i in 0..points.len() - 1 {
                                    let p1 = points[i];
                                    let p2 = points[i + 1];

                                    let img = image::ImageBuffer::from_fn(
                                        (p1.distance(p2) + 1.0) as u32,
                                        stroke_width as u32,
                                        |x, _y| {
                                            if x < stroke_width as u32 {
                                                image::Rgba([
                                                    color.r(),
                                                    color.g(),
                                                    color.b(),
                                                    color.a(),
                                                ])
                                            } else {
                                                image::Rgba([0, 0, 0, 0])
                                            }
                                        },
                                    );

                                    let min = Pos2::new(p1.x.min(p2.x), p1.y.min(p2.y));
                                    imageops::overlay(
                                        &mut bg_image,
                                        &img,
                                        min.x as i64,
                                        min.y as i64,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }

                bg_image.save(original_path)?;

                rfd::MessageDialog::new()
                    .set_title("Info")
                    .set_description("Image saved successfully")
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
            }
        }

        Ok(())
    }
}
