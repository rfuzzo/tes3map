use std::path::PathBuf;

use egui::Context;

use crate::TemplateApp;

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct EditorData {
    pub enabled: bool,
    pub routes_folder: PathBuf,
    pub segments_folder: PathBuf,

    // display options

    // runtime data
    #[serde(skip)]
    pub routes: Vec<Route>,
    #[serde(skip)]
    pub segments: Vec<Segment>,
}

// route struct
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Route {
    pub id: RouteId,
    pub segments: Vec<String>,
}

// route id struct
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct RouteId {
    pub start: String,
    pub destination: String,
    pub service: String,
}

// segment struct
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Segment {
    pub id: String,
    pub route1: Option<Vec<Pos3>>,
    pub route2: Option<Vec<Pos3>>,
    pub segments: Option<Vec<Segment>>,

    // runtime data
    #[serde(skip)]
    pub selected: bool,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Pos3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl TemplateApp {
    pub fn editor_panel(&mut self, ui: &mut egui::Ui, _ctx: &Context) {
        ui.heading("Editor");

        // enabled/disabled
        ui.checkbox(&mut self.editor_data.enabled, "Editor enabled");
        ui.separator();

        // This is an editor panel to draw routes on the map
        // A route consists of a list of segments and an id (startName, destinationName, serviceName)
        // A segment has an id, can have two splines, or a list of segments
        // A spline is a list of points (x, y)

        // The editor panel should have a list of routes, loaded from a folder
        // Each route is serialized in a toml file

        // folder control
        ui.horizontal(|ui| {
            if ui.button("Open routes folder").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&self.editor_data.routes_folder)
                    .pick_folder()
                {
                    self.editor_data.routes_folder = path;
                }
            }
            ui.label(format!("{}", self.editor_data.routes_folder.display()));
        });

        // segments folder control
        ui.horizontal(|ui| {
            if ui.button("Open segments folder").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&self.editor_data.segments_folder)
                    .pick_folder()
                {
                    self.editor_data.segments_folder = path;
                }
            }
            ui.label(format!("{}", self.editor_data.segments_folder.display()));
        });

        // return if folders are not set
        if !self.editor_data.routes_folder.exists() || !self.editor_data.segments_folder.exists() {
            ui.label("Please set the routes and segments folders.");
            return;
        }

        // load button
        if ui.button("Load routes").clicked() {
            // load routes from folder
            self.editor_data.routes.clear();
            if let Ok(entries) = std::fs::read_dir(&self.editor_data.routes_folder) {
                for entry in entries.flatten() {
                    if let Ok(file) = std::fs::read_to_string(entry.path()) {
                        // parse the file as a route
                        if let Ok(route) = toml::de::from_str::<Route>(&file) {
                            self.editor_data.routes.push(route);
                        } else {
                            log::error!("Failed to parse route file: {}", entry.path().display());
                        }
                    }
                }
            }

            // load segments from folder
            self.editor_data.segments.clear();
            if let Ok(entries) = std::fs::read_dir(&self.editor_data.segments_folder) {
                for entry in entries.flatten() {
                    if let Ok(file) = std::fs::read_to_string(entry.path()) {
                        // parse the file as a segment
                        if let Ok(segment) = toml::de::from_str::<Segment>(&file) {
                            self.editor_data.segments.push(segment);
                        } else {
                            log::error!("Failed to parse segment file: {}", entry.path().display());
                        }
                    }
                }
            }
        }

        ui.separator();

        // display options
        ui.label("Display options:");

        ui.separator();

        // routes list
        ui.label("Routes:");
        egui::ScrollArea::vertical().show(ui, |ui| {
            for route in &self.editor_data.routes {
                ui.horizontal(|ui| {
                    let name = format!(
                        "{} -> {} ({})",
                        route.id.start, route.id.destination, route.id.service
                    );
                    ui.collapsing(name, |ui| {
                        for segment in &route.segments {
                            ui.label(segment);
                        }
                    });
                });
            }
        });

        // segments list
        ui.label("Segments:");
        // TODO select all
        egui::ScrollArea::vertical().show(ui, |ui| {
            for segment in self.editor_data.segments.iter_mut() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut segment.selected, "Select");
                    ui.label(segment.id.clone());
                });
            }
        });
    }
}
