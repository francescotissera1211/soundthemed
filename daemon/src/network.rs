//! Network connectivity monitoring via NetworkManager D-Bus.
//!
//! Subscribes to NetworkManager's StateChanged signal on the system bus.
//! Fires network-connectivity-established / network-connectivity-lost events.

use futures_util::StreamExt;
use soundthemed_shared::sound_ids::SoundEvent;
use tokio::sync::mpsc;
use zbus::Connection;

/// NetworkManager connectivity states
const NM_STATE_CONNECTED_GLOBAL: u32 = 70;
const NM_STATE_DISCONNECTED: u32 = 20;

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(signal)]
    fn state_changed(&self, state: u32);

    #[zbus(property, name = "State")]
    fn connectivity_state(&self) -> zbus::Result<u32>;
}

/// Start watching NetworkManager connectivity changes.
pub async fn watch(tx: mpsc::Sender<SoundEvent>) {
    if let Err(e) = watch_inner(tx).await {
        log::error!("network monitor error: {e}");
    }
}

async fn watch_inner(tx: mpsc::Sender<SoundEvent>) -> zbus::Result<()> {
    let connection = Connection::system().await?;
    let proxy = NetworkManagerProxy::new(&connection).await?;

    log::info!("network: watching NetworkManager state changes");

    let mut was_connected = match proxy.connectivity_state().await {
        Ok(state) => {
            log::debug!("network: initial state = {state}");
            state >= NM_STATE_CONNECTED_GLOBAL
        }
        Err(_) => false,
    };

    let mut stream = proxy.receive_state_changed().await?;

    while let Some(signal) = stream.next().await {
        if let Ok(args) = signal.args() {
            let state = args.state;
            let now_connected = state >= NM_STATE_CONNECTED_GLOBAL;
            let now_disconnected = state <= NM_STATE_DISCONNECTED;

            if now_connected && !was_connected {
                log::info!("network: connected (state {state})");
                was_connected = true;
                if tx.send(SoundEvent::NetworkConnected).await.is_err() {
                    break;
                }
            } else if now_disconnected && was_connected {
                log::info!("network: disconnected (state {state})");
                was_connected = false;
                if tx.send(SoundEvent::NetworkDisconnected).await.is_err() {
                    break;
                }
            } else {
                was_connected = now_connected || (!now_disconnected && was_connected);
            }
        }
    }

    Ok(())
}
