use std::{
    collections::HashMap,
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};

use egui::{pos2, reset_button, Color32, ColorImage, Pos2, Rect, Sense};
use image::{
    error::{ImageFormatHint, UnsupportedError, UnsupportedErrorKind},
    DynamicImage, ImageError, RgbaImage,
};
use tes3::esp::{Landscape, LandscapeFlags, LandscapeTexture, Plugin};

use crate::*;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    cwd: Option<PathBuf>,

    // tes3
    #[serde(skip)]
    landscape_records: HashMap<(i32, i32), Landscape>,
    #[serde(skip)]
    texture_records: HashMap<u32, LandscapeTexture>,

    // painting
    #[serde(skip)]
    dimensions: Dimensions,
    #[serde(skip)]
    heights: Vec<f32>,
    #[serde(skip)]
    pixel_color: Vec<Color32>,
    #[serde(skip)]
    background: Option<egui::TextureHandle>,
    #[serde(skip)]
    foreground: Option<egui::TextureHandle>,
    #[serde(skip)]
    textured: Option<egui::TextureHandle>,

    #[serde(skip)]
    data_files: Option<PathBuf>,
    #[serde(skip)]
    info: String,

    // ui
    ui_data: SavedUiData,

    #[serde(skip)]
    zoom_data: ZoomData,
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
                    // texture_indices
                    // if landscape
                    //     .landscape_flags
                    //     .contains(LandscapeFlags::USES_TEXTURES)
                    // {
                    //     let data = &landscape.texture_indices.data;
                    //     let mut indices: Vec<String> = vec![];
                    //     for y in 0..16 {
                    //         for x in 0..16 {
                    //             let key = data[y][x] as u32;
                    //             if let Some(tex) = self.texture_records.get(&key) {
                    //                 //println!("{x},{y}: {}", tex.file_name);
                    //                 indices.push(tex.file_name.to_owned());
                    //             } else {
                    //                 indices.push("None".to_owned());
                    //             }
                    //         }
                    //     }

                    //     texture_map.insert((cx, cy), indices);
                    // }

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

        let img = create_image(&pixels, dimensions, self.ui_data);

        // assign data
        self.dimensions = dimensions;
        self.heights = pixels;

        self.color_pixels_reload();
        let mut img2 = ColorImage::new(dimensions.size(), Color32::WHITE);
        img2.pixels = self.pixel_color.clone();

        let img3 = if let Ok(i) = self.get_textured_pixels() {
            i
        } else {
            ColorImage::new(
                [
                    self.dimensions.width_cells() as usize * TEXTURE_GRID as usize,
                    self.dimensions.height_cells() as usize * TEXTURE_GRID as usize,
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

    fn load_folder(&mut self, path: &PathBuf, ctx: &egui::Context) {
        let mut records: HashMap<(i32, i32), Landscape> = HashMap::default();
        let mut textures: HashMap<u32, LandscapeTexture> = HashMap::default();

        for path in get_plugins_sorted(&path, false) {
            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                for r in plugin.objects_of_type::<Landscape>() {
                    let key = r.grid;
                    records.insert(key, r.clone());
                }
                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    textures.insert(key, r);
                }
            }
        }

        self.landscape_records = records;
        self.texture_records = textures;

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

    fn reset_zoom(&mut self) {
        self.zoom_data.zoom = 1.0;
    }

    fn reset_pan(&mut self) {
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
        ui.checkbox(&mut self.ui_data.overlay_paths, "Show overlay map");
        ui.checkbox(&mut self.ui_data.overlay_textures, "Show textures");

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

    fn get_background(&mut self) -> ColorImage {
        create_image(&self.heights, self.dimensions, self.ui_data)
    }

    fn get_foreground(&mut self) -> ColorImage {
        let mut img2 = ColorImage::new(self.dimensions.size(), Color32::WHITE);
        self.color_pixels_reload();
        img2.pixels = self.pixel_color.clone();
        img2
    }

    fn open_folder(&mut self, ctx: &egui::Context) {
        let folder_option = rfd::FileDialog::new()
            .add_filter("esm", &["esm"])
            .add_filter("esp", &["esp"])
            .pick_folder();

        if let Some(path) = folder_option {
            self.data_files = Some(path.clone());
            self.load_folder(&path, ctx);
        }
    }

    fn open_plugin(&mut self, ctx: &egui::Context) {
        let file_option = rfd::FileDialog::new()
            .add_filter("esm", &["esm"])
            .add_filter("esp", &["esp"])
            .pick_file();

        if let Some(path) = file_option {
            self.data_files = Some(path.clone());

            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                let mut records: HashMap<(i32, i32), Landscape> = HashMap::default();
                let mut textures: HashMap<u32, LandscapeTexture> = HashMap::default();
                for r in plugin.objects_of_type::<Landscape>() {
                    let key = r.grid;
                    records.insert(key, r.clone());
                }
                for r in plugin.into_objects_of_type::<LandscapeTexture>() {
                    let key = r.index;
                    textures.insert(key, r);
                }

                self.landscape_records = records;
                self.texture_records = textures;

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

    fn get_layered_image(&mut self, img: ColorImage, img2: ColorImage) -> ColorImage {
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
        let mut layered_img = ColorImage::new(self.dimensions.size(), Color32::TRANSPARENT);
        layered_img.pixels = layered;
        layered_img
    }

    fn get_textured_pixels(&self) -> std::io::Result<ColorImage> {
        let data_files = if let Some(d) = &self.data_files {
            d
        } else {
            return Err(Error::from(ErrorKind::NotFound));
        };

        let d = self.dimensions;
        let stride = d.width_cells() * TEXTURE_GRID;
        let size = d.width_cells() * TEXTURE_GRID * d.height_cells() * TEXTURE_GRID;
        let mut pixels_color = vec![Color32::BLUE; size as usize];

        for cy in d.min_y..d.max_y + 1 {
            for cx in d.min_x..d.max_x + 1 {
                if let Some(landscape) = self.landscape_records.get(&(cx, cy)) {
                    if landscape
                        .landscape_flags
                        .contains(LandscapeFlags::USES_TEXTURES)
                    {
                        // each tile is subdivided into a 16x16 grid
                        let data = &landscape.texture_indices.data;
                        for gy in 0..GRID_SIZE {
                            for gx in 0..GRID_SIZE {
                                // for each sub-grid get the texture

                                let key = data[gy as usize][gx as usize] as u32;
                                let Some(tex) = self.texture_records.get(&key) else {
                                    continue;
                                };
                                let tex_path = PathBuf::from(data_files)
                                    .join("Textures")
                                    .join(&tex.file_name);
                                if !tex_path.exists() {
                                    continue;
                                }

                                // decode image
                                let mut reader = image::io::Reader::open(&tex_path)?;
                                let ext = tex_path.extension().unwrap().to_string_lossy();
                                if ext.contains("tga") {
                                    reader.set_format(image::ImageFormat::Tga);
                                    continue;
                                } else if ext.contains("dds") {
                                    reader.set_format(image::ImageFormat::Dds);
                                } else {
                                    // do nothing
                                    continue;
                                }

                                let Ok(image) = reader.decode() else { continue };
                                let image_buffer = image.to_rgba8();
                                let pixels = image_buffer.as_flat_samples();
                                let size = [image.width() as _, image.height() as _];

                                if size != [TEXTURE_SIZE as usize, TEXTURE_SIZE as usize] {
                                    continue;
                                }
                                let color_image =
                                    ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                                let pixels = color_image.pixels;

                                // assign pixel

                                for y in 0..TEXTURE_SIZE {
                                    for x in 0..TEXTURE_SIZE {
                                        let tx = d.tranform_to_canvas_x(cx) * TEXTURE_GRID
                                            + gx * GRID_SIZE
                                            + x;
                                        let ty = d.tranform_to_canvas_y(cy) * TEXTURE_GRID
                                            + gy * GRID_SIZE
                                            + y;

                                        let i = (ty * stride) + tx;

                                        let index = (y * TEXTURE_SIZE) + x;
                                        let color = pixels[index as usize];
                                        pixels_color[i as usize] = color;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // no landscape
                    for gy in 0..GRID_SIZE {
                        for gx in 0..GRID_SIZE {
                            for y in 0..TEXTURE_GRID {
                                for x in 0..TEXTURE_GRID {
                                    let tx = d.tranform_to_canvas_x(cx) * TEXTURE_GRID
                                        + gx * GRID_SIZE
                                        + x;
                                    let ty = d.tranform_to_canvas_y(cy) * TEXTURE_GRID
                                        + gy * GRID_SIZE
                                        + y;

                                    let i = (ty * stride) + tx;

                                    pixels_color[i as usize] = Color32::BLACK;
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut img = ColorImage::new(
            [
                d.width_cells() as usize * TEXTURE_GRID as usize,
                d.height_cells() as usize * TEXTURE_GRID as usize,
            ],
            Color32::GOLD,
        );
        img.pixels = pixels_color;
        Ok(img)
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.cwd.is_none() {
            if let Ok(cwd) = env::current_dir() {
                // load once
                self.cwd = Some(cwd.clone());
                self.load_folder(&cwd, ctx);
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load folder").clicked() {
                        self.open_folder(ctx);
                        ui.close_menu();
                    }

                    if ui.button("Load plugin").clicked() {
                        self.open_plugin(ctx);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        // egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
        //     ui.heading("Cells");
        //     ui.separator();
        // });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!(
                "Map (y: [{},{}]; x: [{},{}]; z: [{},{}])",
                self.dimensions.min_y,
                self.dimensions.max_y,
                self.dimensions.min_x,
                self.dimensions.max_x,
                self.dimensions.min_z,
                self.dimensions.max_z
            ));

            ui.separator();

            if self.heights.is_empty() {
                // Default UI
                ui.horizontal(|ui| {
                    if ui.button("Load plugin").clicked() {
                        self.open_plugin(ctx);
                    }

                    if ui.button("Load folder").clicked() {
                        self.open_folder(ctx);
                    }
                });

                // settings
                egui::Frame::popup(ui.style())
                    .stroke(egui::Stroke::NONE)
                    .show(ui, |ui| {
                        ui.set_max_width(170.0);
                        egui::CollapsingHeader::new("Settings").show(ui, |ui| self.options_ui(ui));
                    });

                return;
            }

            // painter
            // let clip_rect = ui.available_rect_before_wrap();
            // let painter = egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip_rect);
            // let response = painter.ctx();

            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

            // panning and zooming
            if let Some(delta) = self.zoom_data.drag_delta.take() {
                self.zoom_data.drag_offset += delta.to_vec2();
            }

            // move to center zoom
            if let Some(z) = self.zoom_data.zoom_delta.take() {
                let r = z - 1.0;
                self.zoom_data.zoom += r;

                // TODO offset the image for smooth zoom
                if let Some(pointer_pos) = response.hover_pos() {
                    let d = pointer_pos * r;
                    self.zoom_data.drag_offset -= d.to_vec2();
                }
            }

            // TODO cut off pan at (0,0)
            let min = self.zoom_data.drag_offset;
            let max =
                response.rect.max * self.zoom_data.zoom + self.zoom_data.drag_offset.to_vec2();
            let canvas = Rect::from_min_max(min, max);
            let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));

            // transforms
            let to = canvas;
            let from = egui::Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(
                    self.dimensions.width() as f32,
                    self.dimensions.height() as f32,
                ),
            );
            let to_screen = egui::emath::RectTransform::from_to(from, to);
            let from_screen = to_screen.inverse();

            // paint maps
            if self.ui_data.overlay_terrain {
                if let Some(texture) = &self.background {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }
            if self.ui_data.overlay_paths {
                if let Some(texture) = &self.foreground {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }
            if self.ui_data.overlay_textures {
                if let Some(texture) = &self.textured {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }

            // Responses

            // hover
            if let Some(pointer_pos) = response.hover_pos() {
                let canvas_pos = from_screen * pointer_pos;

                let canvas_pos_x = canvas_pos.x as usize;
                let canvas_pos_y = canvas_pos.y as usize;
                let i = ((canvas_pos_y * self.dimensions.width()) + canvas_pos_x) as usize;

                if i < self.heights.len() {
                    let value = self.heights[i];

                    let x = canvas_pos.x as usize / VERTEX_CNT;
                    let y = canvas_pos.y as usize / VERTEX_CNT;
                    let cx = self.dimensions.tranform_to_cell_x(x as i32);
                    let cy = self.dimensions.tranform_to_cell_y(y as i32);
                    self.info = format!("({}, {}), height: {}", cx, cy, value);
                }

                if self.ui_data.show_tooltips {
                    egui::show_tooltip(ui.ctx(), egui::Id::new("my_tooltip"), |ui| {
                        ui.label(self.info.clone());
                    });
                }
            }

            // panning
            if response.drag_started() {
                if let Some(drag_start) = response.interact_pointer_pos() {
                    self.zoom_data.drag_start = drag_start;
                }
            } else if response.dragged() {
                if let Some(current_pos) = response.interact_pointer_pos() {
                    let delta = current_pos - self.zoom_data.drag_start.to_vec2();
                    self.zoom_data.drag_delta = Some(delta);
                    self.zoom_data.drag_start = current_pos;
                }
            }

            // zoom
            let delta = ctx.input(|i| i.zoom_delta());
            // let delta = response.input(|i| i.zoom_delta());
            if delta != 1.0 {
                self.zoom_data.zoom_delta = Some(delta);
            }
            if response.middle_clicked() {
                self.reset_zoom();
                self.reset_pan();
            }

            // Make sure we allocate what we used (everything)
            ui.expand_to_include_rect(painter.clip_rect());

            // settings
            // TODO dumb hack
            let settings_rect = egui::Rect::from_min_max(response.rect.min, pos2(0.0, 0.0));
            ui.put(settings_rect, egui::Label::new(""));

            egui::Frame::popup(ui.style())
                .stroke(egui::Stroke::NONE)
                .show(ui, |ui| {
                    ui.set_max_width(270.0);
                    egui::CollapsingHeader::new("Settings ").show(ui, |ui| self.options_ui(ui));
                });

            response.context_menu(|ui| {
                if ui.button("Save as image").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(original_path) = file_option {
                        // combined
                        let img = self.get_background();
                        let img2 = self.get_foreground();
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img, self.dimensions) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }
                    }

                    ui.close_menu();
                }

                if ui.button("Save as layers").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(original_path) = file_option {
                        // save layers
                        let img = self.get_background();
                        let mut new_path = append_number_to_filename(&original_path, 1);
                        match save_image(new_path, &img, self.dimensions) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        let img2 = self.get_foreground();
                        new_path = append_number_to_filename(&original_path, 2);
                        match save_image(new_path, &img2, self.dimensions) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        // combined
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img, self.dimensions) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }
                    }

                    ui.close_menu();
                }
            });
        });
    }
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

fn save_image(
    path: PathBuf,
    color_image: &ColorImage,
    dimensions: Dimensions,
) -> Result<(), image::ImageError> {
    // get image

    let pixels = color_image.as_raw();

    // Create an RgbaImage from the raw pixel data
    if let Some(img) = RgbaImage::from_raw(
        dimensions.width() as u32,
        dimensions.height() as u32,
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
