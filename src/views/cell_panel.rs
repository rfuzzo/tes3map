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

                    if self.runtime_data.selected_ids.contains(key) {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
                    } else {
                        ui.visuals_mut().override_text_color = None;
                    }

                    let label = egui::Label::new(label_text).sense(egui::Sense::click());
                    let response = ui.add(label);

                    // if cell is clicked, select only that cell
                    if response.clicked() {
                        self.runtime_data.selected_ids = vec![*key];
                    }
                }
            });
    }

    pub fn paint_cell(&mut self, ctx: &Context) {
        // get min and max dimensions of selected cells
        let selected_cell_ids = self.runtime_data.selected_ids.clone();

        // get min and max dimensions of selected cells
        let min_x = selected_cell_ids.iter().map(|k| k.0).min().unwrap();
        let min_y = selected_cell_ids.iter().map(|k| k.1).min().unwrap();
        let max_x = selected_cell_ids.iter().map(|k| k.0).max().unwrap();
        let max_y = selected_cell_ids.iter().map(|k| k.1).max().unwrap();

        let dimensions = Dimensions {
            min_x,
            min_y,
            max_x,
            max_y,
            min_z: 0.0,
            max_z: 0.0,
        };

        let max_texture_side = ctx.input(|i| i.max_texture_side);
        let max_texture_resolution = dimensions.get_max_texture_resolution(max_texture_side);
        self.ui_data.landscape_settings.texture_size = max_texture_resolution;

        self.reload_background(ctx, Some(dimensions), true, true);
    }
}
