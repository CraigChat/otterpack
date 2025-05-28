use std::path::PathBuf;
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum AudioFormat {
  FLAC,
  WAV,
  ALAC,
}

impl AudioFormat {
  fn extension(&self) -> &'static str {
    match self {
      AudioFormat::FLAC => "flac",
      AudioFormat::WAV => "wav",
      AudioFormat::ALAC => "m4a",
    }
  }

  fn ffmpeg_args(&self) -> Vec<&'static str> {
    match self {
      AudioFormat::FLAC => vec!["-c:a", "flac"],
      AudioFormat::WAV => vec!["-c:a", "pcm_s16le"],
      AudioFormat::ALAC => vec!["-c:a", "alac"],
    }
  }

  fn display_name(&self) -> &'static str {
    match self {
      AudioFormat::FLAC => "FLAC",
      AudioFormat::WAV => "WAV",
      AudioFormat::ALAC => "ALAC (Apple Lossless)",
    }
  }
}

pub struct TemplateApp {
  dynaudnorm: bool,
  output_path: PathBuf,
  selected_format: AudioFormat,
  processing: bool,
  #[allow(dead_code)]
  runtime: tokio::runtime::Handle,
  error: Option<String>,
  resources: Option<crate::self_extract::ExtractedResources>,
}

impl Default for TemplateApp {
  fn default() -> Self {
    let runtime = tokio::runtime::Handle::current();
    let mut app = Self {
      dynaudnorm: false,
      output_path: std::env::current_dir()
        .unwrap_or_default()
        .join("craig-out"),
      selected_format: AudioFormat::FLAC,
      processing: false,
      runtime,
      error: None,
      resources: None,
    };

    // Extract and validate resources at startup
    match crate::self_extract::setup_resources() {
      Ok(resources) => {
        app.resources = Some(resources);
      }
      Err(e) => {
        app.error = Some(format!("Failed to setup resources: {}", e));
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
      if let Some(error) = &self.error {
        ui.colored_label(egui::Color32::RED, error);
        ui.add_space(32.0);
      } else {
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

        ui.separator();

        ui.horizontal(|ui| {
          ui.label("Output format:");
          for format in AudioFormat::iter() {
            let text = format.display_name();
            if ui
              .selectable_label(self.selected_format == format, text)
              .clicked()
            {
              self.selected_format = format;
            }
          }
        });

        ui.separator();

        ui.checkbox(&mut self.dynaudnorm, "Automatically level volume")
          .on_hover_text("Normalize audio volume using ffmpeg's dynaudnorm filter");

        ui.separator();
      }

      if ui.button("Close").clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      }

      ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        powered_by_egui_and_eframe(ui);
        // Show temp directory
        if let Some(resources) = &self.resources {
          if let Some(temp_dir) = &resources.temp_dir {
            let dir = temp_dir.path().to_string_lossy().to_string();
            ui.label(egui::RichText::new(format!("Temp Folder: {dir}")).small());
          }
        }
        egui::warn_if_debug_build(ui);
      });
    });
  }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
  ui.horizontal(|ui| {
    ui.spacing_mut().item_spacing.x = 0.0;
    ui.label("Executable created with ");
    ui.hyperlink_to("Craig", "https://craig.chat");
    ui.label(".");
  });
}
