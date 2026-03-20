//! Niri compositor event monitoring.
//!
//! Watches `niri msg event-stream` for:
//!   - Window urgency changes (bell) — plays "bell" when a window becomes urgent

use soundthemed_shared::sound_ids::SoundEvent;
use std::collections::HashSet;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Start watching Niri compositor events.
pub async fn watch(tx: mpsc::Sender<SoundEvent>) {
    if let Err(e) = watch_inner(tx).await {
        log::error!("niri monitor error: {e}");
    }
}

async fn watch_inner(tx: mpsc::Sender<SoundEvent>) -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("XDG_CURRENT_DESKTOP").as_deref() != Ok("niri") {
        log::info!("niri: not running under niri, skipping");
        return Ok(());
    }

    let mut child = Command::new("niri")
        .args(["msg", "event-stream"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    log::info!("niri: watching compositor events");

    let stdout = child.stdout.take().ok_or("no stdout from niri msg")?;
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    // Track which windows are currently urgent so we only fire on transitions
    let mut urgent_windows: HashSet<u64> = HashSet::new();
    // Skip the initial dump of events (first few lines within 1 second of startup)
    let start = tokio::time::Instant::now();
    let warmup = tokio::time::Duration::from_secs(2);

    while let Some(line) = lines.next_line().await? {
        // Only process single-window change events, not bulk "Windows changed:" dumps
        if !line.starts_with("Window opened or changed:") {
            continue;
        }

        // Skip initial event dump on startup
        if start.elapsed() < warmup {
            continue;
        }

        // Parse: "Window opened or changed: Window { id: 123, ... is_urgent: true, ... }"
        // Extract "id: N" — but only the FIRST one (the window id), not workspace_id etc.
        let id = match line.find("Window { id: ") {
            Some(pos) => {
                let after = &line[pos + 13..];
                let id_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                match id_str.parse::<u64>() {
                    Ok(n) => n,
                    Err(_) => continue,
                }
            }
            None => continue,
        };

        // Extract is_urgent
        let is_urgent = line.contains("is_urgent: true");

        if is_urgent && !urgent_windows.contains(&id) {
            urgent_windows.insert(id);
            log::info!("niri: window {id} became urgent (bell)");
            if tx.send(SoundEvent::Custom("bell".into())).await.is_err() {
                return Ok(());
            }
        } else if !is_urgent {
            urgent_windows.remove(&id);
        }
    }

    Ok(())
}
