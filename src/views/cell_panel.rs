use crate::{calculate_dimensions, Dimensions, TemplateApp, TEXTURE_MAX_SIZE};

impl TemplateApp {
    pub fn cell_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Cells");
        if ui.button("Paint all").clicked() {
            // TODO reload and paint all
        }

        ui.separator();

        // // TODO search bar
        // ui.horizontal(|ui| {
        //     ui.label("Filter: ");
        //     ui.text_edit_singleline(&mut self.cell_filter);
        // });
        // ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if let Some(d) = calculate_dimensions(&self.land_records, self.ui_data.texture_size)
                {
                    for y in (d.min_y..=d.max_y).rev() {
                        let mut any = false;
                        for x in d.min_x..=d.max_x {
                            if let Some(_v) = self.land_records.get(&(x, y)) {
                                any = true;
                            }
                        }
                        if any {
                            ui.collapsing(format!("Y: {y}"), |ui| {
                                for x in d.min_x..=d.max_x {
                                    if let Some(landscape) = self.land_records.get(&(x, y)) {
                                        if ui.button(format!("({x},{y})")).clicked() {
                                            // store
                                            self.current_landscape = Some(landscape.clone());

                                            let dimensions = Dimensions {
                                                min_x: x,
                                                min_y: y,
                                                max_x: x,
                                                max_y: y,
                                                texture_size: TEXTURE_MAX_SIZE,
                                            };
                                            self.reload_background(ctx, Some(dimensions));
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            });
    }
}
