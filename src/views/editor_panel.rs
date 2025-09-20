use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};

use crate::fs::write;
use egui::{ahash::HashMap, emath::RectTransform, Context, Pos2, Response};

use crate::TemplateApp;

#[derive(Debug, Clone, Default)]
pub struct RouteMetadata {
    pub valid: bool,
}

/// This is the editor panel for the map editor
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct EditorData {
    pub enabled: bool,
    pub mod_folder: PathBuf,

    // display options
    #[serde(skip)]
    pub routes_metadata: HashMap<RouteId, RouteMetadata>,

    // runtime data
    #[serde(skip)]
    pub routes: Vec<Route>,
    #[serde(skip)]
    pub segments: HashMap<String, Segment>,
    #[serde(skip)]
    pub ports: HashMap<String, Port>,

    /// (segment id, point index)
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
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq, Hash)]
pub struct RouteId {
    pub start: String,
    pub destination: String,
    pub service: String,
}

// impl format for routeID
impl std::fmt::Display for RouteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {})",
            self.start, self.destination, self.service
        )
    }
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
        if ui
            .button(
                egui::RichText::new("Load routes")
                    .color(egui::Color32::DARK_GREEN)
                    .strong(),
            )
            .clicked()
        {
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

            // validate button
            if ui
                .button(egui::RichText::new("Validate routes").color(egui::Color32::ORANGE))
                .clicked()
            {
                // validate routes
                self.editor_data.routes_metadata.clear();
                // go through all routes and check if the segments in the route connect
                for route in &self.editor_data.routes {
                    let mut metadata = RouteMetadata { valid: true };

                    // check if all segments exist and put into a flat array
                    let mut segment_vec = Vec::new();
                    for segment in &route.segments {
                        if let Some(segment) = self.editor_data.segments.get(segment) {
                            segment_vec.push(segment);
                        }
                    }

                    // validate that all segments are there
                    if segment_vec.len() == route.segments.len() {
                        log::info!("Route '{}' is valid", route.id);
                    } else {
                        log::warn!("Route '{}' is invalid", route.id);
                        metadata.valid = false;
                    }

                    // validate that all segments connect
                    // start with the port

                    self.editor_data
                        .routes_metadata
                        .insert(route.id.clone(), metadata);
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
                        // parse the file as a port
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
                        // check if routes is invalid
                        if let Some(metadata) = self.editor_data.routes_metadata.get(&route.id) {
                            if !metadata.valid {
                                ui.label("✖");
                            } else {
                                ui.label("✅");
                            }
                        } else {
                            ui.label("❓");
                        }

                        // Determine if exactly this route is active (all its segments selected, none others)
                        let route_seg_set: std::collections::HashSet<&String> =
                            route.segments.iter().collect();

                        let all_in_route_selected = route.segments.iter().all(
                            |id| matches!(self.editor_data.segments.get(id), Some(s) if s.selected),
                        );
                        let any_outside_selected = self
                            .editor_data
                            .segments
                            .iter()
                            .any(|(id, s)| !route_seg_set.contains(id) && s.selected);

                        let mut checked = all_in_route_selected && !any_outside_selected;
                        if ui.checkbox(&mut checked, "").changed() {
                            // Exclusive selection: if checked, enable only this route's segments; if unchecked, disable all
                            if checked {
                                self.editor_data.current_segment = None;
                                for (_id, seg) in self.editor_data.segments.iter_mut() {
                                    seg.selected = false;
                                }
                                for seg_id in &route.segments {
                                    if let Some(seg) = self.editor_data.segments.get_mut(seg_id) {
                                        seg.selected = true;
                                    }
                                }
                            } else {
                                for (_id, seg) in self.editor_data.segments.iter_mut() {
                                    seg.selected = false;
                                }
                                self.editor_data.current_segment = None;
                            }
                        }

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

    // events
    pub fn editor_on_point_dragged(&mut self, from_screen: RectTransform, current_pos: Pos2) {
        // move the selected point if in editor mode
        if !self.editor_data.enabled {
            return;
        }

        if let Some((selected_point, i)) = &self.editor_data.selected_point {
            // get the segment
            if let Some(segment) = self.editor_data.segments.get_mut(selected_point) {
                // get the route
                if let Some(route1) = &mut segment.route1 {
                    // get the point
                    if let Some(_point) = route1.get_mut(*i) {
                        // translate the point to screen space
                        let clicked_point = from_screen * current_pos;
                        let engine_pos = self
                            .dimensions
                            .canvas_to_engine(Pos2::new(clicked_point.x, clicked_point.y));

                        // snap to port points but don't change the port
                        let mut snap_point = None;
                        for port in self.editor_data.ports.values() {
                            if snap_point.is_some() {
                                break;
                            }

                            for data in port.data.values() {
                                // start positions
                                {
                                    let port_pos = Pos2::new(data.position.x, data.position.y);
                                    let dist = (engine_pos - port_pos).length();
                                    if dist < 600.0 {
                                        // do not change the port position, just snap the point to it
                                        snap_point = Some(port_pos);
                                        break;
                                    }
                                }

                                // reverse start positions
                                {
                                    if let Some(position) = &data.positionStart {
                                        let port_pos = Pos2::new(position.x, position.y);
                                        let dist = (engine_pos - port_pos).length();
                                        if dist < 600.0 {
                                            // do not change the port position, just snap the point to it
                                            snap_point = Some(port_pos);
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // snap the point
                        if let Some(snap_point) = snap_point {
                            _point.x = snap_point.x;
                            _point.y = snap_point.y;
                        } else {
                            _point.x = engine_pos.x;
                            _point.y = engine_pos.y;
                        }

                        // check if the point is close to another point in a different segment
                        // and snap to it
                        for (id, segment) in self.editor_data.segments.iter_mut() {
                            // continue if segment is not selected
                            if !segment.selected {
                                continue;
                            }

                            if id != selected_point {
                                if let Some(route1) = &mut segment.route1 {
                                    for point in route1.iter_mut() {
                                        let dist =
                                            (engine_pos - Pos2::new(point.x, point.y)).length();
                                        if dist < 600.0 {
                                            // set the point to the new position
                                            if let Some(snap_point) = snap_point {
                                                point.x = snap_point.x;
                                                point.y = snap_point.y;
                                            } else {
                                                point.x = engine_pos.x;
                                                point.y = engine_pos.y;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn editor_on_ctrl_right_clicked(&mut self, from_screen: RectTransform, interact_pos: Pos2) {
        if !self.editor_data.enabled {
            return;
        }

        let found_point = self.get_point(from_screen, interact_pos, false);
        if let Some((id, i)) = found_point {
            // get the segment
            if let Some(segment) = self.editor_data.segments.get_mut(&id) {
                // get the route
                if let Some(route1) = &mut segment.route1 {
                    // remove the point
                    route1.remove(i);
                }
            }
        }
    }

    pub fn editor_on_ctrl_clicked(&mut self, from_screen: RectTransform, interact_pos: Pos2) {
        if !self.editor_data.enabled {
            return;
        }

        // if found, remove the point from the segment

        // add a point to the selected segment
        if let Some(selected_segment) = &self.editor_data.current_segment {
            // get the segment
            if let Some(segment) = self.editor_data.segments.get_mut(selected_segment) {
                // get the route
                if let Some(route1) = &mut segment.route1 {
                    // get point coordinates
                    let clicked_point = from_screen * interact_pos;
                    let engine_pos = self.dimensions.canvas_to_engine(clicked_point);

                    // add the point after the nearest point
                    let mut min_dist = f32::MAX;
                    let mut min_index = 0;
                    for (i, point) in route1.iter().enumerate() {
                        let dist = (engine_pos - Pos2::new(point.x, point.y)).length();
                        if dist < min_dist {
                            min_dist = dist;
                            min_index = i;
                        }
                    }
                    // add the point to the route
                    if min_index == route1.len() - 1 {
                        route1.push(Pos3::new(engine_pos.x, engine_pos.y, 0.0));
                    } else {
                        route1.insert(min_index, Pos3::new(engine_pos.x, engine_pos.y, 0.0));
                    }
                }
            }
        }
    }

    pub fn editor_on_ctrl_drag_started(&mut self, from_screen: RectTransform, interact_pos: Pos2) {
        // if ctr is pressed and editor mode is enabled
        if !self.editor_data.enabled {
            return;
        }

        let found_point = self.get_point(from_screen, interact_pos, false);
        if let Some((id, i)) = found_point {
            self.editor_data.selected_point = Some((id.clone(), i));
        }
    }

    pub fn editor_on_click(
        &mut self,
        _ui: &mut egui::Ui,
        from_screen: RectTransform,
        interact_pos: Pos2,
    ) -> bool {
        if !self.editor_data.enabled {
            return false;
        }

        let found_point = self.get_point(from_screen, interact_pos, true);

        // if found, remove the point from the segment
        if let Some((id, _i)) = found_point {
            // set the current segment
            self.editor_data.current_segment = Some(id.clone());
            return true;
        }

        false
    }

    pub fn editor_on_hover(
        &mut self,
        ui: &mut egui::Ui,
        response: &Response,
        from_screen: RectTransform,
        pointer_pos: Pos2,
    ) -> bool {
        if !self.editor_data.enabled {
            return false;
        }

        // context menu for editor mode
        // get point
        let found_point = self.get_point(from_screen, pointer_pos, true);
        if let Some((id, i)) = found_point {
            ui.set_width(200.0);

            if let Some(segment) = self.editor_data.segments.get_mut(&id) {
                // get the route
                if let Some(route1) = &mut segment.route1 {
                    if self.ui_data.show_tooltips && ui.ui_contains_pointer() {
                        response.clone().on_hover_ui_at_pointer(|ui| {
                            ui.set_width(200.0);

                            // labels
                            ui.label(format!("Segment: {}", id));
                            ui.label(format!("Point: {}", route1[i]));

                            ui.separator();

                            ui.label("Ctrl + Click to remove point".to_string());
                            ui.label("Ctrl + Drag to move point".to_string());
                            ui.label("Click to select segment".to_string());
                            ui.label("Ctrl + Click to add point".to_string());
                        });
                    }
                }
            }

            return true;
        }

        false
    }

    // methods

    fn get_point(
        &mut self,
        from_screen: RectTransform,
        interact_pos: Pos2,
        ignore_current_segment: bool,
    ) -> Option<(String, usize)> {
        // try to get a point from the segments if distace is less than 10 pixels
        let mut found = false;
        let mut found_point = None;

        // check if within distance of any point in displayed segments
        for (id, segment) in self.editor_data.segments.iter().filter(|(_, s)| s.selected) {
            // check if the segment is selected for editing
            if !ignore_current_segment {
                if let Some(selected_segment) = &self.editor_data.current_segment {
                    if selected_segment != id {
                        continue;
                    }
                }
            }

            if let Some(route1) = &segment.route1 {
                for (i, point) in route1.iter().enumerate() {
                    let clicked_point = from_screen * interact_pos;
                    let canvas_pos = self
                        .dimensions
                        .engine_to_canvas(Pos2::new(point.x, point.y));

                    let dist = (clicked_point - canvas_pos).length();
                    if dist < 0.1 {
                        found = true;
                        found_point = Some((id.clone(), i));
                        break;
                    }
                }
            }
            if found {
                break;
            }
        }
        found_point
    }
}
