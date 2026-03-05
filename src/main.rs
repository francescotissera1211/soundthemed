//! soundthemed — Freedesktop sound theme daemon
//!
//! Plays event sounds for USB hotplug, battery state changes,
//! and more. Resolves sounds from the active freedesktop sound
//! theme (via gsettings or config file fallback).
//!
//! Usage:
//!   soundthemed              # run in foreground
//!   RUST_LOG=debug soundthemed  # with debug logging

mod battery;
mod config;
mod events;
mod player;
mod theme;
mod udev_monitor;

use events::SoundEvent;
use tokio::signal;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    log::info!("soundthemed starting");

    let config = config::load();

    if !config.enabled {
        log::info!("sounds disabled in config, exiting");
        return;
    }

    let theme = config.theme.clone();

    // Channel for all event sources to send sound events
    let (tx, mut rx) = mpsc::channel::<SoundEvent>(32);

    // Spawn event watchers
    let udev_tx = tx.clone();
    udev_monitor::spawn(udev_tx);

    let battery_tx = tx.clone();
    let low = config.battery_low_percent;
    let crit = config.battery_critical_percent;
    tokio::spawn(async move {
        battery::watch(battery_tx, low, crit).await;
    });

    // Drop the original sender so the channel closes
    // when all watchers are done
    drop(tx);

    log::info!("soundthemed running (theme: {theme})");

    // Main event loop
    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                let id = event.sound_id();
                log::debug!("event: {id}");

                if let Some(path) = theme::resolve(&theme, id) {
                    player::play(&path).await;
                } else {
                    log::warn!("no sound file for: {id}");
                }
            }
            _ = signal::ctrl_c() => {
                log::info!("shutting down");
                break;
            }
        }
    }
}
