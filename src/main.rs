#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use otterpack::TemplateApp;

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
  let _guard = runtime.enter();

  let native_options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([500.0, 300.0])
      .with_min_inner_size([500.0, 300.0])
      .with_icon(
        eframe::icon_data::from_png_bytes(&include_bytes!("../assets/otter.png")[..])
          .expect("Failed to load icon"),
      )
      .with_active(true),
    ..Default::default()
  };
  eframe::run_native(
    "Craig Audio Processor",
    native_options,
    Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
  )
}
