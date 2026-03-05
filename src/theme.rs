//! Sound theme resolver — finds sound files following the
//! freedesktop sound theme spec.
//!
//! Search order:
//!   1. ~/.local/share/sounds/<theme>/
//!   2. /usr/share/sounds/<theme>/
//!   3. ~/.local/share/sounds/freedesktop/
//!   4. /usr/share/sounds/freedesktop/
//!
//! Within each theme dir, looks for the sound ID in
//! stereo/ then fallback to the root, trying common
//! extensions: .oga, .ogg, .wav

use std::path::{Path, PathBuf};

/// Supported audio file extensions in preference order
const EXTENSIONS: &[&str] = &["oga", "ogg", "wav"];

/// Subdirectories to search within a theme (in order)
const SUBDIRS: &[&str] = &["stereo", "."];

/// Resolve a sound event ID to a file path.
///
/// Follows the freedesktop sound theme spec search order:
/// try the requested theme first, then fall back to "freedesktop".
///
/// # Arguments
/// * `theme` - Sound theme name (e.g. "ocean")
/// * `sound_id` - Event sound ID (e.g. "device-added")
///
/// # Returns
/// The path to the sound file, or None if not found.
pub fn resolve(theme: &str, sound_id: &str) -> Option<PathBuf> {
    let search_dirs = build_search_dirs(theme);

    for base in &search_dirs {
        for subdir in SUBDIRS {
            let dir = if *subdir == "." {
                base.clone()
            } else {
                base.join(subdir)
            };

            for ext in EXTENSIONS {
                let path = dir.join(format!("{sound_id}.{ext}"));
                if path.is_file() {
                    log::debug!("resolved {sound_id} -> {}", path.display());
                    return Some(path);
                }
            }
        }
    }

    log::debug!("sound not found: {sound_id} (theme: {theme})");
    None
}

/// Build the list of directories to search, in priority order.
fn build_search_dirs(theme: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    let user_sounds = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sounds");

    let system_sounds = Path::new("/usr/share/sounds");

    // Requested theme first
    dirs.push(user_sounds.join(theme));
    dirs.push(system_sounds.join(theme));

    // Fallback to freedesktop theme (if not already the requested theme)
    if theme != "freedesktop" {
        dirs.push(user_sounds.join("freedesktop"));
        dirs.push(system_sounds.join("freedesktop"));
    }

    dirs
}
