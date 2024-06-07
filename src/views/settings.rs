use egui::Ui;

use crate::{EBackground, HeightmapSettings, LandscapeSettings, TemplateApp};

impl TemplateApp {
    /// Settings popup menu
    pub(crate) fn settings_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.label("Background");
        egui::ComboBox::from_id_source("background")
            .selected_text(format!("{:?}", self.ui_data.background))
            .show_ui(ui, |ui| {
                let mut clicked = false;
                if ui
                    .selectable_value(&mut self.ui_data.background, EBackground::None, "None")
                    .clicked()
                {
                    clicked = true;
                }

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
                    self.reload_background(ctx, None, false, false);
                }
            });

        ui.separator();

        ui.label("Overlays");
        if ui
            .checkbox(&mut self.ui_data.overlay_paths, "Show paths")
            .clicked()
        {
            self.reload_paths(ctx);
        }
        ui.checkbox(&mut self.ui_data.overlay_region, "Show regions");
        ui.checkbox(&mut self.ui_data.overlay_grid, "Show cell grid");
        ui.checkbox(&mut self.ui_data.overlay_cities, "Show cities");
        ui.checkbox(&mut self.ui_data.overlay_travel, "Show travel");
        ui.checkbox(&mut self.ui_data.overlay_conflicts, "Show conflicts");

        ui.checkbox(&mut self.ui_data.show_tooltips, "Show tooltips");

        // settings
        ui.separator();
        ui.horizontal(|ui| {
            // if reset then also refresh
            if ui.button("Reset").clicked() {
                // background settings
                if self.ui_data.background == EBackground::Landscape {
                    self.ui_data.landscape_settings = LandscapeSettings::default();
                } else if self.ui_data.background == EBackground::HeightMap {
                    self.ui_data.heightmap_settings = HeightmapSettings::default();
                }

                self.reload_background(ctx, None, false, false);
            }

            // if ui.button("Refresh image").clicked() {
            //     self.reload_background(ctx, None, false, false);
            //
            //     if self.ui_data.overlay_paths {
            //         self.reload_paths(ctx, true);
            //     }
            // }
        });

        // background settings
        if self.ui_data.background == EBackground::Landscape {
            ui.separator();
            self.landscape_settings_ui(ui, ctx);
        } else if self.ui_data.background == EBackground::HeightMap {
            ui.separator();
            self.heightmap_settings_ui(ui, ctx);
        }

        // overlay settings

        ui.separator();

        ui.label("Zoom: Ctrl + Mousewheel");
        ui.label("Reset: middle mouse button");
    }

    fn landscape_settings_ui(&mut self, ui: &mut Ui, _ctx: &egui::Context) {
        let settings = &mut self.ui_data.landscape_settings;
        ui.label("Landscape settings");
        ui.add(egui::Slider::new(&mut settings.texture_size, 2..=200).text("Texture Resolution"));
    }

    fn heightmap_settings_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let settings = &mut self.ui_data.heightmap_settings;
        ui.label("Heightmap settings");
        let mut changed = false;
        if ui
            .color_edit_button_srgba(&mut settings.height_base)
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::Slider::new(&mut settings.height_spectrum, -360..=360).text("Height offset"))
            .changed()
        {
            changed = true;
        }

        if ui
            .color_edit_button_srgba(&mut settings.depth_base)
            .changed()
        {
            changed = true;
        }
        if ui
            .add(egui::Slider::new(&mut settings.depth_spectrum, -360..=360).text("Depth offset"))
            .changed()
        {
            changed = true;
        }

        if changed {
            // reload background
            self.reload_background(ctx, None, false, false);
        }
    }
}
