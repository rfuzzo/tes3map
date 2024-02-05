use std::{collections::HashMap, path::PathBuf};

use egui::{reset_button, Color32, ColorImage, Pos2};

use log::{info, warn};
use seahash::hash;
use tes3::esp::{Landscape, LandscapeFlags, LandscapeTexture, Plugin};

use crate::*;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    pub cwd: Option<PathBuf>,

    // tes3
    #[serde(skip)]
    pub landscape_records: HashMap<CellKey, (u64, Landscape)>,
    #[serde(skip)]
    texture_map: HashMap<(u64, u32), ColorImage>,

    // painting
    #[serde(skip)]
    pub dimensions: Dimensions,
    #[serde(skip)]
    pub dimensions_z: DimensionsZ,
    #[serde(skip)]
    pub heights: Vec<f32>,
    #[serde(skip)]
    foreground_pixels: Vec<Color32>,

    #[serde(skip)]
    pub background: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub foreground: Option<egui::TextureHandle>,
    #[serde(skip)]
    pub textured: Option<egui::TextureHandle>,

    #[serde(skip)]
    data_files: Option<PathBuf>,
    #[serde(skip)]
    pub info: String,
    #[serde(skip)]
    pub current_landscape: Option<Landscape>,

    // ui
    pub ui_data: SavedUiData,

    #[serde(skip)]
    pub zoom_data: ZoomData,
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
    pub fn load_data(&mut self, ctx: &egui::Context) {
        self.background = None;
        self.foreground = None;
        self.textured = None;

        // calculate dimensions
        if let Some(dimensions) = calculate_dimensions(&self.landscape_records) {
            self.dimensions = dimensions;

            // calculate heights
            if let Some((heights, dimensions_z)) =
                calculate_heights(&self.landscape_records, dimensions)
            {
                self.dimensions_z = dimensions_z;
                self.heights = heights;

                let background = self.get_background();
                let foreground = self.get_foreground();
                let textured = self.get_textured();

                let _: &egui::TextureHandle = self.background.get_or_insert_with(|| {
                    ctx.load_texture("background", background, Default::default())
                });

                let _: &egui::TextureHandle = self.foreground.get_or_insert_with(|| {
                    ctx.load_texture("foreground", foreground, Default::default())
                });

                let _: &egui::TextureHandle = self.textured.get_or_insert_with(|| {
                    ctx.load_texture("textured", textured, Default::default())
                });
            }
        }
    }

    pub fn load_data_with_dimension(&mut self, dimensions: Dimensions, ctx: &egui::Context) {
        self.background = None;
        self.foreground = None;
        self.textured = None;

        self.dimensions = dimensions;

        // calculate heights
        if let Some((heights, dimensions_z)) =
            calculate_heights(&self.landscape_records, dimensions)
        {
            self.dimensions_z = dimensions_z;
            self.heights = heights;

            let background = self.get_background();
            let foreground = self.get_foreground();
            let textured = self.get_textured();

            let _: &egui::TextureHandle = self.background.get_or_insert_with(|| {
                ctx.load_texture("background", background, Default::default())
            });

            let _: &egui::TextureHandle = self.foreground.get_or_insert_with(|| {
                ctx.load_texture("foreground", foreground, Default::default())
            });

            let _: &egui::TextureHandle = self
                .textured
                .get_or_insert_with(|| ctx.load_texture("textured", textured, Default::default()));
        }
    }

    fn color_pixels_reload(&mut self) {
        let mut color_map: HashMap<CellKey, [[Color32; 65]; 65]> = HashMap::default();
        let d = self.dimensions;

        for cy in d.min_y..d.max_y + 1 {
            for cx in d.min_x..d.max_x + 1 {
                if let Some((_hash, landscape)) = self.landscape_records.get(&(cx, cy)) {
                    // get color data
                    let mut colors: [[Color32; 65]; 65] =
                        [[Color32::TRANSPARENT; VERTEX_CNT]; VERTEX_CNT];

                    if landscape
                        .landscape_flags
                        .contains(LandscapeFlags::USES_VERTEX_COLORS)
                    {
                        let data = &landscape.vertex_colors.data.clone();

                        for y in 0..VERTEX_CNT {
                            for x in 0..VERTEX_CNT {
                                let r = data[y][x][0];
                                let g = data[y][x][1];
                                let b = data[y][x][2];

                                let ratio = (r as f32 + g as f32 + b as f32) / (3_f32 * 255_f32);
                                let temp = (1_f32 - ratio).clamp(0.0, 1.0);

                                let c = self.ui_data.alpha as f32 / 100_f32;
                                let alpha = if temp < c { temp / c } else { 1_f32 };

                                let rgb = Color32::from_rgba_unmultiplied(
                                    r,
                                    g,
                                    b,
                                    (255_f32 * alpha) as u8,
                                );
                                colors[y][x] = rgb;
                            }
                        }
                    }
                    color_map.insert((cx, cy), colors);
                }
            }
        }

        self.foreground_pixels = color_map_to_pixels(d, color_map);
    }

    pub fn load_folder(&mut self, path: &PathBuf, ctx: &egui::Context) {
        self.landscape_records.clear();
        self.texture_map.clear();

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
                    if let Some(image) = self.load_texture(&r) {
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

        self.load_data(ctx);
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

    pub fn open_plugin(&mut self, ctx: &egui::Context) {
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
                    if let Some(image) = self.load_texture(&r) {
                        self.texture_map.insert((hash, key), image);
                        info!("\tinserting texture [{:?}] {}", (hash, key), r.file_name);
                    } else {
                        warn!("\tmissing texture [{:?}] {}", (hash, key), r.file_name)
                    }
                }

                // get pictures
                self.load_data(ctx);
            }
        }
    }

    pub fn get_layered_image(&mut self, img: ColorImage, img2: ColorImage) -> ColorImage {
        // base image
        let mut layered = img.pixels.clone();

        // overlay second image
        for (i, color1) in img.pixels.into_iter().enumerate() {
            let color2 = img2.pixels[i];
            layered[i] = overlay_colors(color1, color2);
        }

        // create new colorImage
        let mut layered_img = ColorImage::new(
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            Color32::TRANSPARENT,
        );
        layered_img.pixels = layered;
        layered_img
    }

    fn get_textured_pixels(&self) -> Option<ColorImage> {
        let d = self.dimensions;
        let size = d.pixel_size(d.cell_size());
        let mut pixels_color = vec![Color32::TRANSPARENT; size];
        let texture_size = d.texture_size;

        for cy in d.min_y..d.max_y + 1 {
            for cx in d.min_x..d.max_x + 1 {
                if let Some((hash, landscape)) = self.landscape_records.get(&(cx, cy)) {
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
                                    let Some(color_image) = self.texture_map.get(&(*hash, key))
                                    else {
                                        continue;
                                    };

                                    // textures per tile
                                    for x in 0..texture_size {
                                        for y in 0..texture_size {
                                            let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                                + gx * texture_size
                                                + x;
                                            let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                                + (GRID_SIZE - 1 - gy) * texture_size
                                                + y;

                                            let i = (ty * d.stride(d.cell_size())) + tx;

                                            // pick every nth pixel from the texture to downsize
                                            let sx = x * (TEXTURE_MAX_SIZE / texture_size);
                                            let sy = y * (TEXTURE_MAX_SIZE / texture_size);
                                            let index = (sy * texture_size) + sx;

                                            let mut color = color_image.pixels[index];

                                            // blend color when under water
                                            let screenx = tx * VERTEX_CNT / d.cell_size();
                                            let screeny = ty * VERTEX_CNT / d.cell_size();

                                            if let Some(height) =
                                                self.height_from_screen_space(screenx, screeny)
                                            {
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
                            for x in 0..texture_size {
                                for y in 0..texture_size {
                                    let tx = d.tranform_to_canvas_x(cx) * d.cell_size()
                                        + gx * texture_size
                                        + x;
                                    let ty = d.tranform_to_canvas_y(cy) * d.cell_size()
                                        + gy * texture_size
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

    fn load_texture(&self, r: &LandscapeTexture) -> Option<ColorImage> {
        let Some(data_files) = self.data_files.clone() else {
            return None;
        };

        let texture = r.file_name.clone();
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

    // Shortcuts

    pub fn get_background(&mut self) -> ColorImage {
        create_image(
            &self.heights,
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            self.dimensions_z,
            self.ui_data,
        )
    }

    pub fn get_foreground(&mut self) -> ColorImage {
        let mut img2 =
            ColorImage::new(self.dimensions.pixel_size_tuple(VERTEX_CNT), Color32::WHITE);
        self.color_pixels_reload();
        img2.pixels = self.foreground_pixels.clone();
        img2
    }

    pub fn get_textured(&mut self) -> ColorImage {
        if let Some(i) = self.get_textured_pixels() {
            i
        } else {
            // default image
            ColorImage::new(
                [
                    self.dimensions.width() * self.dimensions.cell_size(),
                    self.dimensions.height() * self.dimensions.cell_size(),
                ],
                Color32::BLACK,
            )
        }
    }

    pub fn height_from_screen_space(&self, x: usize, y: usize) -> Option<f32> {
        let i = (y * self.dimensions.stride(VERTEX_CNT)) + x;
        self.heights.get(i).copied()
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
    pub(crate) fn options_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            reset_button(ui, &mut self.ui_data);

            if ui.button("Refresh image").clicked() {
                let img = self.get_background();
                let img2 = self.get_foreground();

                // set handles
                self.background =
                    Some(ui.ctx().load_texture("background", img, Default::default()));
                self.foreground = Some(ui.ctx().load_texture(
                    "foreground",
                    img2,
                    Default::default(),
                ));
            }
        });

        ui.separator();
        ui.label("Overlays");
        ui.checkbox(&mut self.ui_data.overlay_terrain, "Show terrain map");
        ui.checkbox(&mut self.ui_data.overlay_textures, "Show textures");

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
    }
}
