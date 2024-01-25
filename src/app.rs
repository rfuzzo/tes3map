use std::{collections::HashMap, path::PathBuf};

use egui::{reset_button, Color32, ColorImage, Pos2};

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
    landscape_records: HashMap<(i32, i32), Landscape>,
    #[serde(skip)]
    texture_map: HashMap<u32, ColorImage>,

    // painting
    #[serde(skip)]
    pub dimensions: Dimensions,
    #[serde(skip)]
    pub heights: Vec<f32>,
    #[serde(skip)]
    pixel_color: Vec<Color32>,

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
    fn load_data(&mut self) -> Option<(ColorImage, ColorImage, ColorImage)> {
        self.background = None;
        self.foreground = None;
        self.textured = None;

        // get dimensions
        let mut min_x: Option<i32> = None;
        let mut min_y: Option<i32> = None;
        let mut max_x: Option<i32> = None;
        let mut max_y: Option<i32> = None;

        for key in self.landscape_records.keys() {
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

        // calculate heights
        let mut min_z: Option<f32> = None;
        let mut max_z: Option<f32> = None;

        let mut heights_map: HashMap<(i32, i32), [[f32; 65]; 65]> = HashMap::default();
        //let mut texture_map: HashMap<(i32, i32), Vec<String>> = HashMap::default();

        for cy in min_y..max_y + 1 {
            for cx in min_x..max_x + 1 {
                if let Some(landscape) = self.landscape_records.get(&(cx, cy)) {
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

        let dimensions = Dimensions {
            min_x,
            min_y,
            max_x,
            max_y,
            min_z,
            max_z,
        };

        let pixels = height_map_to_pixel_heights(dimensions, heights_map);

        let img = create_image(
            &pixels,
            dimensions.pixel_size_tuple(VERTEX_CNT),
            dimensions,
            self.ui_data,
        );

        // assign data
        self.dimensions = dimensions;
        self.heights = pixels;

        self.color_pixels_reload();
        let mut img2 = ColorImage::new(dimensions.pixel_size_tuple(VERTEX_CNT), Color32::WHITE);
        img2.pixels = self.pixel_color.clone();

        let img3 = if let Some(i) = self.get_textured_pixels() {
            i
        } else {
            ColorImage::new(
                [
                    self.dimensions.width() * CELL_SIZE,
                    self.dimensions.height() * CELL_SIZE,
                ],
                Color32::GOLD,
            )
        };

        Some((img, img2, img3))
    }

    fn color_pixels_reload(&mut self) {
        let mut color_map: HashMap<(i32, i32), [[Color32; 65]; 65]> = HashMap::default();
        let d = self.dimensions;

        for cy in d.min_y..d.max_y + 1 {
            for cx in d.min_x..d.max_x + 1 {
                if let Some(landscape) = self.landscape_records.get(&(cx, cy)) {
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

        self.pixel_color = color_map_to_pixels(d, color_map);
    }

    pub fn load_folder(&mut self, path: &PathBuf, ctx: &egui::Context) {
        self.landscape_records.clear();
        self.texture_map.clear();

        for path in get_plugins_sorted(&path, false) {
            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                for r in plugin.objects_of_type::<Landscape>() {
                    let key = r.grid;
                    self.landscape_records.insert(key, r.clone());
                }
                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    if let Some(image) = self.load_texture(&r) {
                        self.texture_map.insert(key, image);
                    }
                }
            }
        }

        if let Some((back, fore, tex)) = self.load_data() {
            let _: &egui::TextureHandle = self
                .background
                .get_or_insert_with(|| ctx.load_texture("background", back, Default::default()));

            let _: &egui::TextureHandle = self
                .foreground
                .get_or_insert_with(|| ctx.load_texture("foreground", fore, Default::default()));

            let _: &egui::TextureHandle = self
                .textured
                .get_or_insert_with(|| ctx.load_texture("textured", tex, Default::default()));
        }
    }

    pub fn get_background(&mut self) -> ColorImage {
        create_image(
            &self.heights,
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            self.dimensions,
            self.ui_data,
        )
    }

    pub fn get_foreground(&mut self) -> ColorImage {
        let mut img2 =
            ColorImage::new(self.dimensions.pixel_size_tuple(VERTEX_CNT), Color32::WHITE);
        self.color_pixels_reload();
        img2.pixels = self.pixel_color.clone();
        img2
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

            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                // get data
                self.landscape_records.clear();
                self.texture_map.clear();

                for r in plugin.objects_of_type::<Landscape>() {
                    let key = r.grid;
                    self.landscape_records.insert(key, r.clone());
                }
                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    if let Some(image) = self.load_texture(&r) {
                        self.texture_map.insert(key, image);
                    }
                }

                // get pictures
                if let Some((back, fore, tex)) = self.load_data() {
                    let _: &egui::TextureHandle = self.background.get_or_insert_with(|| {
                        ctx.load_texture("background", back, Default::default())
                    });

                    let _: &egui::TextureHandle = self.foreground.get_or_insert_with(|| {
                        ctx.load_texture("foreground", fore, Default::default())
                    });

                    let _: &egui::TextureHandle = self.textured.get_or_insert_with(|| {
                        ctx.load_texture("textured", tex, Default::default())
                    });
                }
            }
        }
    }

    pub fn get_layered_image(&mut self, img: ColorImage, img2: ColorImage) -> ColorImage {
        let mut layered = img.pixels.clone();
        for (i, color1) in img.pixels.iter().enumerate() {
            let color2 = img2.pixels[i];
            let rgb1 = (color1.r(), color1.g(), color1.b());
            let rgb2 = (color2.r(), color2.g(), color2.b());
            let a1 = color1.a() as f32 / 255.0;
            let a2 = color2.a() as f32 / 255.0;
            let f = overlay_colors(rgb1, a1, rgb2, a2);
            layered[i] = Color32::from_rgba_premultiplied(f.0, f.1, f.2, f.3);
        }
        let mut layered_img = ColorImage::new(
            self.dimensions.pixel_size_tuple(VERTEX_CNT),
            Color32::TRANSPARENT,
        );
        layered_img.pixels = layered;
        layered_img
    }

    fn get_textured_pixels(&self) -> Option<ColorImage> {
        let d = self.dimensions;
        let size = d.pixel_size(CELL_SIZE);
        let mut pixels_color = vec![Color32::TRANSPARENT; size];

        for cy in d.min_y..d.max_y + 1 {
            for cx in d.min_x..d.max_x + 1 {
                if let Some(landscape) = self.landscape_records.get(&(cx, cy)) {
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
                                    let Some(color_image) = self.texture_map.get(&key) else {
                                        continue;
                                    };

                                    // textures per tile
                                    for x in 0..TEXTURE_SIZE {
                                        for y in 0..TEXTURE_SIZE {
                                            let tx = d.tranform_to_canvas_x(cx) * CELL_SIZE
                                                + gx * TEXTURE_SIZE
                                                + x;
                                            let ty = d.tranform_to_canvas_y(cy) * CELL_SIZE
                                                + (GRID_SIZE - gy) * TEXTURE_SIZE
                                                + y;

                                            let stride = d.width() * CELL_SIZE;
                                            let i = (ty * stride) + tx;

                                            let index = (y * TEXTURE_SIZE) + x;
                                            let color = color_image.pixels[index];
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
                            for x in 0..TEXTURE_SIZE {
                                for y in 0..TEXTURE_SIZE {
                                    let tx = d.tranform_to_canvas_x(cx) * CELL_SIZE
                                        + gx * TEXTURE_SIZE
                                        + x;
                                    let ty = d.tranform_to_canvas_y(cy) * CELL_SIZE
                                        + gy * TEXTURE_SIZE
                                        + y;

                                    let stride = d.width() * CELL_SIZE;
                                    let i = (ty * stride) + tx;

                                    pixels_color[i] = Color32::BLACK;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut img = ColorImage::new(d.pixel_size_tuple(CELL_SIZE), Color32::GOLD);
        img.pixels = pixels_color;
        Some(img)
    }

    fn load_texture(&self, r: &LandscapeTexture) -> Option<ColorImage> {
        let Some(data_files) = self.data_files.clone() else {
            return None;
        };

        let tex_path = data_files.join("Textures").join(r.file_name.clone());
        if !tex_path.exists() {
            return None;
        }

        // decode image
        if let Ok(mut reader) = image::io::Reader::open(&tex_path) {
            let ext = tex_path.extension().unwrap().to_string_lossy();
            if ext.contains("tga") {
                reader.set_format(image::ImageFormat::Tga);
                return None;
            } else if ext.contains("dds") {
                reader.set_format(image::ImageFormat::Dds);
            } else {
                // TODO do nothing
                return None;
            }

            let Ok(image) = reader.decode() else {
                return None;
            };

            let size = [image.width() as _, image.height() as _];
            // if size != [TEXTURE_SIZE as usize, TEXTURE_SIZE as usize] {
            //     return None;
            // }

            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            return Some(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()));
        }

        None
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
