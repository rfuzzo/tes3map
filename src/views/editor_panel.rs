use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};

use crate::fs::write;
use egui::{ahash::HashMap, Context};

use crate::TemplateApp;

/// This is the editor panel for the map editor
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct EditorData {
    pub enabled: bool,
    pub mod_folder: PathBuf,

    // display options

    // runtime data
    #[serde(skip)]
    pub routes: Vec<Route>,
    #[serde(skip)]
    pub segments: HashMap<String, Segment>,
    #[serde(skip)]
    pub ports: HashMap<String, Port>,

    #[serde(skip)]
    pub selected_point: Option<(String, usize)>,
    #[serde(skip)]
    pub current_segment: Option<String>,
}

impl EditorData {
    // get folders
    pub fn routes_folder(&self) -> PathBuf {
        self.mod_folder.join("routes")
    }

    pub fn segments_folder(&self) -> PathBuf {
        self.mod_folder.join("segments")
    }

    pub fn ports_folder(&self) -> PathBuf {
        self.mod_folder.join("ports")
    }
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

    // runtime data
    #[serde(skip)]
    pub selected: bool,
}

// port struct
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Port {
    pub data: HashMap<String, PortData>,
}

#[allow(non_snake_case)]
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct PortData {
    pub position: Pos3,
    pub rotation: Pos3,
    pub positionEnd: Option<Pos3>,
    pub rotationEnd: Option<Pos3>,
    pub positionStart: Option<Pos3>,
    pub rotationStart: Option<Pos3>,
    pub reverseStart: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Pos3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// implement display for Pos3
impl std::fmt::Display for Pos3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
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
            if ui.button("Open mod folder").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&self.editor_data.mod_folder)
                    .pick_folder()
                {
                    self.editor_data.mod_folder = path;
                }
            }
            ui.label(format!("{}", self.editor_data.mod_folder.display()));
        });

        // return if folders are not set
        if !self.editor_data.mod_folder.exists() || !self.editor_data.mod_folder.exists() {
            ui.label("Please set the routes and segments folders.");
            return;
        }

        ui.separator();

        // load button
        if ui.button("Load routes").clicked() {
            self.editor_data.current_segment = None;

            // load routes from folder
            self.editor_data.routes.clear();
            if let Ok(entries) = read_dir(self.editor_data.routes_folder()) {
                for entry in entries.flatten() {
                    if let Ok(file) = read_to_string(entry.path()) {
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
            if let Ok(entries) = read_dir(self.editor_data.segments_folder()) {
                for entry in entries.flatten() {
                    if let Ok(file) = read_to_string(entry.path()) {
                        // parse the file as a segment
                        if let Ok(segment) = toml::de::from_str::<Segment>(&file) {
                            self.editor_data
                                .segments
                                .insert(segment.id.clone(), segment);
                        } else {
                            log::error!("Failed to parse segment file: {}", entry.path().display());
                        }
                    }
                }
            }

            // load ports from folder
            self.editor_data.ports.clear();
            if let Ok(entries) = read_dir(self.editor_data.ports_folder()) {
                for entry in entries.flatten() {
                    if let Ok(file) = read_to_string(entry.path()) {
                        // parse the file as a segment
                        match toml::de::from_str::<Port>(&file) {
                            Ok(port) => {
                                let name = entry
                                    .file_name()
                                    .to_str()
                                    .unwrap()
                                    .replace(".toml", "")
                                    .to_string();
                                self.editor_data.ports.insert(name, port);
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to parse segment file '{}': {}",
                                    entry.path().display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        ui.horizontal(|ui| {
            // save button
            ui.horizontal(|ui| {
                if ui.button("Save routes").clicked() {
                    // save routes to folder
                    for route in &self.editor_data.routes {
                        let file = toml::ser::to_string_pretty(route).unwrap();
                        let path = self
                            .editor_data
                            .routes_folder()
                            .join(format!("{}_{}.toml", route.id.start, route.id.destination));
                        write(path, file).unwrap();
                    }
                }

                if ui.button("Save ports").clicked() {
                    // save routes to folder
                    for (name, port) in &self.editor_data.ports {
                        let file = toml::ser::to_string_pretty(port).unwrap();
                        let path = self
                            .editor_data
                            .ports_folder()
                            .join(format!("{}.toml", name));
                        write(path, file).unwrap();
                    }
                }

                if ui.button("Save segments").clicked() {
                    // save segments to folder
                    for (_, segment) in self.editor_data.segments.iter().filter(|(_, s)| s.selected)
                    {
                        // round route1 points to 0 decimal places
                        let mut segment = segment.clone();
                        if let Some(ref mut route1) = segment.route1 {
                            for point in route1.iter_mut() {
                                point.x = (point.x).round();
                                point.y = (point.y).round();
                                point.z = (point.z).round();
                            }
                        }

                        let file = toml::ser::to_string_pretty(&segment).unwrap();
                        let path = self
                            .editor_data
                            .segments_folder()
                            .join(format!("{}.toml", segment.id));
                        write(path, file).unwrap();
                    }
                }
            });
        });

        ui.separator();

        // display options
        ui.label("Display options:");

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            // Routes list
            ui.collapsing("Routes", |ui| {
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

            // Segments list
            ui.collapsing("Segments", |ui| {
                // select all
                // horizontal layout for select all and deselect all buttons
                ui.horizontal(|ui| {
                    if ui.button("Select all").clicked() {
                        self.editor_data.current_segment = None;
                        for segment in self.editor_data.segments.values_mut() {
                            segment.selected = true;
                        }
                    }
                    if ui.button("Deselect all").clicked() {
                        self.editor_data.current_segment = None;
                        for segment in self.editor_data.segments.values_mut() {
                            segment.selected = false;
                        }
                    }
                });

                // get sorted keys
                let mut keys: Vec<String> = self.editor_data.segments.keys().cloned().collect();
                keys.sort();

                for id in keys {
                    let segment = self.editor_data.segments.get_mut(&id).unwrap();

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut segment.selected, "Select");

                        // if is current selected segment, highlight it
                        if self.editor_data.current_segment == Some(id.clone()) {
                            ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
                        } else {
                            ui.visuals_mut().override_text_color = None;
                        }

                        if ui.button(id.clone()).clicked() {
                            // set the current segment to the selected one otherwise set to None
                            if self.editor_data.current_segment.is_none() {
                                self.editor_data.current_segment = Some(id.clone());
                            } else {
                                self.editor_data.current_segment = None;
                            }
                        }
                    });
                }
            });

            // Ports list
            ui.collapsing("Ports", |ui| {
                let mut keys: Vec<String> = self.editor_data.ports.keys().cloned().collect();
                keys.sort();

                for id in keys {
                    //let port = self.editor_data.ports.get(&id).unwrap();

                    ui.horizontal(|ui| {
                        ui.label(id.clone());
                        //ui.label(format!("{:?}", port.data.position));
                    });
                }
            });
        });
    }
}
