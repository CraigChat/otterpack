use std::path::PathBuf;
use strum::IntoEnumIterator;
use tokio::sync::mpsc;

use crate::{AudioFormat, ProcessProgress, process_files};

#[derive(PartialEq)]
enum AppStatus {
  Ready,
  Processing,
  Error(String),
  Done,
}

pub struct TemplateApp {
  status: AppStatus,
  output_path: PathBuf,
  runtime: tokio::runtime::Handle,
  resources: Option<crate::self_extract::ExtractedResources>,
  completion_rx: Option<mpsc::UnboundedReceiver<ProcessProgress>>,
  selected_format: AudioFormat,
  dynaudnorm: bool,
  mix: bool,
}

impl Default for TemplateApp {
  fn default() -> Self {
    let runtime = tokio::runtime::Handle::current();
    let mut app = Self {
      status: AppStatus::Ready,
      runtime,
      resources: None,
      completion_rx: None,
      output_path: {
        let folder = if cfg!(debug_assertions) {
          "out".to_string()
        } else {
          std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "otterpack-out".to_string())
        };
        std::env::current_dir().unwrap_or_default().join(folder)
      },
      selected_format: AudioFormat::FLAC,
      dynaudnorm: false,
      mix: false,
    };

    // Extract and validate resources at startup
    match crate::self_extract::setup_resources() {
      Ok(resources) => {
        app.resources = Some(resources);
      }
      Err(e) => {
        app.status = AppStatus::Error(format!("Failed to setup resources: {}", e));
      }
    }

    app
  }
}

impl TemplateApp {
  /// Called once before the first frame.
  pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
    // This is also where you can customize the look and feel of egui using
    // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

    Default::default()
  }
}

impl eframe::App for TemplateApp {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |ui| {
      // Show error message at the top if there is one
      if let AppStatus::Error(error) = &self.status {
        ui.colored_label(egui::Color32::RED, error);
        ui.add_space(32.0);

        if ui.button("Close").clicked() {
          ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
      } else {
        ui.vertical(|ui| {
          if self.status != AppStatus::Ready {
            ui.disable();
          }

          ui.horizontal(|ui| {
            ui.label("Output folder:");
            let mut path_string = self.output_path.to_string_lossy().to_string();
            let output_field = ui.text_edit_singleline(&mut path_string);
            if ui.button("📁 Browse...").clicked() {
              if let Some(path) = rfd::FileDialog::new()
                .set_directory(&self.output_path)
                .pick_folder()
              {
                self.output_path = path;
              }
            }
            // Update PathBuf if text was manually edited
            if output_field.changed() {
              self.output_path = PathBuf::from(&path_string);
            }
            output_field.on_hover_text("The folder where extracted files will be saved");
          });

          ui.horizontal(|ui| {
            ui.label("Format:");
            egui::ComboBox::from_id_salt("format_combo")
              .selected_text(self.selected_format.display_name())
              .width(ui.available_width())
              .show_ui(ui, |ui| {
                for format in AudioFormat::iter() {
                  ui.selectable_value(&mut self.selected_format, format, format.display_name());
                }
              });
          });

          ui.add_space(8.0);

          ui.checkbox(&mut self.mix, "Mix into single track")
            .on_hover_text("Mix all tracks into one file");

          ui.checkbox(&mut self.dynaudnorm, "Automatically level volume")
            .on_hover_text("Normalize audio volume using FFmpeg's dynaudnorm filter");
        });

        ui.separator();

        if self.status == AppStatus::Ready {
          if ui
            .add_sized([ui.available_width(), 20.0], egui::Button::new("Go"))
            .clicked()
          {
            if let Some(resources) = &self.resources {
              let (completion_tx, completion_rx) = mpsc::unbounded_channel();
              self.completion_rx = Some(completion_rx);
              self.status = AppStatus::Processing;

              let resource_path = resources.resource_path.clone();
              let output_path = self.output_path.clone();
              let format = self.selected_format;
              let use_dynaudnorm = self.dynaudnorm;
              let mix = self.mix;

              // Spawn the async task
              self.runtime.spawn(async move {
                if let Err(e) =
                  process_files(resource_path, output_path, format, use_dynaudnorm, mix).await
                {
                  eprintln!("Error processing files: {}", e);
                  let _ = completion_tx.send(ProcessProgress::Error(e));
                } else {
                  let _ = completion_tx.send(ProcessProgress::Finished);
                }
              });
            }
          }
        } else if self.status == AppStatus::Processing {
          ui.heading("Processing files...");
          // Check for completion
          if let Some(rx) = &mut self.completion_rx {
            if let Ok(msg) = rx.try_recv() {
              self.completion_rx = None;
              if let ProcessProgress::Error(e) = msg {
                self.status = AppStatus::Error(format!("Failed to process: {}", e));
              } else {
                self.status = AppStatus::Done;
              }
              ctx.send_viewport_cmd(egui::viewport::ViewportCommand::RequestUserAttention(
                egui::UserAttentionType::Critical,
              ));
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
          }
        } else if self.status == AppStatus::Done {
          ui.heading("Finished processing files!");
          ui.add_space(4.0);
          ui.horizontal(|ui| {
            if ui.button("Open output folder").clicked() {
              let _ = opener::reveal(&self.output_path);
            }
            if ui.button("Close").clicked() {
              ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
          });
        }
      }

      ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        ui.horizontal(|ui| {
          ui.spacing_mut().item_spacing.x = 0.0;
          ui.label("Executable created with ");
          ui.hyperlink_to("Craig", "https://craig.chat");
          ui.label(" using ");
          ui.hyperlink_to("otterpack", "https://github.com/CraigChat/otterpack");
          ui.label(egui::RichText::new(format!(" ({}) ", env!("CARGO_PKG_VERSION"))).small());
          ui.label(".");
        });
        egui::warn_if_debug_build(ui);
      });
    });
  }
}
