use egui::{ahash::HashMap, emath, pos2, Color32, Pos2, Rect, Rounding, Sense, Vec2};
use palette::{rgb::Rgb, FromColor, Hsv};
use tes3::esp::{Landscape, Plugin};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    landscape_records: HashMap<(i32, i32), Landscape>,

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
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Load").clicked() {
                            let file_option = rfd::FileDialog::new()
                                .add_filter("esm", &["esm"])
                                .add_filter("esp", &["esp"])
                                .pick_file();

                            if let Some(path) = file_option {
                                let mut plugin = Plugin::new();

                                // clear
                                self.landscape_records.clear();
                                self.min_x = 0;
                                self.max_x = 0;
                                self.min_y = 0;
                                self.max_y = 0;

                                if plugin.load_path(&path).is_ok() {
                                    for record in plugin.into_objects_of_type::<Landscape>() {
                                        // get grid dimensions
                                        let key = record.grid;
                                        let x = key.0;
                                        let y = key.1;
                                        if x < self.min_x {
                                            self.min_x = x;
                                        }
                                        if y < self.min_y {
                                            self.min_y = y;
                                        }
                                        if x > self.max_x {
                                            self.max_x = x;
                                        }
                                        if y > self.max_y {
                                            self.max_y = y;
                                        }

                                        // store records
                                        self.landscape_records.insert(key, record);
                                    }
                                }
                            }
                        }

                        ui.separator();

                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.heading("Cells");

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Add a lot of widgets here.
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

            // cell vertex heights
            if let Some(landscape) = &self.current_landscape {
                // get vertex data
                if let Some(vertex_heights) = &landscape.vertex_heights {
                    ui.horizontal(|ui| {
                        ui.label(format!("({},{})", landscape.grid.0, landscape.grid.1));
                        ui.label(self.info.clone());
                    });

                    ui.separator();

                    let (response, painter) =
                        ui.allocate_painter(ui.available_size_before_wrap(), Sense::hover());

                    let to_screen = emath::RectTransform::from_to(
                        Rect::from_min_size(Pos2::ZERO, Vec2::new(65.0, 65.0)),
                        response.rect,
                    );
                    let from_screen = to_screen.inverse();

                    let data = vertex_heights.data.clone();

                    let mut shapes = vec![];

                    let mut heights = [[0.0; VERTEX_CNT]; VERTEX_CNT];
                    for y in 0..VERTEX_CNT {
                        for x in 0..VERTEX_CNT {
                            heights[y][x] = data[y][x] as f32;
                        }
                    }

                    // decode
                    let mut offset: f32 = 0.0;
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
                        }
                    }

                    if let Some(pointer_pos) = response.hover_pos() {
                        let canvas_pos = from_screen * pointer_pos;

                        let x = canvas_pos.x as usize;
                        let y = canvas_pos.y as usize;

                        if x < VERTEX_CNT && y < VERTEX_CNT {
                            let value = heights[y][x];
                            self.info = format!(
                                "({}, {}), offset: {}, height: {}",
                                x, y, vertex_heights.offset, value,
                            );
                        }
                    }

                    // print
                    for (y, row) in heights.iter().enumerate() {
                        for (x, value) in row.iter().enumerate() {
                            let color = get_color_for_height(*value);
                            // map to screen space
                            let x_f32 = x as f32;
                            let y_f32 = (VERTEX_CNT - y) as f32;
                            let min = to_screen * pos2(x_f32, y_f32);
                            let max = to_screen * pos2(x_f32 + ZOOM, y_f32 + ZOOM);
                            let rect = Rect::from_min_max(min, max);

                            let shape = egui::Shape::rect_filled(rect, Rounding::default(), color);
                            shapes.push(shape);
                        }
                    }

                    painter.extend(shapes);
                }
            } else {
                ui.label("No cell selected");
            }
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
