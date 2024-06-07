use crate::TemplateApp;

impl TemplateApp {
    pub fn cell_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Cells");
        if ui.button("Paint all").clicked() {
            self.reload_background(ctx, None, true, true);
        }

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
}
