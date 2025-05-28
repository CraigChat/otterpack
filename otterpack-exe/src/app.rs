use std::path::PathBuf;

pub struct TemplateApp {
  dynaudnorm: bool,
  output_path: PathBuf,
}

impl Default for TemplateApp {
  fn default() -> Self {
    Self {
      dynaudnorm: false,
      output_path: std::env::current_dir()
        .unwrap_or_default()
        .join("craig-out"),
    }
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

      ui.checkbox(&mut self.dynaudnorm, "Automatically level volume")
        .on_hover_text("...");

      ui.separator();

      if ui.button("Close").clicked() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      }

      ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        powered_by_egui_and_eframe(ui);
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
