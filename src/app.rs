use std::{collections::HashMap, path::PathBuf};

use background::landscape::compute_landscape_image;
use egui::{reset_button, Color32, ColorImage, Pos2};

use log::{debug, error, info, warn};
use seahash::hash;
use tes3::esp::{Landscape, LandscapeTexture, Plugin};

use crate::*;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default)]
pub struct TemplateApp {
    pub cwd: Option<PathBuf>,

    // ui
    pub ui_data: SavedUiData,
    pub zoom_data: ZoomData,
    pub dimensions: Dimensions,
    pub dimensions_z: DimensionsZ,
    pub heights: Vec<f32>,

    // textures in memory
    pub bg: Option<egui::TextureHandle>,

    // tes3
    data_files: Option<PathBuf>,
    pub landscape_records: HashMap<CellKey, (u64, Landscape)>,
    texture_map: HashMap<(u64, u32), ColorImage>,

    // app
    pub info: String,
    pub current_landscape: Option<Landscape>,
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        Default::default()
    }

    pub fn load_folder(&mut self, path: &PathBuf, _ctx: &egui::Context) {
        self.landscape_records.clear();
        self.texture_map.clear();
        self.data_files = Some(path.clone());

        info!("== loading folder {}", path.display());

        for path in get_plugins_sorted(&path, false) {
            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                let hash = hash(path.to_str().unwrap_or_default().as_bytes());
                info!("\t== loading plugin {} with hash {}", path.display(), hash);

                for landscape in plugin.objects_of_type::<Landscape>() {
                    let key = landscape.grid;
                    self.landscape_records
                        .insert(key, (hash, landscape.clone()));
                }

                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    if let Some(image) = load_texture(&self.data_files, &r) {
                        self.texture_map.insert((hash, key), image);
                        info!(
                            "\t\tinserting texture [{:?}] {} ({})",
                            (hash, key),
                            r.file_name,
                            r.id
                        );
                    } else {
                        warn!("\t\tmissing texture [{:?}] {}", (hash, key), r.file_name)
                    }
                }
            }
        }
    }

    pub fn open_folder(&mut self, ctx: &egui::Context) {
        let folder_option = rfd::FileDialog::new()
            .add_filter("esm", &["esm"])
            .add_filter("esp", &["esp"])
            .pick_folder();

        if let Some(path) = folder_option {
            self.data_files = Some(path.clone());
            self.load_folder(&path, ctx);
        }
    }

    pub fn open_plugin(&mut self, _ctx: &egui::Context) {
        let file_option = rfd::FileDialog::new()
            .add_filter("esm", &["esm"])
            .add_filter("esp", &["esp"])
            .pick_file();

        if let Some(path) = file_option {
            if let Some(dir_path) = path.parent() {
                self.data_files = Some(PathBuf::from(dir_path));
            }

            let hash = hash(path.to_str().unwrap_or_default().as_bytes());
            info!("\t== loading plugin {} with hash {}", path.display(), hash);

            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                // get data
                self.landscape_records.clear();
                self.texture_map.clear();

                for r in plugin.objects_of_type::<Landscape>() {
                    let key = r.grid;
                    self.landscape_records.insert(key, (hash, r.clone()));
                }

                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    if let Some(image) = load_texture(&self.data_files, &r) {
                        self.texture_map.insert((hash, key), image);
                        info!("\tinserting texture [{:?}] {}", (hash, key), r.file_name);
                    } else {
                        warn!("\tmissing texture [{:?}] {}", (hash, key), r.file_name)
                    }
                }
            }
        }
    }

    /// Assigns landscape_records, dimensions and pixels
    pub fn reload_background(&mut self, ctx: &egui::Context, new_dimensions: Option<Dimensions>) {
        self.bg = None;

        // calculate dimensions
        if let Some(dimensions) = new_dimensions {
            self.dimensions = dimensions.clone();
        } else {
            let Some(dimensions) =
                calculate_dimensions(&self.landscape_records, self.ui_data.texture_size)
            else {
                return;
            };

            self.dimensions = dimensions.clone();
        }

        // calculate heights
        if let Some((heights, dimensions_z)) =
            background::heightmap::calculate_heights(&self.landscape_records, &self.dimensions)
        {
            self.dimensions_z = dimensions_z;
            self.heights = heights;
            let max_texture_side = ctx.input(|i| i.max_texture_side);

            match self.ui_data.background {
                EBackground::None => {
                    // do nothing
                }
                EBackground::Landscape => {
                    let landscape_img = self.get_landscape_image(max_texture_side);
                    let _: &egui::TextureHandle = self.bg.get_or_insert_with(|| {
                        ctx.load_texture("background", landscape_img, Default::default())
                    });
                }
                EBackground::HeightMap => {
                    let heightmap_img = self.get_heightmap_image();
                    let _: &egui::TextureHandle = self.bg.get_or_insert_with(|| {
                        ctx.load_texture("background", heightmap_img, Default::default())
                    });
                }
            }
        }
    }

    // Shortcuts

    pub fn get_heightmap_image(&mut self) -> ColorImage {
        create_image(
            &self.heights,
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            self.dimensions_z,
            self.ui_data,
        )
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
            &self.landscape_records,
            &self.texture_map,
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

        img2.pixels.clone_from(&overlay::paths::color_pixels_reload(
            &self.dimensions,
            &self.landscape_records,
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

    /// Settings popup menu
    pub(crate) fn settings_ui(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.horizontal(|ui| {
            reset_button(ui, &mut self.ui_data);

            if ui.button("Refresh image").clicked() {
                self.dimensions.texture_size = self.ui_data.texture_size;

                // TOOD reload
            }
        });

        ui.separator();

        ui.label("Background");
        egui::ComboBox::from_label("Background")
            .selected_text(format!("{:?}", self.ui_data.background))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_data.background, EBackground::None, "None");
                ui.selectable_value(
                    &mut self.ui_data.background,
                    EBackground::HeightMap,
                    "HeightMap",
                );
                ui.selectable_value(
                    &mut self.ui_data.background,
                    EBackground::Landscape,
                    "Landscape",
                );
            });

        ui.separator();

        ui.label("Overlays");
        ui.checkbox(&mut self.ui_data.overlay_paths, "Show overlay map");

        ui.separator();
        ui.checkbox(&mut self.ui_data.show_tooltips, "Show tooltips");

        ui.label("Color");
        ui.add(egui::Slider::new(&mut self.ui_data.alpha, 0..=255).text("Alpha"));

        ui.color_edit_button_srgba(&mut self.ui_data.height_base);
        ui.add(
            egui::Slider::new(&mut self.ui_data.height_spectrum, -360..=360).text("Height offset"),
        );

        ui.color_edit_button_srgba(&mut self.ui_data.depth_base);
        ui.add(
            egui::Slider::new(&mut self.ui_data.depth_spectrum, -360..=360).text("Depth offset"),
        );

        ui.separator();

        ui.add(
            egui::Slider::new(&mut self.ui_data.texture_size, 2..=200).text("Texture Resolution"),
        );

        ui.separator();

        ui.label("zoom with Ctrl + Mousewheel");
        ui.label("reset with middle mouse button");
    }
}
