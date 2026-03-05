//! System event monitoring via udev.
//!
//! Watches for:
//!   - USB device add/remove events
//!   - Power supply (charger) plug/unplug events
//!
//! Runs on a dedicated OS thread because libudev types
//! contain raw pointers and aren't Send. Uses poll() on
//! the monitor fd to block until events arrive.

use crate::events::SoundEvent;
use std::os::unix::io::AsRawFd;
use tokio::sync::mpsc;
use udev::MonitorBuilder;

/// Start watching for udev events (USB + power supply).
///
/// Spawns a blocking OS thread that monitors udev and
/// sends events to the provided async channel.
pub fn spawn(tx: mpsc::Sender<SoundEvent>) {
    std::thread::Builder::new()
        .name("udev-monitor".into())
        .spawn(move || watch_blocking(tx))
        .expect("failed to spawn udev monitor thread");
}

/// Blocking udev event loop. Runs on its own OS thread.
fn watch_blocking(tx: mpsc::Sender<SoundEvent>) {
    let socket = match MonitorBuilder::new()
        .and_then(|b| b.match_subsystem("usb"))
        .and_then(|b| b.match_subsystem("power_supply"))
        .and_then(|b| b.listen())
    {
        Ok(s) => s,
        Err(e) => {
            log::error!("failed to start udev listener: {e}");
            return;
        }
    };

    log::info!("udev: watching for USB and power_supply events");

    let fd = socket.as_raw_fd();

    loop {
        // Block until the udev socket has data to read
        let mut pollfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };

        let ret = unsafe { libc::poll(&mut pollfd as *mut _, 1, -1) };

        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            log::error!("poll error: {err}");
            break;
        }

        if ret == 0 {
            continue;
        }

        // Drain all available events
        for event in socket.iter() {
            let subsystem = event
                .subsystem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            match subsystem {
                "usb" => handle_usb(&event, &tx),
                "power_supply" => handle_power_supply(&event, &tx),
                _ => {}
            }
        }
    }
}

/// Handle a USB device event.
fn handle_usb(event: &udev::Event, tx: &mpsc::Sender<SoundEvent>) {
    let is_device = event
        .devtype()
        .map(|d| d.to_str() == Some("usb_device"))
        .unwrap_or(false);

    if !is_device {
        return;
    }

    let sound = match event.event_type() {
        udev::EventType::Add => {
            log::info!("udev: USB device added");
            SoundEvent::DeviceAdded
        }
        udev::EventType::Remove => {
            log::info!("udev: USB device removed");
            SoundEvent::DeviceRemoved
        }
        _ => return,
    };

    let _ = tx.blocking_send(sound);
}

/// Handle a power_supply change event (charger plug/unplug).
fn handle_power_supply(event: &udev::Event, tx: &mpsc::Sender<SoundEvent>) {
    // Only care about change events on Mains type supplies
    if event.event_type() != udev::EventType::Change {
        return;
    }

    let psu_type = event
        .property_value("POWER_SUPPLY_TYPE")
        .and_then(|v| v.to_str())
        .unwrap_or("");

    if psu_type != "Mains" {
        return;
    }

    let online = event
        .property_value("POWER_SUPPLY_ONLINE")
        .and_then(|v| v.to_str())
        .unwrap_or("");

    let sound = match online {
        "1" => {
            log::info!("power: charger plugged in");
            SoundEvent::PowerPlug
        }
        "0" => {
            log::info!("power: charger unplugged");
            SoundEvent::PowerUnplug
        }
        _ => return,
    };

    let _ = tx.blocking_send(sound);
}
