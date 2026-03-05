//! Sound playback via pw-play (PipeWire).
//!
//! Spawns pw-play as a child process. Fire-and-forget —
//! we don't wait for playback to finish before handling
//! the next event.

use std::path::Path;
use tokio::process::Command;

/// Play a sound file using pw-play.
///
/// Spawns the process in the background and logs any errors.
/// Does not block — returns immediately after spawning.
pub async fn play(path: &Path) {
    log::info!("playing: {}", path.display());

    match Command::new("pw-play").arg(path).spawn() {
        Ok(_child) => {
            // Fire and forget — pw-play handles its own lifecycle.
            // We don't await the child because we don't want to
            // block event processing while a sound plays.
        }
        Err(e) => {
            log::error!("failed to spawn pw-play: {e}");
        }
    }
}
