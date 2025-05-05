use std::path::PathBuf;

use egui::{ahash::HashMap, Context};

use crate::TemplateApp;

/// This is the editor panel for the map editor
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
    pub segments: HashMap<String, Segment>,
    #[serde(skip)]
    pub selected_point: Option<(String, usize)>,
    #[serde(skip)]
    pub selected_segment: Option<String>,
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

impl Pos3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
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
        ui.horizontal(|ui| {
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
                                log::error!(
                                    "Failed to parse route file: {}",
                                    entry.path().display()
                                );
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
                                self.editor_data
                                    .segments
                                    .insert(segment.id.clone(), segment);
                            } else {
                                log::error!(
                                    "Failed to parse segment file: {}",
                                    entry.path().display()
                                );
                            }
                        }
                    }
                }
            }

            // save button
            if ui.button("Save routes").clicked() {
                // TODO rounding

                // save routes to folder
                for route in &self.editor_data.routes {
                    let file = toml::ser::to_string_pretty(route).unwrap();
                    let path = self
                        .editor_data
                        .routes_folder
                        .join(format!("{}_{}.toml", route.id.start, route.id.destination));
                    std::fs::write(path, file).unwrap();
                }

                // save segments to folder
                for segment in self.editor_data.segments.values() {
                    let file = toml::ser::to_string_pretty(segment).unwrap();
                    let path = self
                        .editor_data
                        .segments_folder
                        .join(format!("{}.toml", segment.id));
                    std::fs::write(path, file).unwrap();
                }
            }
        });

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
        // select all
        // horizontal layout for select all and deselect all buttons
        ui.horizontal(|ui| {
            if ui.button("Select all").clicked() {
                for segment in self.editor_data.segments.values_mut() {
                    segment.selected = true;
                }
            }
            if ui.button("Deselect all").clicked() {
                for segment in self.editor_data.segments.values_mut() {
                    segment.selected = false;
                }
            }
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, segment) in self.editor_data.segments.iter_mut() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut segment.selected, "Select");

                    // if is current selected segment, highlight it
                    if self.editor_data.selected_segment == Some(id.clone()) {
                        ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
                    } else {
                        ui.visuals_mut().override_text_color = None;
                    }

                    if ui.button(format!("Edit {}", id)).clicked() {
                        // select the segment
                        self.editor_data.selected_segment = Some(id.clone());
                    }
                });
            }
        });
    }
}
