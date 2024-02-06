#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    use log::{warn, LevelFilter};

    let log_path = format!("{}.log", NAME);
    let log_level = if cfg!(debug_assertions) {
        LevelFilter::Info
    } else {
        LevelFilter::Warn
    };
    let _ = simple_logging::log_to_file(log_path, log_level);
    warn!("{} v{}", NAME, VERSION);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .unwrap(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "tes3 map",
        native_options,
        Box::new(|cc| Box::new(tes3map::TemplateApp::new(cc))),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "tes3map_id", // hardcode it
                web_options,
                Box::new(|cc| Box::new(tes3map::TemplateApp::new(cc))),
            )
            .await
            .expect("failed to start eframe");
    });
}
