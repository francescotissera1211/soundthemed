//! Theme creator — converts a folder of audio files into a
//! freedesktop-compliant sound theme.

use crate::sound_ids;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Supported input audio formats
const INPUT_EXTENSIONS: &[&str] = &["oga", "ogg", "mp3", "wav", "m4a", "flac", "opus", "wma", "aac"];

/// Result of creating a theme
#[derive(Debug)]
pub struct CreateResult {
    pub theme_dir: PathBuf,
    pub converted: Vec<String>,
    pub skipped: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

/// Create a new sound theme from a directory of audio files.
///
/// Files should be named after freedesktop event IDs (e.g., device-added.mp3).
/// All files are converted to Ogg Vorbis (.oga) and placed in the proper
/// directory structure under ~/.local/share/sounds/<name>/.
pub fn create_theme(name: &str, from_dir: &Path) -> Result<CreateResult, String> {
    if name.is_empty() {
        return Err("Theme name cannot be empty".into());
    }

    if !from_dir.is_dir() {
        return Err(format!("Source directory does not exist: {}", from_dir.display()));
    }

    // Check ffmpeg is available
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err("ffmpeg is not installed or not in PATH".into());
    }

    let theme_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sounds")
        .join(name);

    let stereo_dir = theme_dir.join("stereo");
    std::fs::create_dir_all(&stereo_dir)
        .map_err(|e| format!("Failed to create theme directory: {e}"))?;

    // Write index.theme
    let index_content = format!(
        "[Sound Theme]\nName={name}\nDirectories=stereo\n\n[stereo]\nOutputProfile=stereo\n"
    );
    std::fs::write(theme_dir.join("index.theme"), index_content)
        .map_err(|e| format!("Failed to write index.theme: {e}"))?;

    let mut result = CreateResult {
        theme_dir: theme_dir.clone(),
        converted: Vec::new(),
        skipped: Vec::new(),
        warnings: Vec::new(),
    };

    // Scan input directory
    let entries = std::fs::read_dir(from_dir)
        .map_err(|e| format!("Failed to read source directory: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };

        if !INPUT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let event_id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Warn if not a known freedesktop ID
        if sound_ids::description_for(&event_id).is_none() {
            result.warnings.push(format!(
                "'{event_id}' is not a standard freedesktop sound event ID"
            ));
        }

        let output_path = stereo_dir.join(format!("{event_id}.oga"));

        // If already .oga or .ogg, just copy
        if ext == "oga" || ext == "ogg" {
            match std::fs::copy(&path, &output_path) {
                Ok(_) => result.converted.push(event_id),
                Err(e) => result.skipped.push((event_id, format!("copy failed: {e}"))),
            }
            continue;
        }

        // Convert with ffmpeg
        let status = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(&path)
            .args(["-c:a", "libvorbis", "-q:a", "5"])
            .arg(&output_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => result.converted.push(event_id),
            Ok(s) => result.skipped.push((
                event_id,
                format!("ffmpeg exited with code {}", s.code().unwrap_or(-1)),
            )),
            Err(e) => result.skipped.push((event_id, format!("ffmpeg error: {e}"))),
        }
    }

    log::info!(
        "theme '{}' created: {} sounds converted, {} skipped",
        name,
        result.converted.len(),
        result.skipped.len()
    );

    Ok(result)
}
