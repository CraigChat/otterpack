use std::{path::PathBuf, process::Stdio};

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
  mix: bool,
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
    let mut mixes = 0;
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
        filter.push_str(&format!("{mix_filter} amix={co}{mix_extra}[mix{mixes}]"));
        mix_filter.push_str(&format!("[aud{mixes}]"));
        co = 1;
        mixes += 1;
      }
    }

    filter.push_str(&format!("{mix_filter} amix={co}{mix_extra}[aud]"));
    command.args(&["-filter_complex", &filter]);
    command.args(&["-map", "[aud]"]);
    
    // Add format-specific encoder args
    command.args(format.ffmpeg_args());
    
    // Set output path
    let mut mix_output = output_path.join("craig");
    mix_output.set_extension(format.extension());
    command.arg(&mix_output);
    
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
      let mut output_path = output_path.join(
        input_path
          .file_name()
          .ok_or_else(|| anyhow::anyhow!("Invalid input filename"))?,
      );
      output_path.set_extension(format.extension());

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

  Ok(())
}
