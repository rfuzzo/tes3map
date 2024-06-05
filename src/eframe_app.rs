use std::env;

use crate::*;
use crate::app::ESidePanelView;

impl eframe::App for TemplateApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // on start, we check the current folder for esps
        if self.data_files.is_none() {
            if let Ok(cwd) = env::current_dir() {
                // load once
                self.data_files = Some(cwd.clone());
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui
                        .hyperlink_to("Github repo", "https://github.com/rfuzzo/tes3map")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });

                ui.add_space(16.0);

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        // right panel
        egui::SidePanel::right("cell_panel").show(ctx, |ui| {
            // tab bar
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.side_panel_view,
                    ESidePanelView::Plugins,
                    "Plugins",
                );
                ui.selectable_value(&mut self.side_panel_view, ESidePanelView::Cells, "Cells");
            });

            match self.side_panel_view {
                // view
                app::ESidePanelView::Plugins => self.plugins_panel(ui, ctx),
                app::ESidePanelView::Cells => self.cell_panel(ui, ctx),
            }
        });

        // footer
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // Status Bar
            ui.horizontal(|ui| {
                // map bounds
                ui.label(format!(
                    "({},{}) - ({},{})",
                    self.dimensions.min_x,
                    self.dimensions.min_y,
                    self.dimensions.max_x,
                    self.dimensions.max_y
                ));
                ui.separator();
                ui.label(get_cell_name(&self.cell_records, self.hover_pos));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.map_panel(ui, ctx);
        });
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
