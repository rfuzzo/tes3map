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

                if clicked && self.background_handle.is_some() {
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
        if self.background_handle.is_some() {
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

                if ui.button("Refresh").clicked() {
                    self.reload_background(ctx, None, false, false);

                    if self.ui_data.overlay_paths {
                        self.reload_paths(ctx);
                    }
                }
            });
        }

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

    fn landscape_settings_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        ui.label("Landscape settings");

        ui.checkbox(&mut self.ui_data.realtime_update, "Realtime update");

        let max_texture_side = ctx.input(|i| i.max_texture_side);
        let max_texture_resolution = self.dimensions.get_max_texture_resolution(max_texture_side);

        ui.add(
            egui::Slider::new(
                &mut self.ui_data.landscape_settings.texture_size,
                2..=max_texture_resolution,
            )
            .text("Texture Resolution"),
        );

        if ui
            .checkbox(
                &mut self.ui_data.landscape_settings.show_water,
                "Render water",
            )
            .changed()
        {
            self.reload_background(ctx, None, false, false);
        }
        if ui
            .checkbox(
                &mut self.ui_data.landscape_settings.remove_water,
                "Clip water",
            )
            .changed()
        {
            self.reload_background(ctx, None, false, false);
        }
    }

    fn heightmap_settings_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let settings = &mut self.ui_data.heightmap_settings;
        ui.label("Heightmap settings");

        ui.checkbox(&mut self.ui_data.realtime_update, "Realtime update");

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

        if changed && self.ui_data.realtime_update {
            // reload background
            self.reload_background(ctx, None, false, false);
        }
    }
}
