use std::{path::PathBuf, process::Stdio};

use strum::EnumIter;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum AudioFormat {
  FLAC,
  Audacity,
  WAV,
  AAC,
  ALAC,
}

impl AudioFormat {
  fn extension(&self) -> &'static str {
    match self {
      AudioFormat::FLAC | AudioFormat::Audacity => "flac",
      AudioFormat::WAV => "wav",
      AudioFormat::AAC => "m4a",
      AudioFormat::ALAC => "m4a",
    }
  }

  fn ffmpeg_args(&self) -> Vec<&'static str> {
    match self {
      AudioFormat::FLAC | AudioFormat::Audacity => vec!["-c:a", "flac", "-f", "flac"],
      AudioFormat::WAV => vec!["-c:a", "pcm_s16le", "-f", "wav"],
      AudioFormat::AAC => vec!["-c:a", "aac", "-f", "ipod"],
      AudioFormat::ALAC => vec!["-c:a", "alac", "-f", "ipod"],
    }
  }

  pub fn display_name(&self) -> &'static str {
    match self {
      AudioFormat::FLAC => "FLAC",
      AudioFormat::WAV => "wav",
      AudioFormat::AAC => "AAC (MPEG-4)",
      AudioFormat::ALAC => "ALAC (Apple Lossless)",
      AudioFormat::Audacity => "Audacity Project",
    }
  }

  pub fn is_project_format(&self) -> bool {
    matches!(self, AudioFormat::Audacity)
  }
}

#[derive(Debug)]
pub enum ProcessProgress {
  Finished,
  Error(anyhow::Error),
}

pub static AUP_HEADER: &str = concat!(
  "<?xml version=\"1.0\" standalone=\"no\" ?>\n",
  "<!DOCTYPE project PUBLIC \"-//audacityproject-1.3.0//DTD//EN\" \"http://audacity.sourceforge.net/xml/audacityproject-1.3.0.dtd\" >\n",
  "<project xmlns=\"http://audacity.sourceforge.net/xml/\" projname=\"Craig\" version=\"1.3.0\" audacityversion=\"2.2.2\" rate=\"48000.0\">\n"
);

pub static AUP_FOLDER_NAME: &str = "craig_data";

pub async fn process_files(
  resource_path: PathBuf,
  root_output_path: PathBuf,
  format: AudioFormat,
  use_dynaudnorm: bool,
  mix: bool,
) -> anyhow::Result<()> {
  let mut output_path = root_output_path.clone();
  if format.is_project_format() {
    output_path.push(AUP_FOLDER_NAME);
  }
  // Create output directory if it doesn't exist
  tokio::fs::create_dir_all(&output_path).await?;

  // Get ffmpeg path
  let ffmpeg = resource_path.join("ffmpeg.exe");
  if !ffmpeg.exists() {
    return Err(anyhow::anyhow!("ffmpeg.exe not found in resources"));
  }

  // Collect FLAC files
  let mut entries = tokio::fs::read_dir(&resource_path).await?;
  let mut flac_files = Vec::new();

  while let Some(entry) = entries.next_entry().await? {
    let path = entry.path();
    if path.extension().and_then(|s| s.to_str()) == Some("flac") {
      flac_files.push(path);
    }
  }

  let mut result_files = Vec::new();

  if mix && !flac_files.is_empty() {
    // Mix all tracks into one file
    println!("Mixing {} tracks together", flac_files.len());
    
    // Create the filter complex string in chunks of 32 files
    let mut filter = String::new();
    let mut mix_filter = String::new();

    let mut command = Command::new(&ffmpeg);
    command.arg("-y");
    
    // Add all input files
    let mut co = 0;
    let mix_extra = {
      if use_dynaudnorm {
        ",dynaudnorm"
      } else {
        ""
      }
    };
    for (i, file) in flac_files.iter().enumerate() {
      command.arg("-i").arg(file);
      let input_filter = {
        if use_dynaudnorm {
          "dynaudnorm"
        } else {
          "anull"
        }
      };
      filter.push_str(&format!("[{i}:a]{input_filter}[aud{co}];"));
      mix_filter.push_str(&format!("[aud{co}]"));
      co += 1;

        
      // amix can only mix 32 at a time
      if co >= 32 {
        filter.push_str(&format!("{mix_filter} amix={co}{mix_extra}[aud{co}];"));
        mix_filter = format!("[aud{co}]") ;
        co = 1;
      }
    }

    filter.push_str(&format!("{mix_filter} amix={co}{mix_extra}[aud]"));
    command.args(["-filter_complex", &filter]);
    command.args(["-map", "[aud]"]);

    command.args(format.ffmpeg_args());

    let mut file_output_path = output_path.join("craig");
    file_output_path.set_extension(format.extension());
    result_files.push(file_output_path.file_name().unwrap().to_owned());
    command.arg(&file_output_path);

    println!("Running mix command");
    let status = command
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .status()
      .await?;
      
    if !status.success() {
      return Err(anyhow::anyhow!("ffmpeg mixing failed with status: {}", status));
    }
  } else {
    // Process files individually
    for input_path in flac_files {
      let mut file_output_path = output_path.join(
        input_path
          .file_name()
          .ok_or_else(|| anyhow::anyhow!("Invalid input filename"))?,
      );
      file_output_path.set_extension(format.extension());

      println!("Converting {:?} to {:?}", input_path, file_output_path);

      let mut command = Command::new(&ffmpeg);
      command.arg("-y").arg("-i").arg(&input_path);

      if use_dynaudnorm {
        command.args(["-af", "dynaudnorm"]);
      }

      command.args(format.ffmpeg_args());

      result_files.push(file_output_path.file_name().unwrap().to_owned());
      command.arg(&file_output_path);

      let status = command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await?;

      if !status.success() {
        return Err(anyhow::anyhow!("ffmpeg failed with status: {}", status));
      }
    }
  }


  if format.is_project_format() {
    // Create Audacity project file
    let mut aup = AUP_HEADER.to_owned();
    for file in result_files {
      aup.push_str(&format!(
        "\t<import filename=\"{}\" offset=\"0.00000000\" mute=\"0\" solo=\"0\" height=\"150\" minimized=\"0\" gain=\"1.0\" pan=\"0.0\"/>\n",
        file.to_string_lossy()
      ));
    }
    aup.push_str("</project>");

    tokio::fs::write(root_output_path.join("craig.aup"), aup).await?;
  }

  Ok(())
}
