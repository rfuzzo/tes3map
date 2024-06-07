use std::collections::hash_map::Entry;

use log::{info, warn};
use tes3::esp::{Cell, Landscape, LandscapeTexture, Npc, Plugin, Region};

use crate::*;

impl TemplateApp {
    pub fn plugins_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Plugins");

        // load plugins
        if self.plugins.is_none() && self.data_files.is_some() {
            self.refresh_plugins();
        }

        // data files path and open button
        ui.horizontal(|ui| {
            if let Some(data_files) = &self.data_files {
                ui.label(format!("{}", data_files.display()));
            } else {
                ui.label("No data files loaded");
            }
            // open folder button
            if ui.button("🗁").clicked() {
                let folder_option = rfd::FileDialog::new().pick_folder();
                if let Some(path) = folder_option {
                    self.data_files = Some(path.clone());
                    // populate plugins here
                    let plugins = get_plugins_sorted(&path, false);
                    let vms = plugins
                        .iter()
                        .map(|p| PluginViewModel::from_path(p.clone()))
                        .collect();
                    self.plugins = Some(vms);
                }
            }
        });

        if self.data_files.is_none() {
            return;
        }

        // buttons
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                self.refresh_plugins();
            }

            if ui.button("Select all").clicked() {
                if let Some(plugins) = &mut self.plugins {
                    for vm in plugins.iter_mut() {
                        vm.enabled = true;
                    }
                }
            }
            // deselect all
            if ui.button("Select none").clicked() {
                if let Some(plugins) = &mut self.plugins {
                    for vm in plugins.iter_mut() {
                        vm.enabled = false;
                    }
                }
            }

            ui.visuals_mut().override_text_color = Some(Color32::DARK_GREEN);
            if ui.button("Load").clicked() {
                self.load_plugin_data();
                self.reload_background(ctx, None, true, true);
                self.reload_paths(ctx);
            }
            ui.visuals_mut().override_text_color = None;
        });

        ui.separator();

        // search bar
        ui.horizontal(|ui| {
            ui.label("Filter: ");
            ui.text_edit_singleline(&mut self.runtime_data.plugin_filter);
            // clear filter button
            if ui.button("x").clicked() {
                self.runtime_data.plugin_filter.clear();
            }
        });

        // plugins list
        if let Some(plugins) = &mut self.plugins {
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    // show plugins
                    for vm in plugins.iter_mut() {
                        // upper and lowercase search
                        let name = vm.get_name();

                        if !self.runtime_data.plugin_filter.is_empty()
                            && !name
                                .to_lowercase()
                                .contains(&self.runtime_data.plugin_filter.to_lowercase())
                        {
                            continue;
                        }

                        // checkbox with filename

                        ui.checkbox(&mut vm.enabled, name);
                    }
                });
        }
    }

    fn refresh_plugins(&mut self) {
        // populate plugins here
        let path = self.data_files.as_ref().unwrap();
        let plugins = get_plugins_sorted(&path, false);
        let vms = plugins
            .iter()
            .map(|p| PluginViewModel::from_path(p.clone()))
            .collect();
        self.plugins = Some(vms);
    }

    fn load_plugin_data(&mut self) {
        // guarded return on self.plugins
        if self.plugins.is_none() {
            warn!("No plugins loaded");
            return;
        }

        // clear previous data
        self.land_records.clear();
        self.ltex_records.clear();
        self.regn_records.clear();
        self.travel_edges.clear();
        self.cell_records.clear();
        self.cell_conflicts.clear();

        // load plugins into memory
        let mut land_records: HashMap<CellKey, Landscape> = HashMap::default();

        let mut cells: HashMap<CellKey, Cell> = HashMap::default();
        //let mut land_id_map: HashMap<String, CellKey> = HashMap::default();
        let mut cell_conflicts: HashMap<CellKey, Vec<u64>> = HashMap::default();
        let mut travels: HashMap<String, (Vec<CellKey>, String)> = HashMap::default();
        let mut npcs: HashMap<String, CellKey> = HashMap::default();

        let enabled_plugins: Vec<&PluginViewModel> = self
            .plugins
            .as_ref()
            .unwrap()
            .iter()
            .filter(|p| p.enabled)
            .collect();

        for vm in enabled_plugins {
            let path = vm.path.clone();
            let mut plugin = Plugin::new();
            if plugin
                .load_path_filtered(&path, |tag| {
                    matches!(
                        &tag,
                        b"TES3" | b"LAND" | b"LTEX" | b"CELL" | b"NPC_" | b"REGN"
                    )
                })
                .is_ok()
            {
                info!(
                    "\t== loading plugin {} with hash {}",
                    path.display(),
                    vm.hash
                );

                // add travels
                for npc in plugin.objects_of_type::<Npc>() {
                    let travel_destinations = npc.travel_destinations.clone();
                    if !travel_destinations.is_empty() {
                        let mut travel_destination_cells: Vec<CellKey> = vec![];
                        for d in travel_destinations {
                            let mut x = (d.translation[0] / 8192.0) as i32;
                            if x < 0 {
                                x -= 1;
                            }
                            let mut y = (d.translation[1] / 8192.0) as i32;
                            if y < 0 {
                                y -= 1;
                            }

                            travel_destination_cells.push((x, y));
                        }

                        // get npc class
                        let class = &npc.class;
                        travels.insert(npc.id.clone(), (travel_destination_cells, class.clone()));
                    }
                }

                // add Cells
                for cell in plugin.objects_of_type::<Cell>() {
                    if cell.is_interior() {
                        continue;
                    }

                    let key = (cell.data.grid.0, cell.data.grid.1);

                    for (npc_id, _) in travels.clone() {
                        if cell.references.iter().any(|p| p.1.id == npc_id) {
                            npcs.insert(npc_id, key);
                        }
                    }

                    if let Entry::Vacant(e) = cell_conflicts.entry(key) {
                        e.insert(vec![vm.hash]);
                    } else {
                        let mut value = cell_conflicts.get(&key).unwrap().to_owned();
                        value.push(vm.hash);
                        cell_conflicts.insert(key, value);
                    }

                    cells.insert(key, cell.clone());
                }

                // add landscape
                for land in plugin.objects_of_type::<Landscape>() {
                    let key = (land.grid.0, land.grid.1);

                    // add landscape
                    land_records.insert(key, land.clone());
                    //land_id_map.insert(get_unique_id(&TES3Object::Landscape(land.clone())), key);
                }

                // add landscape textures
                for ltex in plugin.objects_of_type::<LandscapeTexture>() {
                    // add landscape
                    self.ltex_records.insert(ltex.index, ltex.clone());
                }

                // add regions
                for region in plugin.objects_of_type::<Region>() {
                    self.regn_records.insert(region.id.clone(), region.clone());
                }
            }
        }

        // travel overlay
        let mut edges: Vec<(String, (CellKey, CellKey))> = vec![];
        for (key, start) in npcs.clone() {
            if let Some((dest, class)) = travels.get(&key) {
                for d in dest {
                    if !edges.contains(&(class.to_string(), (*d, start))) {
                        edges.push((class.to_string(), (start, *d)));
                    }
                }
            }
        }
        edges.dedup();
        let mut ordered_edges = HashMap::default();
        for (class, _pairs) in edges.iter() {
            ordered_edges.insert(class.to_string(), vec![]);
        }
        for (class, pair) in edges {
            if let Some(v) = ordered_edges.get_mut(&class) {
                v.push(pair);
            }
        }
        self.travel_edges = ordered_edges;

        // get final list of cells
        for (k, v) in cell_conflicts.iter().filter(|p| p.1.len() > 1) {
            self.cell_conflicts.insert(*k, v.to_vec());
        }

        self.land_records = land_records;
        self.cell_records = cells;
        // self.land_ids = land_id_map;
    }
}
