use egui::Context;

use crate::dimensions::Dimensions;
use crate::TemplateApp;

impl TemplateApp {
    pub fn cell_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Cells");

        // horizontal layout
        ui.horizontal(|ui| {
            if ui.button("Reset").clicked() {
                self.reload_background(ctx, None, true, true);
            }
            if ui.button("Paint selected").clicked() {
                self.paint_cell(ctx);
            }
        });

        ui.separator();

        // search bar
        ui.horizontal(|ui| {
            ui.label("Filter: ");
            ui.text_edit_singleline(&mut self.runtime_data.cell_filter);
            // clear filter button
            if ui.button("x").clicked() {
                self.runtime_data.plugin_filter.clear();
            }
        });

        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let mut ids = self.cell_records.keys().collect::<Vec<_>>();
                ids.sort();

                for key in ids {
                    // upper and lowercase search
                    let cell_name = self.cell_records.get(key).unwrap().name.clone();
                    let label_text = format!("{:?} - {}", key, cell_name);
                    if !self.runtime_data.cell_filter.is_empty()
                        && !label_text
                            .to_lowercase()
                            .contains(&self.runtime_data.cell_filter.to_lowercase())
                    {
                        continue;
                    }

                    if let Some(selected_key) = self.runtime_data.selected_id {
                        if selected_key == *key {
                            ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
                        } else {
                            ui.visuals_mut().override_text_color = None;
                        }
                    }

                    let label = egui::Label::new(label_text).sense(egui::Sense::click());
                    let response = ui.add(label);
                    if response.clicked() {
                        self.runtime_data.selected_id = Some(*key);
                    }
                }
            });
    }

    pub fn paint_cell(&mut self, ctx: &Context) {
        if let Some(cell_key) = self.runtime_data.selected_id {
            let x = cell_key.0;
            let y = cell_key.1;

            let mut dimensions = Dimensions {
                min_x: x,
                min_y: y,
                max_x: x,
                max_y: y,
                min_z: 0.0,
                max_z: 0.0,
                texture_size: 32,
            };

            let max_texture_side = ctx.input(|i| i.max_texture_side);
            let max_texture_resolution =
                dimensions.get_max_texture_resolution(max_texture_side);
            dimensions.texture_size = max_texture_resolution;
            
            self.reload_background(ctx, Some(dimensions), true, true);
        }
    }
}
