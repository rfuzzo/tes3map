use std::{collections::HashMap, env, path::PathBuf};

use egui::{pos2, Color32, ColorImage, Pos2, Rect, Sense, Vec2};
use tes3::esp::{Landscape, Plugin};

use crate::{
    create_image, generate_pixels, get_plugins_sorted, Dimensions, UiData, ZoomData, VERTEX_CNT,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    cwd: Option<PathBuf>,

    // tes3
    #[serde(skip)]
    landscape_records: HashMap<(i32, i32), Landscape>,

    // painting
    #[serde(skip)]
    dimensions: Dimensions,
    #[serde(skip)]
    pixels: Vec<f32>,
    #[serde(skip)]
    texture: Option<egui::TextureHandle>,

    // ui
    ui_data: UiData,

    #[serde(skip)]
    info: String,

    #[serde(skip)]
    zoom_data: ZoomData,

    #[serde(skip)]
    zinfo: String,
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
    fn load_data(&mut self, records: &HashMap<(i32, i32), Landscape>) -> Option<ColorImage> {
        self.texture = None;

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

        let min_y = min_y?;
        let max_y = max_y?;
        let min_x = min_x?;
        let max_x = max_x?;

        // calculate heights
        let mut min_z: Option<f32> = None;
        let mut max_z: Option<f32> = None;
        let mut heights_map: HashMap<(i32, i32), [[f32; 65]; 65]> = HashMap::default();
        for cy in min_y..max_y + 1 {
            for cx in min_x..max_x + 1 {
                //let mut heights_vec = vec![];

                if let Some(landscape) = records.get(&(cx, cy)) {
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

        let pixels = generate_pixels(dimensions, heights_map);
        let img = create_image(&pixels, dimensions, self.ui_data);

        // assign data
        self.landscape_records = records.clone();
        self.dimensions = dimensions;
        self.pixels = pixels;

        Some(img)
    }

    fn load_folder(&mut self, path: &PathBuf, ctx: &egui::Context) {
        let plugins = get_plugins_sorted(&path, false);

        let mut records: HashMap<(i32, i32), Landscape> = HashMap::default();
        for path in plugins {
            let mut plugin = Plugin::new();
            if plugin.load_path(&path).is_ok() {
                let objects = plugin
                    .into_objects_of_type::<Landscape>()
                    .collect::<Vec<_>>();
                for r in objects {
                    let key = r.grid;
                    records.insert(key, r);
                }
            }
        }

        if let Some(img) = self.load_data(&records) {
            let _texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
                // Load the texture only once.
                ctx.load_texture("my-image", img, Default::default())
            });
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
                        let folder_option = rfd::FileDialog::new()
                            .add_filter("esm", &["esm"])
                            .add_filter("esp", &["esp"])
                            .pick_folder();

                        if let Some(path) = folder_option {
                            self.load_folder(&path, ctx);
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
                                if let Some(img) = self.load_data(&records) {
                                    let _texture: &egui::TextureHandle =
                                        self.texture.get_or_insert_with(|| {
                                            // Load the texture only once.
                                            ui.ctx().load_texture(
                                                "my-image",
                                                img,
                                                Default::default(),
                                            )
                                        });
                                }
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
                self.dimensions.min_y,
                self.dimensions.max_y,
                self.dimensions.min_x,
                self.dimensions.max_x,
                self.dimensions.min_z,
                self.dimensions.max_z
            ));
            ui.horizontal(|ui| {
                ui.label("Info: ");
                ui.label(&self.info);
                ui.separator();
                ui.label(&self.zinfo);
            });
            // toolbar
            ui.horizontal(|ui| {
                //ui.add(egui::Slider::new(&mut self.zoom_data.zoom2, 0..=100).text("Zoom"));

                ui.separator();

                ui.color_edit_button_srgba(&mut self.ui_data.height_base);
                ui.add(
                    egui::Slider::new(&mut self.ui_data.height_spectrum, 0..=360)
                        .text("Height offset"),
                );
                ui.color_edit_button_srgba(&mut self.ui_data.depth_base);
                ui.add(
                    egui::Slider::new(&mut self.ui_data.depth_spectrum, 0..=360)
                        .text("Depth offset"),
                );
                if ui.button("Default").clicked() {
                    self.ui_data = UiData::default();
                }
                if ui.button("Reload").clicked() {
                    let img = create_image(&self.pixels, self.dimensions, self.ui_data);
                    let handle = ui.ctx().load_texture("my-image", img, Default::default());
                    self.texture = Some(handle);
                }
            });

            ui.separator();

            if self.pixels.is_empty() {
                return;
            }

            // painter
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

                // TODO
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
                pos2(self.dimensions.nx() as f32, self.dimensions.ny() as f32),
            );
            let to_screen = egui::emath::RectTransform::from_to(from, to);
            let from_screen = to_screen.inverse();

            // paint
            if let Some(texture) = &self.texture {
                painter.image(texture.into(), canvas, uv, Color32::WHITE)
            }

            // hover
            if let Some(pointer_pos) = response.hover_pos() {
                let canvas_pos = from_screen * pointer_pos;

                let canvas_pos_x = canvas_pos.x as i32;
                let canvas_pos_y = canvas_pos.y as i32;
                let i = ((canvas_pos_y * self.dimensions.nx()) + canvas_pos_x) as usize;

                if i < self.pixels.len() {
                    let value = self.pixels[i];

                    let x = canvas_pos.x as usize / VERTEX_CNT;
                    let y = canvas_pos.y as usize / VERTEX_CNT;
                    let cx = self.dimensions.tranform_to_cell_x(x as i32);
                    let cy = self.dimensions.tranform_to_cell_y(y as i32);
                    self.info = format!("({}, {}), height: {}", cx, cy, value);
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
            if delta != 1.0 {
                self.zoom_data.zoom_delta = Some(delta);
            }

            if response.middle_clicked() {
                self.reset_zoom();
                self.reset_pan();
            }
        });
    }
}
