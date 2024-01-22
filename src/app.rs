use std::collections::HashMap;

use egui::{pos2, Color32, ColorImage, Rect, Sense};
use tes3::esp::{Landscape, Plugin};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    paint_all: bool,

    #[serde(skip)]
    heights_map: HashMap<(i32, i32), [[f32; 65]; 65]>,
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

    fn load_data(&mut self, plugin: Plugin) {
        let mut min_x: Option<i32> = None;
        let mut min_y: Option<i32> = None;
        let mut max_x: Option<i32> = None;
        let mut max_y: Option<i32> = None;

        for record in plugin.into_objects_of_type::<Landscape>() {
            // get grid dimensions
            let key = record.grid;
            let x = key.0;
            let y = key.1;

            if let Some(minx) = min_x {
                if x < minx {
                    min_x = Some(x);
                }
            } else {
                min_x = Some(x);
            }
            if let Some(miny) = min_y {
                if y < miny {
                    min_y = Some(y);
                }
            } else {
                min_y = Some(y);
            }
            if let Some(maxx) = max_x {
                if x > maxx {
                    max_x = Some(x);
                }
            } else {
                max_x = Some(x);
            }
            if let Some(maxy) = max_y {
                if y > maxy {
                    max_y = Some(y);
                }
            } else {
                max_y = Some(y);
            }

            // store records
            self.landscape_records.insert(key, record);
        }

        self.min_x = min_x.unwrap();
        self.min_y = min_y.unwrap();
        self.max_x = max_x.unwrap();
        self.max_y = max_y.unwrap();

        // calculate heights
        for cy in self.min_y..self.max_y + 1 {
            for cx in self.min_x..self.max_x + 1 {
                //let mut heights_vec = vec![];

                if let Some(landscape) = &self.landscape_records.get(&(cx, cy)) {
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
                        for y in 0..VERTEX_CNT {
                            for x in 0..VERTEX_CNT {
                                offset += heights[y][x];
                                heights[y][x] = offset;
                            }
                            offset = heights[y][0];
                        }

                        for row in &mut heights {
                            for height in row {
                                *height *= 8.0;
                            }
                        }

                        self.heights_map.insert((cx, cy), heights);
                    }
                }
            }
        }
    }

    fn generate_image(&mut self) -> ColorImage {
        // dimensions
        let nx = (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32);
        let ny = (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32);
        let mut pixel: Vec<f32> = vec![-1.0; nx as usize * ny as usize];

        for cy in self.min_y..self.max_y + 1 {
            for cx in self.min_x..self.max_x + 1 {
                let tx = cx - self.min_x;
                let ty = self.max_y - cy;

                if let Some(heights) = self.heights_map.get(&(cx, cy)) {
                    // look up heightmap
                    for (y, row) in heights.iter().enumerate() {
                        for (x, value) in row.iter().enumerate() {
                            let x_f32 = (VERTEX_CNT as i32 * tx) + x as i32;
                            let y_f32 = (VERTEX_CNT as i32 * ty) + (VERTEX_CNT as i32 - y as i32);

                            let i = (y_f32 * nx) + x_f32;
                            pixel[i as usize] = *value;
                        }
                    }
                } else {
                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            let x_f32 = (VERTEX_CNT as i32 * tx) + x as i32;
                            let y_f32 = (VERTEX_CNT as i32 * ty) + y as i32;

                            let i = (y_f32 * nx) + x_f32;
                            pixel[i as usize] = -1.0;
                        }
                    }
                }
            }
        }

        let size = [nx as usize, ny as usize];
        let mut img = ColorImage::new(size, Color32::BLUE);
        let p = pixel
            .iter()
            .map(|f| get_color_for_height(*f))
            .collect::<Vec<_>>();
        img.pixels = p;

        img
    }

    // fn paint_cell(&mut self, to_screen: emath::RectTransform) {
    //     if let Some(landscape) = &self.current_landscape {
    //         if let Some(heights) = self.heights_map.get(&(landscape.grid.0, landscape.grid.1)) {
    //             for (y, row) in heights.iter().enumerate() {
    //                 for (x, value) in row.iter().enumerate() {
    //                     let color = get_color_for_height(*value);
    //                     // map to screen space
    //                     let x_f32 = x as f32;
    //                     let y_f32 = (VERTEX_CNT - y) as f32;
    //                     let min = to_screen * pos2(x_f32, y_f32);
    //                     let max = to_screen * pos2(x_f32 + ZOOM, y_f32 + ZOOM);
    //                     let rect = Rect::from_min_max(min, max);

    //                     let shape = egui::Shape::rect_filled(rect, Rounding::default(), color);
    //                     //self.shapes.push(shape);
    //                 }
    //             }
    //         }
    //     }
    // }
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
                    if ui.button("Load").clicked() {
                        let file_option = rfd::FileDialog::new()
                            .add_filter("esm", &["esm"])
                            .add_filter("esp", &["esp"])
                            .pick_file();

                        if let Some(path) = file_option {
                            let mut plugin = Plugin::new();

                            // clear
                            self.texture = None;
                            self.landscape_records.clear();
                            self.min_x = 0;
                            self.max_x = 0;
                            self.min_y = 0;
                            self.max_y = 0;

                            if plugin.load_path(&path).is_ok() {
                                self.load_data(plugin);
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

        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.heading("Cells");
            ui.checkbox(&mut self.paint_all, "Paint whole map");
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for y in (self.min_y..=self.max_y).rev() {
                        let mut any = false;
                        for x in self.min_x..=self.max_x {
                            if let Some(_v) = self.landscape_records.get(&(x, y)) {
                                any = true;
                            }
                        }
                        if any {
                            ui.collapsing(format!("Y: {y}"), |ui| {
                                for x in self.min_x..=self.max_x {
                                    if let Some(v) = self.landscape_records.get(&(x, y)) {
                                        if ui.button(format!("({x},{y})")).clicked() {
                                            // store
                                            self.texture = None;
                                            self.current_landscape = Some(v.clone());
                                        }
                                    }
                                }
                            });
                        }
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!(
                "Map (Max y: {}, Min y: {}, Max x: {}, Min x: {})",
                self.max_y, self.min_y, self.max_x, self.min_x
            ));
            ui.separator();

            if self.heights_map.is_empty() {
                return;
            }

            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

            let _from = if self.paint_all {
                let nx = (1 + self.max_x - self.min_x) * (VERTEX_CNT as i32);
                let ny = (1 + self.max_y - self.min_y) * (VERTEX_CNT as i32);

                egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(nx as f32, ny as f32))
            } else {
                egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(VERTEX_CNT as f32, VERTEX_CNT as f32))
            };

            //let to_screen = egui::emath::RectTransform::from_to(from, response.rect);
            //let from_screen = to_screen.inverse();

            // paint
            if self.paint_all {
                if self.texture.is_none() {
                    let img = self.generate_image();
                    let _texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
                        // Load the texture only once.
                        ui.ctx().load_texture("my-image", img, Default::default())
                    });
                }
            }
            // else if self.pixel.is_empty() {
            //     // cell vertex heights
            //     self.paint_cell(to_screen);
            // }

            //painter.extend(self.shapes.clone());
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

//const ZOOM: f32 = 1.0;
const VERTEX_CNT: usize = 65;

fn get_color_for_height(value: f32) -> Color32 {
    if value < 0.0 {
        Color32::BLUE
    } else {
        Color32::BROWN
        //map_height_to_color(value)
    }
}
