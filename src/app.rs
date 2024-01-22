use std::collections::HashMap;

use eframe::glow::COLOR;
use egui::{emath, epaint::TextShape, pos2, Color32, Pos2, Rect, Rounding, Sense, Vec2};
use palette::{rgb::Rgb, FromColor, Hsv};
use tes3::esp::{Landscape, Plugin};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    paint_all: bool,
    #[serde(skip)]
    heights_map: HashMap<(i32, i32), [[f32; 65]; 65]>,
    #[serde(skip)]
    landscape_records: HashMap<(i32, i32), Landscape>,

    #[serde(skip)]
    shapes: Vec<egui::Shape>,
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

    fn generate_shapes(&mut self, to_screen: emath::RectTransform) {
        //for cy in (self.min_y..self.max_y + 1).rev() {
        for cy in (self.min_y..self.max_y + 1) {
            for cx in self.min_x..self.max_x + 1 {
                let tx = cx;
                let ty = cy;

                if let Some(heights) = self.heights_map.get(&(cx, cy)) {
                    // look up heightmap
                    for (y, row) in heights.iter().enumerate() {
                        for (x, value) in row.iter().enumerate() {
                            let color = get_color_for_height(*value);
                            // map to screen space
                            let x_f32 = (VERTEX_CNT as i32 * tx) as f32 + x as f32;
                            let y_f32 = (VERTEX_CNT as i32 * ty) as f32 + y as f32;
                            let min = to_screen * pos2(x_f32, y_f32);
                            let max = to_screen * pos2(x_f32 + ZOOM, y_f32 + ZOOM);
                            let rect = Rect::from_min_max(min, max);

                            let shape = egui::Shape::rect_filled(rect, Rounding::default(), color);
                            self.shapes.push(shape);
                        }
                    }
                } else {
                    // print empty rect
                    let x = (VERTEX_CNT as i32 * tx) as f32;
                    let y = (VERTEX_CNT as i32 * ty) as f32;

                    let min = pos2(x, y);
                    let max = min + Vec2::new(VERTEX_CNT as f32, VERTEX_CNT as f32);
                    let rect = Rect::from_min_max(to_screen * min, to_screen * max);

                    let shape = egui::Shape::rect_filled(rect, Rounding::default(), Color32::BLACK);
                    self.shapes.push(shape);
                }
            }
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                ui.menu_button("File", |ui| {
                    if ui.button("Load").clicked() {
                        let file_option = rfd::FileDialog::new()
                            .add_filter("esm", &["esm"])
                            .add_filter("esp", &["esp"])
                            .pick_file();

                        if let Some(path) = file_option {
                            let mut plugin = Plugin::new();

                            // clear
                            self.shapes.clear();
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
                                            self.shapes.clear();
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

            // cell vertex heights
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

            let from = if self.paint_all {
                Rect::from_min_max(
                    pos2(
                        VERTEX_CNT as f32 * self.min_x as f32,
                        VERTEX_CNT as f32 * self.min_y as f32,
                    ),
                    pos2(
                        VERTEX_CNT as f32 * self.max_x as f32,
                        VERTEX_CNT as f32 * self.max_y as f32,
                    ),
                )
            } else {
                Rect::from_min_max(pos2(0.0, 0.0), pos2(VERTEX_CNT as f32, VERTEX_CNT as f32))
            };

            let to_screen = emath::RectTransform::from_to(from, response.rect);
            let from_screen = to_screen.inverse();

            // paint
            if self.paint_all {
                if self.shapes.is_empty() {
                    self.generate_shapes(to_screen);
                }
            } else if self.shapes.is_empty() {
                // cell vertex heights
                if let Some(landscape) = &self.current_landscape {
                    if let Some(heights) =
                        self.heights_map.get(&(landscape.grid.0, landscape.grid.1))
                    {
                        for (y, row) in heights.iter().enumerate() {
                            for (x, value) in row.iter().enumerate() {
                                let color = get_color_for_height(*value);
                                // map to screen space
                                let x_f32 = x as f32;
                                let y_f32 = (VERTEX_CNT - y) as f32;
                                let min = to_screen * pos2(x_f32, y_f32);
                                let max = to_screen * pos2(x_f32 + ZOOM, y_f32 + ZOOM);
                                let rect = Rect::from_min_max(min, max);

                                let shape =
                                    egui::Shape::rect_filled(rect, Rounding::default(), color);
                                self.shapes.push(shape);
                            }
                        }
                    }
                }
            }

            painter.extend(self.shapes.clone());
        });
    }
}

const ZOOM: f32 = 1.0;
const VERTEX_CNT: usize = 65;

fn map_height_to_color(height: f32) -> Color32 {
    // Assuming height ranges from 0 to 2000
    let normalized_height = height / 2000.0;

    // Map normalized height to hue in the range [0.0, 240.0]
    let hue = normalized_height * 240.0;

    // Set saturation and value to create a nice gradient
    let saturation = 1.0;
    let value = 0.8;

    // Create an HSV color
    //let color = Hsv::new(hue, saturation, value);

    let hsv_u8 = Hsv::new_srgb(hue, saturation, value);
    //let hsv_f32 = hsv_u8.into_format::<f32>();

    // Convert HSV to RGB
    let rgb = Rgb::from_color(hsv_u8);

    Color32::from_rgb(rgb.red as u8, rgb.green as u8, rgb.blue as u8)
}

fn get_color_for_height(value: f32) -> Color32 {
    if value < 0.0 {
        Color32::BLUE
    } else {
        Color32::BROWN
        //map_height_to_color(value)
    }
}
