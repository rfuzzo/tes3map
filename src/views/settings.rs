use egui::reset_button;

use crate::{EBackground, TemplateApp};

impl TemplateApp {
    /// Settings popup menu
    pub(crate) fn settings_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            reset_button(ui, &mut self.ui_data);

            if ui.button("Refresh image").clicked() {
                self.reload_background(ctx, None);
            }
        });

        ui.separator();

        ui.label("Background");
        egui::ComboBox::from_id_source("background")
            .selected_text(format!("{:?}", self.ui_data.background))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.ui_data.background, EBackground::None, "None");
                let mut clicked = false;
                if ui
                    .selectable_value(
                        &mut self.ui_data.background,
                        EBackground::GameMap,
                        "Game map",
                    )
                    .clicked()
                {
                    clicked = true;
                }
                if ui
                    .selectable_value(
                        &mut self.ui_data.background,
                        EBackground::HeightMap,
                        "Heightmap",
                    )
                    .clicked()
                {
                    clicked = true;
                }
                if ui
                    .selectable_value(
                        &mut self.ui_data.background,
                        EBackground::Landscape,
                        "Landscape",
                    )
                    .clicked()
                {
                    clicked = true;
                }

                if clicked {
                    self.reload_background(ctx, None);
                }
            });

        ui.separator();

        ui.label("Overlays");
        ui.checkbox(&mut self.ui_data.overlay_paths, "Show paths");
        ui.checkbox(&mut self.ui_data.overlay_region, "Show regions");
        ui.checkbox(&mut self.ui_data.overlay_grid, "Show cell grid");
        ui.checkbox(&mut self.ui_data.overlay_cities, "Show cities");
        ui.checkbox(&mut self.ui_data.overlay_travel, "Show travel");

        ui.separator();
        ui.checkbox(&mut self.ui_data.show_tooltips, "Show tooltips");

        ui.label("Color");
        ui.add(egui::Slider::new(&mut self.ui_data.alpha, 0..=255).text("Alpha"));

        ui.color_edit_button_srgba(&mut self.ui_data.height_base);
        ui.add(
            egui::Slider::new(&mut self.ui_data.height_spectrum, -360..=360).text("Height offset"),
        );

        ui.color_edit_button_srgba(&mut self.ui_data.depth_base);
        ui.add(
            egui::Slider::new(&mut self.ui_data.depth_spectrum, -360..=360).text("Depth offset"),
        );

        ui.separator();

        ui.add(
            egui::Slider::new(&mut self.ui_data.texture_size, 2..=200).text("Texture Resolution"),
        );

        ui.separator();

        ui.label("zoom with Ctrl + Mousewheel");
        ui.label("reset with middle mouse button");
    }
}
