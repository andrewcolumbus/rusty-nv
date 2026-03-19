#![windows_subsystem = "windows"]

mod app;
mod export;
mod highlight;
mod highlighter;
mod import;
mod markup;
mod note;
mod search;
mod shortcuts;
mod storage;
mod theme;
mod ui;

use app::NvApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../logo.png"))
        .expect("Failed to load icon");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "rust-nv",
        native_options,
        Box::new(|cc| Ok(Box::new(NvApp::new(cc)))),
    )
}
