//! Sound playback via pw-play (PipeWire).
//!
//! Spawns pw-play as a child process. Fire-and-forget —
//! we don't wait for playback to finish before handling
//! the next event. Multiple sounds can overlap naturally.

use std::path::Path;
use tokio::process::Command;

/// Play a sound file using pw-play.
///
/// Spawns the process in the background and logs any errors.
/// Does not block — returns immediately after spawning.
/// Multiple calls will overlap naturally (e.g. rapid volume key presses).
pub async fn play(path: &Path) {
    log::info!("playing: {}", path.display());

    match Command::new("pw-play").arg(path).spawn() {
        Ok(_child) => {
            // Fire and forget — pw-play handles its own lifecycle.
        }
        Err(e) => {
            log::error!("failed to spawn pw-play: {e}");
        }
    }
}

/// Play a sound file and wait for it to finish.
/// Used for shutdown sounds where we must complete playback before exiting.
pub async fn play_and_wait(path: &Path) {
    log::info!("playing (blocking): {}", path.display());

    match Command::new("pw-play").arg(path).output().await {
        Ok(_) => {}
        Err(e) => {
            log::error!("failed to play: {e}");
        }
    }
}
