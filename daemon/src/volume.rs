//! Audio volume change monitoring via WirePlumber (wpctl).
//!
//! Polls the default sink volume periodically and fires an event
//! when the volume changes, with debouncing to avoid spamming
//! during continuous adjustment (e.g. holding a volume key or
//! turning a dial). Fires once when a change is first detected,
//! then suppresses until the volume stabilises.

use soundthemed_shared::sound_ids::SoundEvent;
use tokio::sync::mpsc;
use tokio::time::{self, Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(20);
const COOLDOWN: Duration = Duration::from_millis(20);

/// Start watching for volume changes.
pub async fn watch(tx: mpsc::Sender<SoundEvent>) {
    if let Err(e) = watch_inner(tx).await {
        log::error!("volume monitor error: {e}");
    }
}

async fn watch_inner(tx: mpsc::Sender<SoundEvent>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("volume: watching via wpctl polling");

    let mut last_volume = get_volume().await;
    let mut interval = time::interval(POLL_INTERVAL);
    let mut last_sound = Instant::now() - COOLDOWN; // allow first sound immediately

    // Skip the first tick
    interval.tick().await;

    loop {
        interval.tick().await;

        let current = get_volume().await;
        if current != last_volume {
            last_volume = current;

            // Only play if enough time has passed and no media is playing
            let now = Instant::now();
            if now.duration_since(last_sound) >= COOLDOWN && !is_media_playing().await {
                last_sound = now;
                log::debug!("volume: change detected, playing sound");
                if tx.send(SoundEvent::AudioVolumeChange).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Check if any media player is currently playing.
async fn is_media_playing() -> bool {
    let output = tokio::process::Command::new("playerctl")
        .args(["-a", "status"])
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let statuses = String::from_utf8_lossy(&o.stdout);
            statuses.lines().any(|line| line.trim() == "Playing")
        }
        _ => false,
    }
}

/// Get the current default sink volume level.
async fn get_volume() -> Option<String> {
    let output = tokio::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8(output.stdout).ok()?;
    text.strip_prefix("Volume: ")
        .map(|s| s.trim().to_string())
}
