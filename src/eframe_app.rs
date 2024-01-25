use crate::*;

use std::env;

use egui::{pos2, Color32, Pos2, Rect, Sense};

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

        egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
            ui.heading("Cells");
            if ui.button("Paint all").clicked() {
                // paint all
                self.load_data(ctx);
            }

            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(d) = calculate_dimensions(&self.landscape_records) {
                        for y in (d.min_y..=d.max_y).rev() {
                            let mut any = false;
                            for x in d.min_x..=d.max_x {
                                if let Some(_v) = self.landscape_records.get(&(x, y)) {
                                    any = true;
                                }
                            }
                            if any {
                                ui.collapsing(format!("Y: {y}"), |ui| {
                                    for x in d.min_x..=d.max_x {
                                        if let Some(v) = self.landscape_records.get(&(x, y)) {
                                            if ui.button(format!("({x},{y})")).clicked() {
                                                // store
                                                self.current_landscape = Some(v.clone());

                                                let dimensions = Dimensions {
                                                    min_x: x,
                                                    min_y: y,
                                                    max_x: x,
                                                    max_y: y,
                                                    texture_size: TEXTURE_MAX_SIZE,
                                                };
                                                self.load_data_with_dimension(dimensions, ctx);
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading(format!(
                "Map (y: [{},{}]; x: [{},{}]; z: [{},{}])",
                self.dimensions.min_y,
                self.dimensions.max_y,
                self.dimensions.min_x,
                self.dimensions.max_x,
                self.dimensions_z.min_z,
                self.dimensions_z.max_z
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
            let from: Rect = egui::Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(
                    self.dimensions.width() as f32 * VERTEX_CNT as f32,
                    self.dimensions.height() as f32 * VERTEX_CNT as f32,
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
            if self.ui_data.overlay_textures {
                if let Some(texture) = &self.textured {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }
            if self.ui_data.overlay_paths {
                if let Some(texture) = &self.foreground {
                    painter.image(texture.into(), canvas, uv, Color32::WHITE);
                }
            }

            // Responses

            // hover
            if let Some(pointer_pos) = response.hover_pos() {
                let cell_pos_zeroed = from_screen * pointer_pos;

                // get pixel index
                let x = cell_pos_zeroed.x as usize;
                let y = cell_pos_zeroed.y as usize;

                let i = (y * self.dimensions.stride(VERTEX_CNT)) + x;

                if i < self.heights.len() {
                    // get cell grid
                    let cx = self.dimensions.tranform_to_cell_x((x / VERTEX_CNT) as i32);
                    let cy = self.dimensions.tranform_to_cell_y((y / VERTEX_CNT) as i32);

                    // get height
                    let value = self.heights[i as usize];
                    self.info = format!("({cx}, {cy}), height: {value}",);
                }

                if self.ui_data.show_tooltips {
                    egui::show_tooltip(ui.ctx(), egui::Id::new("hover_tooltip"), |ui| {
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
            // dumb ui hack
            let settings_rect = egui::Rect::from_min_max(response.rect.min, pos2(0.0, 0.0));
            ui.put(settings_rect, egui::Label::new(""));

            egui::Frame::popup(ui.style())
                .stroke(egui::Stroke::NONE)
                .show(ui, |ui| {
                    ui.set_max_width(270.0);
                    egui::CollapsingHeader::new("Settings ").show(ui, |ui| self.options_ui(ui));
                });

            response.context_menu(|ui| {
                if ui.button("Reset zoom").clicked() {
                    self.reset_pan();
                    self.reset_zoom();
                }

                ui.separator();

                if ui.button("Save as image").clicked() {
                    let file_option = rfd::FileDialog::new()
                        .add_filter("png", &["png"])
                        .save_file();

                    if let Some(original_path) = file_option {
                        // combined
                        let img = self.get_background();
                        let img2 = self.get_foreground();
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img) {
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
                        match save_image(new_path, &img) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        let img2 = self.get_foreground();
                        new_path = append_number_to_filename(&original_path, 2);
                        match save_image(new_path, &img2) {
                            Ok(_) => {}
                            Err(e) => println!("{}", e),
                        }

                        // combined
                        let layered_img = self.get_layered_image(img, img2);
                        match save_image(original_path, &layered_img) {
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
