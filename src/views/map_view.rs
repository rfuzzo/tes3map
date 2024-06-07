use eframe::emath::{pos2, Pos2, Rect, RectTransform};
use eframe::epaint::{Color32, Rounding, Shape, Stroke};
use egui::Sense;

use crate::{CellKey, EBackground, height_from_screen_space, save_image, TemplateApp, VERTEX_CNT};
use crate::app::TooltipInfo;
use crate::overlay::cities::get_cities_shapes;
use crate::overlay::conflicts::get_conflict_shapes;
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
            self.dimensions.min_z,
            self.dimensions.max_z
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

        // transforms
        let pixel_width = self.dimensions.width() as f32 * self.dimensions.cell_size() as f32;
        let pixel_height = self.dimensions.height() as f32 * self.dimensions.cell_size() as f32;

        let from: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(pixel_width, pixel_height));

        let min = self.zoom_data.drag_offset;
        let max = Pos2::new(response.rect.max.x, response.rect.max.x) * self.zoom_data.zoom
            + self.zoom_data.drag_offset.to_vec2();
        let canvas = Rect::from_min_max(min, max);

        let to_screen = RectTransform::from_to(from, canvas);
        let from_screen = to_screen.inverse();

        // paint maps

        let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));
        // let rx = (response.rect.max.x - response.rect.min.x) / pixel_width;
        // let ry = (response.rect.max.y - response.rect.min.y) / pixel_height;
        // let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(rx, ry));

        // Background
        if let Some(handle) = &self.background_handle {
            painter.image(handle.into(), canvas, uv, Color32::WHITE);
        }

        // Overlays
        if self.ui_data.overlay_paths {
            if let Some(handle) = &self.paths_handle {
                painter.image(handle.into(), canvas, uv, Color32::WHITE);
            }
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
        if self.ui_data.overlay_conflicts {
            let shapes = get_conflict_shapes(to_screen, &self.dimensions, &self.cell_conflicts);
            painter.extend(shapes);
        }

        // overlay selected cell
        if let Some(key) = self.runtime_data.selected_id {
            let rect = self.get_rect_at_cell(to_screen, key);
            let shape =
                Shape::rect_stroke(rect, Rounding::default(), Stroke::new(4.0, Color32::RED));
            painter.add(shape);
        }

        // Responses

        // hover
        if let Some(pointer_pos) = response.hover_pos() {
            let key = self.cellkey_from_screen(from_screen, pointer_pos);
            self.runtime_data.hover_pos = key;

            let mut tooltipinfo = TooltipInfo {
                key,
                height: 0.0,
                region: String::new(),
                cell_name: String::new(),
                conflicts: Vec::new(),
            };

            // get cell
            if let Some(cell) = self.cell_records.get(&key) {
                tooltipinfo.cell_name.clone_from(&cell.name);
                if let Some(region) = cell.region.as_ref() {
                    tooltipinfo.region.clone_from(region);
                }
            }

            // get height
            if self.ui_data.background == EBackground::HeightMap {
                let transformed_position = from_screen * pointer_pos;
                if let Some(height) = height_from_screen_space(
                    &self.heights,
                    &self.dimensions,
                    transformed_position.x as usize / VERTEX_CNT,
                    transformed_position.y as usize / VERTEX_CNT,
                ) {
                    tooltipinfo.height = height;
                }
            }

            // get conflicts
            if self.ui_data.show_tooltips {
                if let Some(conflicts) = self.cell_conflicts.get(&key) {
                    tooltipinfo.conflicts.clone_from(conflicts);
                }
            }

            self.runtime_data.info = tooltipinfo;

            if self.ui_data.show_tooltips {
                egui::show_tooltip(ui.ctx(), egui::Id::new("hover_tooltip"), |ui| {
                    let info = self.runtime_data.info.clone();
                    ui.label(format!("{:?} - {}", info.key, info.cell_name));
                    ui.label(format!("Region: {}", info.region));

                    // only show if current background is heightmap
                    if self.ui_data.background == EBackground::HeightMap {
                        ui.label("________");
                        ui.label(format!("Height: {}", info.height));
                    }

                    // show conflicts
                    if !info.conflicts.is_empty() {
                        ui.label("________");
                        ui.label("Conflicts:");
                        for conflict in info.conflicts {
                            // lookup plugin name by conflict id
                            if let Some(plugin) = self
                                .plugins
                                .as_ref()
                                .unwrap()
                                .iter()
                                .find(|p| p.hash == conflict)
                            {
                                ui.label(format!("  - {}", plugin.get_name()));
                            }
                        }
                    }
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
                    if self.runtime_data.selected_id == Some(key) {
                        // toggle off if the same cell is clicked
                        self.runtime_data.selected_id = None;
                    } else {
                        self.runtime_data.selected_id = Some(key);
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
                // construct default name from the first plugin name then the background type abbreviated
                let background_name = match self.ui_data.background {
                    EBackground::None => "",
                    EBackground::Landscape => "l",
                    EBackground::HeightMap => "h",
                    EBackground::GameMap => "g",
                };
                let first_plugin = self
                    .plugins
                    .as_ref()
                    .unwrap()
                    .iter()
                    .filter(|p| p.enabled)
                    .nth(0)
                    .unwrap();
                let plugin_name = first_plugin.get_name();
                let defaultname = format!("{}_{}.png", plugin_name, background_name);

                let file_option = rfd::FileDialog::new()
                    .add_filter("png", &["png"])
                    .set_file_name(defaultname)
                    .save_file();

                if let Some(original_path) = file_option {
                    let mut image = None;
                    match self.ui_data.background {
                        EBackground::None => {}
                        EBackground::Landscape => {
                            let max_texture_side = ctx.input(|i| i.max_texture_side);
                            image = Some(self.get_landscape_image(max_texture_side));
                        }
                        EBackground::HeightMap => {
                            image = Some(self.get_heightmap_image());
                        }
                        EBackground::GameMap => {
                            image = Some(self.get_gamemap_image());
                        }
                    }

                    if let Some(image) = image {
                        // TODO save shape overlays

                        let msg = if let Err(e) = save_image(&original_path, &image) {
                            format!("Error saving image: {}", e)
                        } else {
                            // message
                            format!("Image saved to: {}", original_path.display())
                        };

                        rfd::MessageDialog::new()
                            .set_title("Info")
                            .set_description(msg)
                            .set_buttons(rfd::MessageButtons::Ok)
                            .show();
                    }

                    ui.close_menu();
                }
            }
        });
    }
}
