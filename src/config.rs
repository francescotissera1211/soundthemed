//! Configuration — gsettings with config file fallback
//!
//! Checks gsettings for the active sound theme first.
//! If gsettings isn't available (no GNOME, no dbus, etc),
//! falls back to ~/.config/soundthemed/config.toml.

use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

/// Config file structure
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Sound theme name (e.g. "freedesktop", "ocean", "ubuntu")
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Whether sounds are enabled globally
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Battery percentage to trigger low warning
    #[serde(default = "default_battery_low")]
    pub battery_low_percent: u8,

    /// Battery percentage to trigger critical warning
    #[serde(default = "default_battery_critical")]
    pub battery_critical_percent: u8,
}

fn default_theme() -> String {
    "freedesktop".into()
}

fn default_enabled() -> bool {
    true
}

fn default_battery_low() -> u8 {
    15
}

fn default_battery_critical() -> u8 {
    5
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            enabled: default_enabled(),
            battery_low_percent: default_battery_low(),
            battery_critical_percent: default_battery_critical(),
        }
    }
}

/// Try to get the sound theme name from gsettings.
/// Returns None if gsettings isn't available or fails.
fn theme_from_gsettings() -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.sound", "theme-name"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    // gsettings wraps the value in single quotes: 'freedesktop'
    let trimmed = raw.trim().trim_matches('\'');

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Path to the config file: ~/.config/soundthemed/config.toml
fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("soundthemed")
        .join("config.toml")
}

/// Load config. Tries gsettings first for the theme name,
/// then falls back to the config file, then defaults.
pub fn load() -> Config {
    let mut config = if let Ok(contents) = std::fs::read_to_string(config_path()) {
        match toml::from_str::<Config>(&contents) {
            Ok(c) => {
                log::info!("loaded config from {}", config_path().display());
                c
            }
            Err(e) => {
                log::warn!("failed to parse config: {e}, using defaults");
                Config::default()
            }
        }
    } else {
        log::info!("no config file found, using defaults");
        Config::default()
    };

    // gsettings overrides config file theme if available
    if let Some(theme) = theme_from_gsettings() {
        log::info!("gsettings sound theme: {theme}");
        config.theme = theme;
    }

    log::info!("active theme: {}", config.theme);
    config
}
