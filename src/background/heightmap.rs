use eframe::epaint::{Color32, ColorImage};
use palette::{convert::FromColorUnclamped, Hsv, IntoColor, LinSrgb};

use crate::{Dimensions, HeightmapSettings, VERTEX_CNT};

fn height_to_color(height: f32, dimensions: &Dimensions, settings: &HeightmapSettings) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        settings.height_base.r(),
        settings.height_base.g(),
        settings.height_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the height to the range [0.0, 1.0]
    let normalized_height = height / dimensions.max_z;

    let hue = base.hue + normalized_height * settings.height_spectrum as f32;
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

fn depth_to_color(depth: f32, dimensions: &Dimensions, settings: &HeightmapSettings) -> Color32 {
    let b: LinSrgb<u8> = LinSrgb::from_components((
        settings.depth_base.r(),
        settings.depth_base.g(),
        settings.depth_base.b(),
    ));
    let base = Hsv::from_color_unclamped(b.into_format::<f32>());

    // Normalize the depth to the range [0.0, 1.0]
    let normalized_depth = depth / dimensions.min_z;

    let hue = base.hue + normalized_depth * settings.depth_spectrum as f32;
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

fn get_color_for_height(
    value: f32,
    dimensions: &Dimensions,
    settings: &HeightmapSettings,
) -> Color32 {
    if value < dimensions.min_z {
        return Color32::TRANSPARENT;
    }

    if value < 0.0 {
        depth_to_color(value, dimensions, settings)
    } else {
        height_to_color(value, dimensions, settings)
    }
}
pub fn generate_heightmap(
    pixels: &[f32],
    dimensions: &Dimensions,
    settings: &HeightmapSettings,
) -> ColorImage {
    let size = dimensions.pixel_size_tuple(VERTEX_CNT);
    let mut img = ColorImage::new(size, Color32::WHITE);
    let p = pixels
        .iter()
        .map(|f| get_color_for_height(*f, dimensions, settings))
        .collect::<Vec<_>>();
    img.pixels = p;
    img
}
