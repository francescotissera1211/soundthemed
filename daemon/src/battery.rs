//! Battery level monitoring via /sys/class/power_supply.
//!
//! Polls battery percentage periodically and fires events
//! for low and critical levels. Charge state changes
//! (plug/unplug) are handled instantly by the udev monitor
//! instead, so this module only watches the percentage.

use soundthemed_shared::sound_ids::SoundEvent;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

const POLL_INTERVAL: Duration = Duration::from_secs(60);
const POWER_SUPPLY_DIR: &str = "/sys/class/power_supply";

/// Find the first battery in /sys/class/power_supply.
fn find_battery() -> Option<PathBuf> {
    let entries = std::fs::read_dir(POWER_SUPPLY_DIR).ok()?;

    for entry in entries.flatten() {
        let type_path = entry.path().join("type");
        if let Ok(psu_type) = std::fs::read_to_string(&type_path) {
            if psu_type.trim() == "Battery" {
                return Some(entry.path());
            }
        }
    }

    None
}

/// Read a sysfs file and return its trimmed contents.
fn read_sysfs(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Parse the battery charge percentage.
fn read_capacity(battery_path: &Path) -> Option<u8> {
    read_sysfs(&battery_path.join("capacity"))?.parse().ok()
}

/// Check if the battery is currently discharging.
fn is_discharging(battery_path: &Path) -> bool {
    matches!(
        read_sysfs(&battery_path.join("status")).as_deref(),
        Some("Discharging") | Some("Not charging")
    )
}

/// Start polling battery level.
///
/// Fires events when battery drops below low or critical
/// thresholds while discharging. Resets when battery
/// recovers above the threshold.
pub async fn watch(tx: mpsc::Sender<SoundEvent>, low_pct: u8, crit_pct: u8) {
    let battery_path = match find_battery() {
        Some(p) => {
            log::info!("battery: monitoring {}", p.display());
            p
        }
        None => {
            log::info!("battery: no battery found, skipping monitor");
            return;
        }
    };

    let mut fired_low = false;
    let mut fired_critical = false;
    let mut interval = time::interval(POLL_INTERVAL);

    loop {
        interval.tick().await;

        if !is_discharging(&battery_path) {
            // Reset when charging
            fired_low = false;
            fired_critical = false;
            continue;
        }

        let pct = match read_capacity(&battery_path) {
            Some(p) => p,
            None => continue,
        };

        if pct <= crit_pct && !fired_critical {
            log::warn!("battery: critical ({pct}%)");
            fired_critical = true;
            if tx.send(SoundEvent::BatteryCritical).await.is_err() {
                break;
            }
        } else if pct <= low_pct && !fired_low {
            log::warn!("battery: low ({pct}%)");
            fired_low = true;
            if tx.send(SoundEvent::BatteryLow).await.is_err() {
                break;
            }
        }

        // Reset if recovered
        if pct > low_pct {
            fired_low = false;
            fired_critical = false;
        }
    }
}
