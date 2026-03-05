//! Sound event definitions — maps system events to
//! freedesktop sound theme IDs.

/// A sound event that should trigger playback.
#[derive(Debug, Clone)]
pub enum SoundEvent {
    /// USB device plugged in
    DeviceAdded,
    /// USB device unplugged
    DeviceRemoved,
    /// Power cable plugged in
    PowerPlug,
    /// Power cable unplugged
    PowerUnplug,
    /// Battery level is low
    BatteryLow,
    /// Battery level is critical
    BatteryCritical,
}

impl SoundEvent {
    /// Get the freedesktop sound theme ID for this event.
    pub fn sound_id(&self) -> &'static str {
        match self {
            Self::DeviceAdded => "device-added",
            Self::DeviceRemoved => "device-removed",
            Self::PowerPlug => "power-plug",
            Self::PowerUnplug => "power-unplug",
            Self::BatteryLow => "battery-low",
            Self::BatteryCritical => "battery-caution",
        }
    }
}
