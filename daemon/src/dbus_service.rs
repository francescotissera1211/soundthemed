//! D-Bus service for soundthemed.
//!
//! Exposes org.freedesktop.SoundThemed1 on the session bus,
//! allowing external apps/scripts to request sound playback
//! by freedesktop event ID.

use soundthemed_shared::sound_ids::SoundEvent;
use std::sync::Arc;
use tokio::sync::mpsc;
use zbus::object_server::SignalEmitter;
use zbus::{interface, Connection};

pub struct SoundThemedService {
    tx: mpsc::Sender<SoundEvent>,
    theme: Arc<std::sync::RwLock<String>>,
    enabled: Arc<std::sync::RwLock<bool>>,
}

impl SoundThemedService {
    pub fn new(
        tx: mpsc::Sender<SoundEvent>,
        theme: Arc<std::sync::RwLock<String>>,
        enabled: Arc<std::sync::RwLock<bool>>,
    ) -> Self {
        Self { tx, theme, enabled }
    }
}

#[interface(name = "org.freedesktop.SoundThemed1")]
impl SoundThemedService {
    /// Play a sound by its freedesktop event ID (fire-and-forget).
    async fn play_sound(&self, event_id: String) {
        log::info!("dbus: PlaySound({event_id})");
        let _ = self.tx.send(SoundEvent::Custom(event_id)).await;
    }

    /// Play a sound and return whether the sound file was found.
    async fn play_sound_sync(&self, event_id: String) -> bool {
        let theme = self.theme.read().unwrap().clone();
        let exists = soundthemed_shared::theme::resolve(&theme, &event_id).is_some();
        if exists {
            let _ = self.tx.send(SoundEvent::Custom(event_id)).await;
        }
        exists
    }

    /// Check if a sound exists in the current theme.
    fn has_sound(&self, event_id: String) -> bool {
        let theme = self.theme.read().unwrap().clone();
        soundthemed_shared::theme::resolve(&theme, &event_id).is_some()
    }

    /// Get the current theme name.
    fn get_theme(&self) -> String {
        self.theme.read().unwrap().clone()
    }

    /// Reload configuration from disk.
    async fn reload_config(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) {
        log::info!("dbus: ReloadConfig requested");
        // The actual reload is handled by the main loop via SIGHUP.
        // Send SIGHUP to ourselves.
        unsafe {
            libc::kill(libc::getpid(), libc::SIGHUP);
        }
    }

    /// Whether sounds are globally enabled.
    #[zbus(property)]
    fn enabled(&self) -> bool {
        *self.enabled.read().unwrap()
    }

    /// The active theme name.
    #[zbus(property)]
    fn theme_name(&self) -> String {
        self.theme.read().unwrap().clone()
    }
}

/// Start the D-Bus service on the session bus.
pub async fn serve(
    tx: mpsc::Sender<SoundEvent>,
    theme: Arc<std::sync::RwLock<String>>,
    enabled: Arc<std::sync::RwLock<bool>>,
) {
    if let Err(e) = serve_inner(tx, theme, enabled).await {
        log::error!("D-Bus service error: {e}");
    }
}

async fn serve_inner(
    tx: mpsc::Sender<SoundEvent>,
    theme: Arc<std::sync::RwLock<String>>,
    enabled: Arc<std::sync::RwLock<bool>>,
) -> zbus::Result<()> {
    let service = SoundThemedService::new(tx, theme, enabled);

    let connection = Connection::session().await?;

    connection
        .object_server()
        .at("/org/freedesktop/SoundThemed1", service)
        .await?;

    connection
        .request_name("org.freedesktop.SoundThemed1")
        .await?;

    log::info!("dbus: serving org.freedesktop.SoundThemed1 on session bus");

    // Keep the connection alive
    std::future::pending::<()>().await;

    Ok(())
}
