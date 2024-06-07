use std::{collections::HashMap, path::PathBuf};

use egui::{Color32, ColorImage, Pos2};
use log::{debug, error};
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
    pub texture_map: HashMap<String, ColorImage>,

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
        let image = self.get_overlay_path_image();
        self.paths_handle = Some(ctx.load_texture("paths", image, Default::default()));
    }

    pub fn populate_texture_map(&mut self, max_texture_side: usize) {
        // if the resolution is the same, no need to reload
        let max_texture_resolution = self.dimensions.get_max_texture_resolution(max_texture_side);
        if max_texture_resolution > self.texture_map_resolution
            && self.texture_map_resolution == self.dimensions.texture_size
            && self.dimensions.texture_size == self.ui_data.landscape_settings.texture_size
        {
            debug!("Texture resolution is the same, no need to reload");
            return;
        }

        // otherwise check if possible
        let width = self.dimensions.pixel_width();
        let height = self.dimensions.pixel_height();
        if width > max_texture_side || height > max_texture_side {
            error!(
                "Texture size too large: (width: {}, height: {}), supported side: {}, max_texture_side: {}",
                width, height, max_texture_side, max_texture_resolution
            );

            debug!("cell size {}", self.dimensions.cell_size());
            debug!("texture_size {}", self.dimensions.texture_size);
            debug!("Resetting texture size to 16");

            self.dimensions.texture_size = 16;
            self.ui_data.landscape_settings.texture_size = 16;

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
        let texture_size = self.dimensions.texture_size;
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

                                        if let Some(tex) = load_texture(&self.data_files, ltex) {
                                            // transform texture and downsize

                                            // scale texture to fit the texture_size
                                            let mut pixels = vec![
                                                Color32::TRANSPARENT;
                                                texture_size * texture_size
                                            ];

                                            // textures per tile
                                            for x in 0..texture_size {
                                                for y in 0..texture_size {
                                                    // pick every nth pixel from the texture to downsize
                                                    let sx = x * (TEXTURE_MAX_SIZE / texture_size);
                                                    let sy = y * (TEXTURE_MAX_SIZE / texture_size);
                                                    let index = (sy * texture_size) + sx;
                                                    let color = tex.pixels[index];

                                                    let i = (y * texture_size) + x;
                                                    pixels[i] = color;
                                                }
                                            }

                                            let downsized_texture = ColorImage {
                                                size: [texture_size, texture_size],
                                                pixels,
                                            };

                                            info!("Loaded texture: {}", ltex.file_name);
                                            self.texture_map
                                                .insert(texture_name, downsized_texture);
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
            self.ui_data.landscape_settings.texture_size = dimensions.texture_size;
        } else if recalculate_dimensions {
            self.dimensions = calculate_dimensions(
                &self.dimensions,
                &self.land_records,
                self.ui_data.landscape_settings.texture_size,
            );
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
        // glow supports textures up to this
        let dimensions = &mut self.dimensions;

        if let Some(i) = compute_landscape_image(
            dimensions,
            &self.land_records,
            &self.ltex_records,
            &self.heights,
            &self.texture_map,
        ) {
            i
        } else {
            // default image
            ColorImage::new(
                [
                    dimensions.width() * dimensions.cell_size(),
                    dimensions.height() * dimensions.cell_size(),
                ],
                Color32::BLACK,
            )
        }
    }

    pub fn get_overlay_path_image(&mut self) -> ColorImage {
        let mut img2 =
            ColorImage::new(self.dimensions.pixel_size_tuple(VERTEX_CNT), Color32::WHITE);

        img2.pixels.clone_from(&overlay::paths::get_color_pixels(
            &self.dimensions,
            &self.land_records,
        ));
        img2
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
}
