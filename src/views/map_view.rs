use eframe::emath::{pos2, Pos2, Rect, RectTransform};
use eframe::epaint::{Color32, Shape, Stroke};
use egui::{CornerRadius, Sense, StrokeKind};
use log::info;

use crate::app::TooltipInfo;
use crate::overlay::cities::get_cities_shapes;
use crate::overlay::conflicts::get_conflict_shapes;
use crate::overlay::grid::get_grid_shapes;
use crate::overlay::regions::get_region_shapes;
use crate::overlay::travel::get_travel_shapes;
use crate::*;

impl TemplateApp {
    fn cellkey_from_screen(&mut self, from_screen: RectTransform, pointer_pos: Pos2) -> CellKey {
        let transformed_position = from_screen * pointer_pos;
        // get cell grid
        self.dimensions
            .canvas_to_cell(Pos2::new(transformed_position.x, transformed_position.y))
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

                if let Some(pointer_pos) = response.hover_pos() {
                    let d = pointer_pos * r;
                    self.zoom_data.drag_offset -= d.to_vec2();
                }
            }
        }

        // transforms
        let real_width = self.dimensions.width() as f32;
        let real_height = self.dimensions.height() as f32;
        let from: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(real_width, real_height));
        let r = real_height / real_width;

        let min = self.zoom_data.drag_offset;
        let max = Pos2::new(response.rect.max.x, response.rect.max.x * r) * self.zoom_data.zoom
            + self.zoom_data.drag_offset.to_vec2();
        let canvas = Rect::from_min_max(min, max);

        let to_screen = RectTransform::from_to(from, canvas);
        let from_screen = to_screen.inverse();

        // paint maps

        let uv = Rect::from_min_max(pos2(0.0, 0.0), Pos2::new(1.0, 1.0));

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
        for class in self.travel_edges.keys() {
            if let Some(class_option) = self.ui_data.overlay_travel.get(class) {
                if *class_option {
                    let shapes =
                        get_travel_shapes(to_screen, &self.dimensions, &self.travel_edges, class);
                    painter.extend(shapes);
                }
            }
        }
        if self.ui_data.overlay_conflicts {
            let shapes = get_conflict_shapes(to_screen, &self.dimensions, &self.cell_conflicts);
            painter.extend(shapes);
        }
        // routes
        if self.editor_data.enabled {
            let shapes = overlay::mod_splines::get_segments_shapes(
                to_screen,
                &self.dimensions,
                &self.editor_data,
                &response.hover_pos(),
            );
            painter.extend(shapes);
        }

        // overlay selected cell
        for key in &self.runtime_data.selected_ids {
            let rect = get_rect_at_cell(&self.dimensions, to_screen, *key);
            let shape = Shape::rect_stroke(
                rect,
                CornerRadius::default(),
                Stroke::new(4.0, Color32::RED),
                StrokeKind::Outside,
            );
            painter.add(shape);
        }
        if let Some(pivot_id) = self.runtime_data.pivot_id {
            let rect = get_rect_at_cell(&self.dimensions, to_screen, pivot_id);
            let shape = Shape::rect_stroke(
                rect,
                CornerRadius::default(),
                Stroke::new(4.0, Color32::LIGHT_BLUE),
                StrokeKind::Outside,
            );
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
                debug: String::new(),
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
                tooltipinfo.debug = format!("{:?}", transformed_position);

                let x = transformed_position.x * VERTEX_CNT as f32;
                let y = transformed_position.y * VERTEX_CNT as f32;

                if let Some(height) = height_from_screen_space(
                    &self.heights,
                    &self.dimensions,
                    x as usize,
                    y as usize,
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

            if self.ui_data.show_tooltips && ui.ui_contains_pointer() {
                egui::show_tooltip(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new("hover_tooltip"),
                    |ui| {
                        ui.set_width(200.0);

                        let info = self.runtime_data.info.clone();
                        ui.label(format!("{:?} - {}", info.key, info.cell_name));
                        ui.label(format!("Region: {}", info.region));

                        // only show if current background is heightmap
                        if self.ui_data.background == EBackground::HeightMap {
                            ui.label("________");
                            ui.label(format!("Height: {}", info.height));
                        }

                        // show conflicts
                        if self.ui_data.overlay_conflicts && !info.conflicts.is_empty() {
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

                        // show travel info
                    },
                );
            }
        }

        // panning
        if !ui.ctx().input(|i| i.modifiers.ctrl) {
            self.editor_data.selected_point = None;
        }

        if response.drag_started() {
            if let Some(drag_start) = response.interact_pointer_pos() {
                if ui.ctx().input(|i| i.modifiers.ctrl) {
                    self.on_ctrl_drag_started(from_screen, drag_start);
                } else {
                    self.zoom_data.drag_start = drag_start;
                }
            }
        } else if response.dragged() {
            if let Some(current_pos) = response.interact_pointer_pos() {
                if ui.ctx().input(|i| i.modifiers.ctrl) {
                    self.on_point_dragged(from_screen, current_pos);
                } else {
                    let delta = current_pos - self.zoom_data.drag_start.to_vec2();
                    self.zoom_data.drag_delta = Some(delta);
                    self.zoom_data.drag_start = current_pos;
                }
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
        let settings_rect = Rect::from_pos(pos2(10.0, 50.0));
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

            if ui.button("Paint selected cells").clicked() {
                self.paint_cells(ctx);
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Save as image").clicked() {
                match self.save_image(ctx) {
                    Ok(_) => {}
                    Err(e) => {
                        info!("Error saving image: {:?}", e);
                    }
                }

                ui.close_menu();
            }
        });

        // click
        if let Some(interact_pos) = painter.ctx().pointer_interact_pos() {
            if ui.ctx().input(|i| i.pointer.primary_clicked()) {
                // check if ctrl is pressed
                if ui.ctx().input(|i| i.modifiers.ctrl) {
                    self.on_ctrl_clicked(from_screen, interact_pos);
                } else {
                    self.on_click(ui, from_screen, interact_pos);
                }
            }
        }
    }

    fn on_point_dragged(&mut self, from_screen: RectTransform, current_pos: Pos2) {
        // move the selected point if in editor mode
        if self.editor_data.enabled {
            if let Some((s, i)) = &self.editor_data.selected_point {
                // get the segment
                if let Some(segment) = self.editor_data.segments.get_mut(s) {
                    // get the route
                    if let Some(route1) = &mut segment.route1 {
                        // get the point
                        if let Some(point) = route1.get_mut(*i) {
                            // tranlate the point to screen space
                            let clicked_point = from_screen * current_pos;

                            let engine_pos = self
                                .dimensions
                                .canvas_to_engine(Pos2::new(clicked_point.x, clicked_point.y));

                            // move the point
                            point.x = engine_pos.x;
                            point.y = engine_pos.y;
                        }
                    }
                }
            }
        }
    }

    fn on_ctrl_clicked(&mut self, from_screen: RectTransform, interact_pos: Pos2) {
        // if ctr is pressed and editor mode is enabled
        if self.editor_data.enabled {
            // try to get a point from the segments if distace is less than 10 pixels
            let mut found = false;
            let mut found_point = None;

            // check if within distance of any point
            for (id, segment) in self.editor_data.segments.iter().filter(|(_, s)| s.selected) {
                if let Some(route1) = &segment.route1 {
                    for (i, point) in route1.iter().enumerate() {
                        let clicked_point = from_screen * interact_pos;
                        let canvas_pos = self
                            .dimensions
                            .engine_to_canvas(Pos2::new(point.x, point.y));

                        let dist = (clicked_point - canvas_pos).length();
                        if dist < 0.1 {
                            found = true;
                            found_point = Some((id.clone(), i));
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }

            // if found, remove the point from the segment
            if let Some((s, i)) = found_point {
                // get the segment
                if let Some(segment) = self.editor_data.segments.get_mut(&s) {
                    // get the route
                    if let Some(route1) = &mut segment.route1 {
                        // remove the point
                        route1.remove(i);
                    }
                }
            }
        }
    }

    fn on_ctrl_drag_started(&mut self, from_screen: RectTransform, interact_pos: Pos2) {
        // if ctr is pressed and editor mode is enabled
        if self.editor_data.enabled {
            // try to get a point from the segments if distace is less than 10 pixels
            let mut found = false;

            // check if within distance of any point
            for (id, segment) in self.editor_data.segments.iter().filter(|(_, s)| s.selected) {
                if let Some(route1) = &segment.route1 {
                    for (i, point) in route1.iter().enumerate() {
                        let clicked_point = from_screen * interact_pos;
                        let canvas_pos = self
                            .dimensions
                            .engine_to_canvas(Pos2::new(point.x, point.y));

                        let dist = (clicked_point - canvas_pos).length();
                        if dist < 0.1 {
                            found = true;
                            self.editor_data.selected_point = Some((id.clone(), i));
                            break;
                        }
                    }
                }
                if found {
                    break;
                }
            }
        }
    }

    fn on_click(&mut self, ui: &mut egui::Ui, from_screen: RectTransform, interact_pos: Pos2) {
        // if in the cell panel, we select the cell
        let key = self.cellkey_from_screen(from_screen, interact_pos);

        // check if withing dimensions
        let inside = key.0 >= self.dimensions.min_x
            && key.0 <= self.dimensions.max_x
            && key.1 >= self.dimensions.min_y
            && key.1 <= self.dimensions.max_y;

        if inside {
            // toggle selection
            if ui.ctx().input(|i| i.modifiers.ctrl) {
                // toggle and add to selection
                self.runtime_data.pivot_id = None;
                if self.runtime_data.selected_ids.contains(&key) {
                    self.runtime_data.selected_ids.retain(|&x| x != key);
                } else {
                    self.runtime_data.selected_ids.push(key);
                }
            } else if ui.ctx().input(|i| i.modifiers.shift) {
                // shift selects all cells between the last selected cell and the current cell
                if !self.runtime_data.selected_ids.is_empty() {
                    // x check. check if

                    let start = if self.runtime_data.selected_ids.len() == 1 {
                        self.runtime_data.selected_ids[0]
                    } else if self.runtime_data.pivot_id.is_some() {
                        self.runtime_data.pivot_id.unwrap()
                    } else {
                        *self.runtime_data.selected_ids.last().unwrap()
                    };
                    self.runtime_data.pivot_id = Some(start);
                    let end = key;

                    // add all keys between start and end

                    let min_x = start.0.min(end.0);
                    let max_x = start.0.max(end.0);

                    let min_y = start.1.min(end.1);
                    let max_y = start.1.max(end.1);

                    let mut keys = Vec::<CellKey>::new();

                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            keys.push((x, y));
                        }
                    }

                    self.runtime_data.selected_ids = keys;
                } else {
                    self.runtime_data.selected_ids = vec![key];
                    self.runtime_data.pivot_id = Some(key);
                }
            } else {
                #[allow(clippy::collapsible_else_if)]
                if self.runtime_data.selected_ids.contains(&key) {
                    // toggle off if the same cell is clicked
                    self.runtime_data.selected_ids = Vec::new();
                    self.runtime_data.pivot_id = None;
                } else {
                    self.runtime_data.selected_ids = vec![key];
                    self.runtime_data.pivot_id = Some(key);
                }
            }
        }
    }
}
