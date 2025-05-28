#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use otterpack_exe::TemplateApp;

fn main() -> eframe::Result {
  env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

  let native_options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([500.0, 200.0])
      .with_min_inner_size([500.0, 200.0]),
    // .with_icon(
    //     // NOTE: Adding an icon is optional
    //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
    //         .expect("Failed to load icon"),
    // ),
    ..Default::default()
  };
  eframe::run_native(
    "Craig Audio Procesor",
    native_options,
    Box::new(|cc| Ok(Box::new(TemplateApp::new(cc)))),
  )
}
