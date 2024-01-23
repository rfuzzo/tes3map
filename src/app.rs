use std::collections::HashMap;

use egui::{pos2, Color32, ColorImage, Rect, Sense};
use tes3::esp::{Landscape, Plugin};

use crate::{get_color_for_height, get_plugins_sorted};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    depth_spectrum: usize,
    depth_base: Color32,
    height_spectrum: usize,
    height_base: Color32,

    #[serde(skip)]
    pixels: Vec<f32>,
    #[serde(skip)]
    landscape_records: HashMap<(i32, i32), Landscape>,

    #[serde(skip)]
    texture: Option<egui::TextureHandle>,
    #[serde(skip)]
    info: String,
    #[serde(skip)]
    current_landscape: Option<Landscape>,

    #[serde(skip)]
    min_x: i32,
    #[serde(skip)]
    min_y: i32,
    #[serde(skip)]
    max_x: i32,
    #[serde(skip)]
    max_y: i32,

    #[serde(skip)]
    min_z: f32,
    #[serde(skip)]
    max_z: f32,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            depth_spectrum: 20,
            depth_base: Color32::BLUE,
            height_spectrum: 120,
            height_base: Color32::DARK_GREEN,
            pixels: Vec::default(),
            landscape_records: Default::default(),
            texture: Default::default(),
            info: Default::default(),
            current_landscape: Default::default(),
            min_x: Default::default(),
            min_y: Default::default(),
            max_x: Default::default(),
            max_y: Default::default(),
            min_z: Default::default(),
            max_z: Default::default(),
        }
    }
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

    fn load_data(&mut self, records: &HashMap<(i32, i32), Landscape>) -> ColorImage {
        // clear
        self.texture = None;
        self.landscape_records.clear();
        self.min_x = 0;
        self.max_x = 0;
        self.min_y = 0;
        self.max_y = 0;

        // get dimensions
        let mut min_x: Option<i32> = None;
        let mut min_y: Option<i32> = None;
        let mut max_x: Option<i32> = None;
        let mut max_y: Option<i32> = None;

        for key in records.keys() {
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

        self.min_x = min_x.unwrap();
        self.min_y = min_y.unwrap();
        self.max_x = max_x.unwrap();
        self.max_y = max_y.unwrap();

        let map = self.calculate_heights(records);
        self.generate_pixels(map)
    }

    fn calculate_heights(
        &mut self,
        landscape_records: &HashMap<(i32, i32), Landscape>,
    ) -> HashMap<(i32, i32), [[f32; 65]; 65]> {
        // calculate heights

        self.min_z = 0_f32;
        self.max_z = 0_f32;
        let mut min_z: Option<f32> = None;
        let mut max_z: Option<f32> = None;

        let mut heights_map: HashMap<(i32, i32), [[f32; 65]; 65]> = HashMap::default();
        for cy in self.min_y..self.max_y + 1 {
            for cx in self.min_x..self.max_x + 1 {
                //let mut heights_vec = vec![];

                if let Some(landscape) = landscape_records.get(&(cx, cy)) {
                    // get vertex data
                    if let Some(vertex_heights) = &landscape.vertex_heights {
                        // get data
                        let data = vertex_heights.data.clone();
                        let mut heights: [[f32; 65]; 65] = [[0.0; VERTEX_CNT]; VERTEX_CNT];
                        for y in 0..VERTEX_CNT {
                            for x in 0..VERTEX_CNT {
                                heights[y][x] = data[y][x] as f32;
                            }
                        }

                        // decode
                        let mut offset: f32 = vertex_heights.offset;
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

        self.landscape_records = landscape_records.clone();
        self.min_z = min_z.unwrap();
        self.max_z = max_z.unwrap();

        heights_map
    }

    fn generate_pixels(&mut self, heights_map: HashMap<(i32, i32), [[f32; 65]; 65]>) -> ColorImage {
        // dimensions
        let nx = (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32);
        let ny = (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32);
        let mut pixels = vec![-1.0; nx as usize * ny as usize];

        for cy in self.min_y..self.max_y + 1 {
            for cx in self.min_x..self.max_x + 1 {
                let tx = cx - self.min_x;
                let ty = self.max_y - cy;

                if let Some(heights) = heights_map.get(&(cx, cy)) {
                    // look up heightmap
                    for (y, row) in heights.iter().enumerate() {
                        for (x, value) in row.iter().enumerate() {
                            let x_f32 = (VERTEX_CNT as i32 * tx) + x as i32;
                            let y_f32 = (VERTEX_CNT as i32 * ty) + (VERTEX_CNT as i32 - y as i32);

                            let i = (y_f32 * nx) + x_f32;
                            pixels[i as usize] = *value;
                        }
                    }
                } else {
                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            let x_f32 = (VERTEX_CNT as i32 * tx) + x as i32;
                            let y_f32 = (VERTEX_CNT as i32 * ty) + y as i32;

                            let i = (y_f32 * nx) + x_f32;
                            pixels[i as usize] = -1.0;
                        }
                    }
                }
            }
        }

        let img = self.get_image(&pixels);
        self.pixels = pixels;
        img
    }

    fn get_image(&self, pixels: &[f32]) -> ColorImage {
        let nx = (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32);
        let ny = (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32);
        let size = [nx as usize, ny as usize];
        let mut img = ColorImage::new(size, Color32::BLUE);
        let p = pixels
            .iter()
            .map(|f| {
                get_color_for_height(
                    *f,
                    self.height_base,
                    self.height_spectrum,
                    self.max_z,
                    self.depth_base,
                    self.depth_spectrum,
                    self.min_z,
                )
            })
            .collect::<Vec<_>>();
        img.pixels = p;
        img
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load folder").clicked() {
                        let folder_option = rfd::FileDialog::new()
                            .add_filter("esm", &["esm"])
                            .add_filter("esp", &["esp"])
                            .pick_folder();

                        if let Some(path) = folder_option {
                            let plugins = get_plugins_sorted(&path, false);

                            let mut records: HashMap<(i32, i32), Landscape> = HashMap::default();
                            for path in plugins {
                                let mut plugin = Plugin::new();
                                if plugin.load_path(&path).is_ok() {
                                    for r in plugin.into_objects_of_type::<Landscape>() {
                                        let key = r.grid;
                                        records.insert(key, r);
                                    }
                                }
                            }

                            let img = self.load_data(&records);
                            let _texture: &egui::TextureHandle =
                                self.texture.get_or_insert_with(|| {
                                    // Load the texture only once.
                                    ui.ctx().load_texture("my-image", img, Default::default())
                                });
                        }

                        ui.close_menu();
                    }

                    if ui.button("Load plugin").clicked() {
                        let file_option = rfd::FileDialog::new()
                            .add_filter("esm", &["esm"])
                            .add_filter("esp", &["esp"])
                            .pick_file();

                        if let Some(path) = file_option {
                            let mut plugin = Plugin::new();
                            if plugin.load_path(&path).is_ok() {
                                let mut records: HashMap<(i32, i32), Landscape> =
                                    HashMap::default();
                                for r in plugin.into_objects_of_type::<Landscape>() {
                                    let key = r.grid;
                                    records.insert(key, r);
                                }
                                let img = self.load_data(&records);
                                let _texture: &egui::TextureHandle =
                                    self.texture.get_or_insert_with(|| {
                                        // Load the texture only once.
                                        ui.ctx().load_texture("my-image", img, Default::default())
                                    });
                            }
                        }

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
                self.min_y, self.max_y, self.min_x, self.max_x, self.min_z, self.max_z
            ));
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(&mut self.height_base);
                ui.add(egui::Slider::new(&mut self.height_spectrum, 0..=360).text("Height offset"));
                ui.color_edit_button_srgba(&mut self.depth_base);
                ui.add(egui::Slider::new(&mut self.depth_spectrum, 0..=360).text("Depth offset"));
                if ui.button("Reload").clicked() {
                    let img = self.get_image(&self.pixels);
                    let handle = ui.ctx().load_texture("my-image", img, Default::default());
                    self.texture = Some(handle);
                }
            });
            ui.separator();

            if self.pixels.is_empty() {
                return;
            }

            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

            let nx = (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32);
            let ny = (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32);
            let _from = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(nx as f32, ny as f32));

            //let to_screen = egui::emath::RectTransform::from_to(from, response.rect);
            //let from_screen = to_screen.inverse();

            // paint
            if let Some(texture) = &self.texture {
                painter.image(
                    texture.into(),
                    response.rect,
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE,
                )
            }
        });
    }
}

const VERTEX_CNT: usize = 65;
