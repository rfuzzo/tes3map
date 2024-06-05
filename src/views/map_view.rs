use eframe::emath::{pos2, Pos2, Rect, RectTransform};
use eframe::epaint::{Color32, Rounding, Shape, Stroke};
use egui::Sense;

use crate::{CellKey, EBackground, height_from_screen_space, save_image, TemplateApp, VERTEX_CNT};
use crate::overlay::cities::get_cities_shapes;
use crate::overlay::grid::get_grid_shapes;
use crate::overlay::regions::get_region_shapes;
use crate::overlay::travel::get_travel_shapes;

impl TemplateApp {
    fn cellkey_from_screen(&mut self, from_screen: RectTransform, pointer_pos: Pos2) -> CellKey {
        let transformed_position = from_screen * pointer_pos;
        // get cell grid
        self.dimensions
            .tranform_to_cell(Pos2::new(transformed_position.x, transformed_position.y))
    }

    fn get_rect_at_cell(&mut self, to_screen: RectTransform, key: CellKey) -> Rect {
        let cell_size = self.dimensions.cell_size();
        let p00x = cell_size * self.dimensions.tranform_to_canvas_x(key.0);
        let p00y = cell_size * self.dimensions.tranform_to_canvas_y(key.1);
        let p00 = Pos2::new(p00x as f32, p00y as f32);
        let p11 = Pos2::new((p00x + cell_size) as f32, (p00y + cell_size) as f32);
        Rect::from_two_pos(to_screen * p00, to_screen * p11)
    }

    pub fn map_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
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
            // settings
            egui::Frame::popup(ui.style())
                .stroke(Stroke::NONE)
                .show(ui, |ui| {
                    ui.set_max_width(170.0);
                    egui::CollapsingHeader::new("Settings")
                        .show(ui, |ui| self.settings_ui(ui, ctx));
                });

            return;
        }

        // painter
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());

        // zoom
        if let Some(delta) = self.zoom_data.drag_delta.take() {
            self.zoom_data.drag_offset += delta.to_vec2();
        }

        if let Some(z) = self.zoom_data.zoom_delta.take() {
            let r = z - 1.0;
            let mut current_zoom = self.zoom_data.zoom;
            current_zoom += r;
            if current_zoom > 0.0 {
                self.zoom_data.zoom = current_zoom;

                // TODO offset the image for smooth zoom
                if let Some(pointer_pos) = response.hover_pos() {
                    let d = pointer_pos * r;
                    self.zoom_data.drag_offset -= d.to_vec2();
                }
            }
        }

        // TODO cut off pan at (0,0)
        // zoomed and panned canvas
        let min = self.zoom_data.drag_offset;
        let max = response.rect.max * self.zoom_data.zoom + self.zoom_data.drag_offset.to_vec2();
        let canvas = Rect::from_min_max(min, max);

        // transforms
        let pixel_width = self.dimensions.width() as f32 * self.dimensions.cell_size() as f32;
        let pixel_height = self.dimensions.height() as f32 * self.dimensions.cell_size() as f32;
        let to = canvas;
        let from: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(pixel_width, pixel_height));
        let to_screen = RectTransform::from_to(from, to);
        let from_screen = to_screen.inverse();

        // paint maps
        let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));
        // let rx = (response.rect.max.x - response.rect.min.x) / pixel_width;
        // let ry = (response.rect.max.y - response.rect.min.y) / pixel_height;
        // let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(rx, ry));

        // Background
        if let Some(texture) = &self.bg {
            painter.image(texture.into(), canvas, uv, Color32::WHITE);
        }

        // TODO Overlays
        if self.ui_data.overlay_paths {
            // let texture =
            //     get_color_pixels(&self.dimensions, &self.land_records, self.ui_data.alpha);
            // TODO texture handles
            // painter.image(texture, canvas, uv, Color32::WHITE);
        }
        if self.ui_data.overlay_region {
            let shapes = get_region_shapes(
                to_screen,
                &self.dimensions,
                &self.regn_records,
                &self.cell_records,
            );
            painter.extend(shapes);
        }
        if self.ui_data.overlay_grid {
            let shapes = get_grid_shapes(to_screen, &self.dimensions);
            painter.extend(shapes);
        }
        if self.ui_data.overlay_cities {
            let shapes = get_cities_shapes(to_screen, &self.dimensions, &self.cell_records);
            painter.extend(shapes);
        }
        if self.ui_data.overlay_travel {
            let shapes = get_travel_shapes(to_screen, &self.dimensions, &self.travel_edges);
            painter.extend(shapes);
        }
        // overlay selected cell
        if let Some(key) = self.selected_id {
            let rect = self.get_rect_at_cell(to_screen, key);
            let shape =
                Shape::rect_stroke(rect, Rounding::default(), Stroke::new(4.0, Color32::RED));
            painter.add(shape);
        }

        // Responses

        // hover
        if let Some(pointer_pos) = response.hover_pos() {
            let key = self.cellkey_from_screen(from_screen, pointer_pos);
            self.hover_pos = key;

            let transformed_position = from_screen * pointer_pos;
            if let Some(value) = height_from_screen_space(
                &self.heights,
                &self.dimensions,
                transformed_position.x as usize / VERTEX_CNT,
                transformed_position.y as usize / VERTEX_CNT,
            ) {
                self.info = format!("({:?}), height: {}", key, value);
            }

            if self.ui_data.show_tooltips {
                egui::show_tooltip(ui.ctx(), egui::Id::new("hover_tooltip"), |ui| {
                    ui.label(self.info.clone());
                });
            }
        }

        // click
        if let Some(interact_pos) = painter.ctx().pointer_interact_pos() {
            if ui.ctx().input(|i| i.pointer.primary_clicked()) {
                // if in the cell panel, we select the cell
                let key = self.cellkey_from_screen(from_screen, interact_pos);
                if self.cell_records.contains_key(&key) {
                    // toggle selection
                    if self.selected_id == Some(key) {
                        // toggle off if the same cell is clicked
                        self.selected_id = None;
                    } else {
                        self.selected_id = Some(key);
                    }
                }
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
        let settings_rect = Rect::from_min_max(response.rect.min, pos2(0.0, 0.0));
        ui.put(settings_rect, egui::Label::new(""));

        egui::Frame::popup(ui.style())
            .stroke(Stroke::NONE)
            .show(ui, |ui| {
                ui.set_max_width(270.0);
                egui::CollapsingHeader::new("Settings ").show(ui, |ui| self.settings_ui(ui, ctx));
            });

        response.context_menu(|ui| {
            if ui.button("Reset zoom").clicked() {
                self.reset_pan();
                self.reset_zoom();
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Save as image").clicked() {
                let file_option = rfd::FileDialog::new()
                    .add_filter("png", &["png"])
                    .save_file();

                if let Some(original_path) = file_option {
                    let mut image = None;
                    match self.ui_data.background {
                        EBackground::None => {}
                        EBackground::Landscape => {}
                        EBackground::HeightMap => {
                            image = Some(self.get_heightmap_image());
                        }
                        EBackground::GameMap => {
                            image = Some(self.get_gamemap_image());
                        }
                    }

                    if let Some(image) = image {
                        if let Err(e) = save_image(&original_path, &image) {
                            println!("{}", e)
                        }
                    }

                    // if self.ui_data.overlay_paths {
                    //     todo!()
                    // } else if self.ui_data.overlay_region {
                    //     todo!()
                    // } else if self.ui_data.overlay_grid {
                    //     todo!()
                    // } else if self.ui_data.overlay_cities {
                    //     todo!()
                    // } else if self.ui_data.overlay_travel {
                    //     todo!()
                    // } else {
                    //     todo!()
                    // }
                }
            }

            ui.close_menu();
        });
    }
}
