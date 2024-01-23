use std::collections::HashMap;

use egui::{pos2, Color32, ColorImage, Rect, Sense};
use tes3::esp::{Landscape, Plugin};

use crate::{create_image, generate_pixels, get_plugins_sorted, Dimensions, UiData, VERTEX_CNT};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
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

                            if let Some(img) = self.load_data(&records) {
                                let _texture: &egui::TextureHandle =
                                    self.texture.get_or_insert_with(|| {
                                        // Load the texture only once.
                                        ui.ctx().load_texture("my-image", img, Default::default())
                                    });
                            }
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

            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

            let _from = egui::Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(self.dimensions.nx() as f32, self.dimensions.ny() as f32),
            );

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
