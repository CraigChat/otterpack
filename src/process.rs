use std::path::PathBuf;

use strum::EnumIter;
use tokio::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter)]
pub enum AudioFormat {
  FLAC,
  WAV,
  AAC,
  ALAC,
}

impl AudioFormat {
  fn extension(&self) -> &'static str {
    match self {
      AudioFormat::FLAC => "flac",
      AudioFormat::WAV => "wav",
      AudioFormat::AAC => "m4a",
      AudioFormat::ALAC => "m4a",
    }
  }

  fn ffmpeg_args(&self) -> Vec<&'static str> {
    match self {
      AudioFormat::FLAC => vec!["-c:a", "flac", "-f", "flac"],
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
    }
  }
}

#[derive(Debug)]
pub enum ProcessProgress {
  Finished,
  Error(anyhow::Error),
}

pub async fn process_files(
  resource_path: PathBuf,
  output_path: PathBuf,
  format: AudioFormat,
  use_dynaudnorm: bool,
) -> anyhow::Result<()> {
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

  for input_path in flac_files {
    let output_filename = input_path
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("Invalid input filename"))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Invalid input filename encoding"))?
      .replace(".flac", &format!(".{}", format.extension()));

    let output_path = output_path.join(output_filename);

    println!("Converting {:?} to {:?}", input_path, output_path);

    let mut command = Command::new(&ffmpeg);
    command.arg("-y").arg("-i").arg(&input_path);

    // Add dynaudnorm filter if enabled
    if use_dynaudnorm {
      command.args(&["-af", "dynaudnorm"]);
    }

    // Add format-specific encoder args
    command.args(format.ffmpeg_args());

    // Add output path
    command.arg(&output_path);

    let status = command.status().await?;

    if !status.success() {
      return Err(anyhow::anyhow!("ffmpeg failed with status: {}", status));
    }
  }

  Ok(())
}
