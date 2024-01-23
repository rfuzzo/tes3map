#![warn(clippy::all, rust_2018_idioms)]

mod app;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub use app::TemplateApp;
use egui::Color32;
use palette::{convert::FromColorUnclamped, Hsv, IntoColor, LinSrgb};

/// Get all plugins (esp, omwaddon, omwscripts) in a folder
fn get_plugins_in_folder<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get all plugins
    let mut results: Vec<PathBuf> = vec![];
    if let Ok(plugins) = std::fs::read_dir(path) {
        plugins.for_each(|p| {
            if let Ok(file) = p {
                let file_path = file.path();
                if file_path.is_file() {
                    if let Some(ext_os) = file_path.extension() {
                        let ext = ext_os.to_ascii_lowercase();
                        if ext == "esm"
                            || ext == "esp"
                            || (use_omw_plugins && ext == "omwaddon")
                            || (use_omw_plugins && ext == "omwscripts")
                        {
                            results.push(file_path);
                        }
                    }
                }
            }
        });
    }
    results
}

fn get_plugins_sorted<P>(path: &P, use_omw_plugins: bool) -> Vec<PathBuf>
where
    P: AsRef<Path>,
{
    // get plugins
    let mut plugins = get_plugins_in_folder(path, use_omw_plugins);

    // sort
    plugins.sort_by(|a, b| {
        fs::metadata(a.clone())
            .expect("filetime")
            .modified()
            .unwrap()
            .cmp(
                &fs::metadata(b.clone())
                    .expect("filetime")
                    .modified()
                    .unwrap(),
            )
    });

    plugins
}

fn get_color_for_height(
    value: f32,
    height_base: Color32,
    height_spectrum: usize,
    max_z: f32,
    depth_base: Color32,
    depth_spectrum: usize,
    min_z: f32,
) -> Color32 {
    if value < 0.0 {
        depth_to_color(value, depth_base, depth_spectrum, min_z)
    } else {
        height_to_color(value, height_base, height_spectrum, max_z)
    }
}

fn height_to_color(
    height: f32,
    height_base: Color32,
    height_spectrum: usize,
    max_z: f32,
) -> Color32 {
    let b: LinSrgb<u8> =
        LinSrgb::from_components((height_base.r(), height_base.g(), height_base.b()));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the height to the range [0.0, 1.0]
    let normalized_height = height / max_z;

    // Map normalized height to hue in the range [120.0, 30.0] (green to brown)
    // let hue = 120.0 - normalized_height * self.height_spectrum as f32;
    // let saturation = 1.0;
    // let value = 0.65;

    let hue = base.hue - normalized_height * height_spectrum as f32;
    let saturation = base.saturation;
    let value = 0.65;
    //base.value;

    // Create an HSV color
    let color = Hsv::new(hue, saturation, value);

    // Convert HSV to linear RGB
    let linear_rgb: LinSrgb = color.into_color();

    // Convert linear RGB to gamma-corrected RGB
    let c: LinSrgb<u8> = linear_rgb.into_format();

    Color32::from_rgb(c.red, c.green, c.blue)
}

fn depth_to_color(depth: f32, depth_base: Color32, depth_spectrum: usize, min_z: f32) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((depth_base.r(), depth_base.g(), depth_base.b()));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the depth to the range [0.0, 1.0]
    let normalized_depth = -depth / min_z;

    // Map normalized depth to hue in the range [240.0, 180.0] (blue to light blue)
    // let hue = 240.0 - normalized_depth * depth_spectrum as f32;
    // let saturation = 1.0;
    // let value = 0.8;

    let hue = base.hue - normalized_depth * depth_spectrum as f32;
    let saturation = base.saturation;
    let value = base.value;

    // Create an HSV color
    let color = Hsv::new(hue, saturation, value);

    // Convert HSV to linear RGB
    let linear_rgb: LinSrgb = color.into_color();

    // Convert linear RGB to gamma-corrected RGB
    let c: LinSrgb<u8> = linear_rgb.into_format();
    Color32::from_rgb(c.red, c.green, c.blue)
}
