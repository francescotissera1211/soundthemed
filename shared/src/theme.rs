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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Supported audio file extensions in preference order
const EXTENSIONS: &[&str] = &["oga", "ogg", "wav"];

/// Subdirectories to search within a theme (in order)
const SUBDIRS: &[&str] = &["stereo", "."];

/// Info about an installed sound theme
#[derive(Debug, Clone)]
pub struct ThemeInfo {
    /// Directory name (used as the theme identifier)
    pub id: String,
    /// Display name from index.theme, or the directory name if not found
    pub display_name: String,
    /// Path to the theme directory
    pub path: PathBuf,
}

/// Resolve a sound event ID to a file path.
///
/// Follows the freedesktop sound theme spec search order:
/// try the requested theme first, then fall back to "freedesktop".
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

/// List all installed sound themes from both user and system directories.
pub fn list_themes() -> Vec<ThemeInfo> {
    let mut themes: HashMap<String, ThemeInfo> = HashMap::new();

    let user_sounds = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sounds");
    let system_sounds = PathBuf::from("/usr/share/sounds");

    // System first, then user (user overrides system)
    for base in [&system_sounds, &user_sounds] {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                // Must have at least one sound file to count as a theme
                if !has_sound_files(&path) {
                    continue;
                }

                let display_name = read_theme_name(&path).unwrap_or_else(|| id.clone());

                themes.insert(
                    id.clone(),
                    ThemeInfo {
                        id,
                        display_name,
                        path,
                    },
                );
            }
        }
    }

    let mut result: Vec<ThemeInfo> = themes.into_values().collect();
    result.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    result
}

/// List all sound files in a theme, returning (event_id, path) pairs.
pub fn list_theme_sounds(theme: &str) -> Vec<(String, PathBuf)> {
    let mut sounds = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let user_sounds = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("sounds");
    let system_sounds = PathBuf::from("/usr/share/sounds");

    let theme_dirs = [user_sounds.join(theme), system_sounds.join(theme)];

    for theme_dir in &theme_dirs {
        for subdir in SUBDIRS {
            let dir = if *subdir == "." {
                theme_dir.clone()
            } else {
                theme_dir.join(subdir)
            };

            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if EXTENSIONS.contains(&ext) {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                if seen.insert(stem.to_string()) {
                                    sounds.push((stem.to_string(), path));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    sounds.sort_by(|a, b| a.0.cmp(&b.0));
    sounds
}

/// Check if a directory (or its stereo/ subdir) contains any sound files.
fn has_sound_files(path: &Path) -> bool {
    for subdir in &["stereo", "."] {
        let dir = if *subdir == "." {
            path.to_path_buf()
        } else {
            path.join(subdir)
        };

        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if EXTENSIONS.contains(&ext) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Read the display name from a theme's index.theme file.
fn read_theme_name(theme_dir: &Path) -> Option<String> {
    let index_path = theme_dir.join("index.theme");
    let contents = std::fs::read_to_string(index_path).ok()?;

    for line in contents.lines() {
        if let Some(name) = line.strip_prefix("Name=") {
            let name = name.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }

    None
}
