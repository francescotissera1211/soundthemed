//! Desktop notification monitoring via org.freedesktop.Notifications D-Bus.
//!
//! Watches for incoming notifications and plays appropriate sounds
//! based on urgency level:
//!   - Low/Normal urgency → "message"
//!   - Critical urgency → "dialog-warning"
//!
//! Uses a dedicated D-Bus connection (not the shared session connection)
//! because BecomeMonitor converts the connection to monitor-only mode,
//! which would break the D-Bus service on the shared connection.

use futures_util::StreamExt;
use soundthemed_shared::sound_ids::SoundEvent;
use tokio::sync::mpsc;
use zbus::connection::Builder;
use zbus::message::Body;

/// Start watching for desktop notifications.
pub async fn watch(tx: mpsc::Sender<SoundEvent>) {
    if let Err(e) = watch_inner(tx).await {
        log::error!("notifications monitor error: {e}");
    }
}

async fn watch_inner(tx: mpsc::Sender<SoundEvent>) -> zbus::Result<()> {
    // Create a dedicated private connection — do NOT use Connection::session()
    // which shares a connection that the D-Bus service also uses.
    let connection = Builder::session()?
        .internal_executor(false)
        .build()
        .await?;

    // Spawn the internal executor for this connection
    let conn_clone = connection.clone();
    tokio::spawn(async move {
        loop {
            conn_clone.executor().tick().await;
        }
    });

    // Monitor the Notify method calls on the notifications bus.
    let rule = "type='method_call',interface='org.freedesktop.Notifications',member='Notify'";

    // BecomeMonitor on this dedicated connection only
    connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus.Monitoring"),
            "BecomeMonitor",
            &(vec![rule], 0u32),
        )
        .await?;

    log::info!("notifications: monitoring desktop notifications");

    let mut stream = zbus::MessageStream::from(&connection);

    while let Some(msg) = stream.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => continue,
        };

        let body: Body = msg.body();
        if let Ok((
            _app_name,
            _replaces_id,
            _icon,
            _summary,
            _body_text,
            _actions,
            hints,
            _timeout,
        )) = body.deserialize::<(
            String,
            u32,
            String,
            String,
            String,
            Vec<String>,
            std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
            i32,
        )>() {
            let urgency: u8 = hints
                .get("urgency")
                .and_then(|v| u8::try_from(v).ok())
                .unwrap_or(1);

            let suppress = hints
                .get("suppress-sound")
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false);

            if suppress {
                continue;
            }

            let event = match urgency {
                2 => SoundEvent::Custom("dialog-warning".into()),
                _ => SoundEvent::Custom("message".into()),
            };

            log::debug!(
                "notification: app={}, urgency={urgency}",
                _app_name
            );

            if tx.send(event).await.is_err() {
                break;
            }
        }
    }

    Ok(())
}
