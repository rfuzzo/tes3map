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
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TemplateApp {
    pub data_files: Option<PathBuf>,
    pub ui_data: SavedUiData,

    // ui
    #[serde(skip)]
    pub zoom_data: ZoomData,
    #[serde(skip)]
    pub dimensions: Dimensions,
    #[serde(skip)]
    pub dimensions_z: DimensionsZ,
    #[serde(skip)]
    pub heights: Vec<f32>,

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
    pub bg: Option<egui::TextureHandle>,

    // app
    #[serde(skip)]
    pub hover_pos: CellKey,
    #[serde(skip)]
    pub side_panel_view: ESidePanelView,
    #[serde(skip)]
    pub info: TooltipInfo,
    #[serde(skip)]
    pub current_landscape: Option<Landscape>,
    #[serde(skip)]
    pub selected_id: Option<CellKey>,
    #[serde(skip)]
    pub cell_filter: String,
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

    /// Assigns landscape_records, dimensions and pixels
    pub fn reload_background(&mut self, ctx: &egui::Context, new_dimensions: Option<Dimensions>) {
        self.bg = None;

        // calculate dimensions
        if let Some(dimensions) = new_dimensions {
            self.dimensions = dimensions.clone();
        } else {
            let Some(dimensions) =
                calculate_dimensions(&self.land_records, self.ui_data.texture_size)
            else {
                return;
            };

            self.dimensions = dimensions.clone();
        }

        // calculate heights
        if let Some((heights, dimensions_z)) =
            background::heightmap::calculate_heights(&self.land_records, &self.dimensions)
        {
            self.dimensions_z = dimensions_z;
            self.heights = heights;

            match self.ui_data.background {
                EBackground::None => {
                    // do nothing
                }
                EBackground::Landscape => {
                    let max_texture_side = ctx.input(|i| i.max_texture_side);
                    let image = self.get_landscape_image(max_texture_side);
                    let _: &egui::TextureHandle = self.bg.get_or_insert_with(|| {
                        ctx.load_texture("background", image, Default::default())
                    });
                }
                EBackground::HeightMap => {
                    let image = self.get_heightmap_image();
                    let _: &egui::TextureHandle = self.bg.get_or_insert_with(|| {
                        ctx.load_texture("background", image, Default::default())
                    });
                }
                EBackground::GameMap => {
                    let image = self.get_gamemap_image();
                    let _: &egui::TextureHandle = self.bg.get_or_insert_with(|| {
                        ctx.load_texture("background", image, Default::default())
                    });
                }
            }
        }
    }

    // Shortcuts

    pub fn get_heightmap_image(&mut self) -> ColorImage {
        generate_heightmap(
            &self.heights,
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            self.dimensions_z,
            self.ui_data,
        )
    }
    pub fn get_gamemap_image(&mut self) -> ColorImage {
        generate_map(&self.dimensions, &self.land_records)
    }

    pub fn get_landscape_image(&mut self, max_texture_side: usize) -> ColorImage {
        // glow supports textures up to this
        let dimensions = &mut self.dimensions;
        let size_tuple = dimensions.pixel_size_tuple(dimensions.cell_size());
        let width = size_tuple[0];
        let height = size_tuple[1];
        if width > max_texture_side || height > max_texture_side {
            error!(
                "Texture size too large: (width: {}, height: {}), supported side: {}",
                width, height, max_texture_side
            );

            debug!("cell size {}", dimensions.cell_size());
            debug!("texture_size {}", dimensions.texture_size);
            debug!("Resetting texture size to 16");

            dimensions.texture_size = 16;

            // rfd messagebox
            rfd::MessageDialog::new()
                .set_title("Error")
                .set_description("Texture size too large, resetting to 16")
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
        }

        if let Some(i) = compute_landscape_image(
            dimensions,
            &self.land_records,
            &self.ltex_records,
            &self.data_files,
            &self.heights,
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
            self.ui_data.alpha,
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
