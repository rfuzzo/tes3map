use crate::TemplateApp;

impl TemplateApp {
    pub fn cell_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Cells");
        if ui.button("Paint all").clicked() {
            self.reload_background(ctx, None);
        }

        ui.separator();

        // TODO search bar
        ui.horizontal(|ui| {
            ui.label("Filter: ");
            ui.text_edit_singleline(&mut self.cell_filter);
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let mut ids = self.cell_records.keys().collect::<Vec<_>>();
                ids.sort();

                for key in ids {
                    // TODO upper and lowercase search
                    let key_str = format!("{:?}", key);
                    if !self.cell_filter.is_empty()
                        && !key_str
                            .to_lowercase()
                            .contains(&self.cell_filter.to_lowercase())
                    {
                        continue;
                    }

                    if let Some(selected_key) = self.selected_id {
                        if selected_key == *key {
                            ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
                        } else {
                            ui.visuals_mut().override_text_color = None;
                        }
                    }

                    let response = ui.add(egui::Label::new(key_str).sense(egui::Sense::click()));
                    if response.clicked() {
                        self.selected_id = Some(*key);
                    }
                }
            });
    }
}
