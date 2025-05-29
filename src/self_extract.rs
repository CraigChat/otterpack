use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug)]
pub enum PackSource {
  DebugFolder(PathBuf),
  EmbeddedZip {
    exe_path: PathBuf,
    zip_start: u64,
    zip_size: u64,
  },
}

const ZIP_MAGIC: &[u8] = b"PK\x03\x04";
const MAX_SEARCH_SIZE: u64 = 10 * 1024 * 1024; // Look for ZIP signature in the first 10MB

pub fn find_pack_source() -> Result<PackSource> {
  if cfg!(debug_assertions) {
    // In debug mode, look for _otterpack folder relative to executable
    let debug_folder = std::env::current_dir()?.to_path_buf().join("_otterpack");

    if debug_folder.is_dir() {
      return Ok(PackSource::DebugFolder(debug_folder));
    }
  }

  // Try to find embedded zip
  let exe_path = std::env::current_exe()?;
  let mut file = File::open(&exe_path)?;

  // Get file size
  let file_size = file.metadata()?.len();

  // Buffer for reading
  let mut buffer = vec![0u8; ZIP_MAGIC.len()];
  let mut pos = 0;

  // Search for ZIP magic number from the start (only search first MAX_SEARCH_SIZE bytes)
  let search_size = file_size.min(MAX_SEARCH_SIZE);

  while pos < search_size - ZIP_MAGIC.len() as u64 {
    file.seek(SeekFrom::Start(pos))?;
    file.read_exact(&mut buffer)?;

    if buffer == ZIP_MAGIC {
      // Found ZIP header - read until end of file
      let zip_start = pos;
      let zip_size = file_size - pos;

      return Ok(PackSource::EmbeddedZip {
        exe_path,
        zip_start,
        zip_size,
      });
    }

    pos += 1;
  }

  if cfg!(debug_assertions) {
    anyhow::bail!(
      "No _otterpack folder or embedded ZIP found. In debug mode, place ffmpeg.exe in the _otterpack folder."
    )
  } else {
    anyhow::bail!(
      "This executable does not have a bundled ZIP file. Please use a properly packaged version."
    )
  }
}

fn extract_zip_contents(source: &PackSource) -> Result<tempfile::TempDir> {
  match source {
    PackSource::EmbeddedZip {
      exe_path,
      zip_start,
      zip_size,
    } => {
      // Open the exe file
      let mut exe_file = File::open(exe_path)?;

      // Seek to the start of ZIP data
      exe_file.seek(SeekFrom::Start(*zip_start))?;

      // Read the ZIP portion into memory (this is usually small, just contains ffmpeg.exe)
      let mut zip_data = vec![0u8; *zip_size as usize];
      exe_file.read_exact(&mut zip_data)?;

      // Create ZIP archive from the data
      let cursor = std::io::Cursor::new(zip_data);
      let mut archive = zip::ZipArchive::new(cursor).context("Failed to read ZIP data")?;

      // Create temporary directory
      let temp_dir = tempfile::Builder::new().prefix("otterpack-").tempdir()?;

      // Extract only root-level files from the ZIP
      for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        // Skip directories and files in subdirectories
        if name.ends_with('/') || name.contains('/') || name.contains('\\') {
          continue;
        }

        // Create output path in temp directory
        let out_path = temp_dir.path().join(name);

        // Create the file and copy contents
        let mut outfile = File::create(&out_path)
          .context(format!("Failed to create file: {}", out_path.display()))?;
        io::copy(&mut file, &mut outfile)
          .context(format!("Failed to write file: {}", out_path.display()))?;
      }

      Ok(temp_dir)
    }
    PackSource::DebugFolder(_) => {
      anyhow::bail!("Cannot extract from debug folder - resources should be used directly")
    }
  }
}

pub struct ExtractedResources {
  pub temp_dir: Option<tempfile::TempDir>,
  pub resource_path: PathBuf,
}

pub async fn setup_resources() -> Result<ExtractedResources> {
  tokio::task::spawn_blocking(|| {
    let source = find_pack_source()?;

    match source {
      PackSource::DebugFolder(path) => {
        // Validate debug folder contents
        let ffmpeg_path = path.join("ffmpeg.exe");
        if !ffmpeg_path.exists() {
          anyhow::bail!(
            "ffmpeg.exe not found in debug folder at {}",
            ffmpeg_path.display()
          );
        }

        Ok(ExtractedResources {
          temp_dir: None,
          resource_path: path,
        })
      }
      PackSource::EmbeddedZip { .. } => {
        // Extract and validate contents
        let temp_dir = extract_zip_contents(&source)?;

        // Validate the extracted contents
        let ffmpeg_path = temp_dir.path().join("ffmpeg.exe");
        if !ffmpeg_path.exists() {
          anyhow::bail!("ffmpeg.exe not found in extracted resources");
        }

        let resource_path = temp_dir.path().to_owned();
        Ok(ExtractedResources {
          temp_dir: Some(temp_dir),
          resource_path,
        })
      }
    }
  })
  .await?
}
