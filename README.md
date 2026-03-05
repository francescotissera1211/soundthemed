# soundthemed

A lightweight daemon that plays [freedesktop sound theme](https://www.freedesktop.org/wiki/Specifications/sound-theme-spec/) sounds in response to system events. Built for Wayland desktops and compositors that don't ship their own event sounds.

## What it does

- **USB hotplug** — plays `device-added` / `device-removed` when you plug or unplug a USB device
- **Charger events** — plays `power-plug` / `power-unplug` instantly when the AC adapter state changes
- **Battery warnings** — plays `battery-low` and `battery-caution` when battery drops below configurable thresholds

All events are detected via udev (instant, no polling for USB/charger). Battery percentage is polled once per minute.

## How it works

1. Detects system events (udev for USB and power, sysfs polling for battery level)
2. Resolves the event to a sound file from your active freedesktop sound theme
3. Plays it with `pw-play` (PipeWire)

Sound theme is read from gsettings (`org.gnome.desktop.sound` → `theme-name`). If gsettings isn't available, falls back to a config file or the `freedesktop` default theme.

## Requirements

- Linux with udev
- PipeWire (`pw-play` in PATH)
- A freedesktop-compatible sound theme installed (most distros ship one)

## Building

```sh
cargo build --release
```

The binary is at `target/release/soundthemed`.

## Usage

```sh
# Run in foreground
soundthemed

# With debug logging
RUST_LOG=debug soundthemed
```

Designed to be started by your compositor's autostart, a systemd user unit, or similar.

### Systemd user service

```ini
# ~/.config/systemd/user/soundthemed.service
[Unit]
Description=Freedesktop sound theme daemon
After=pipewire.service

[Service]
ExecStart=/path/to/soundthemed
Restart=on-failure

[Install]
WantedBy=default.target
```

```sh
systemctl --user enable --now soundthemed
```

## Configuration

Optionally create `~/.config/soundthemed/config.toml`:

```toml
# Sound theme name (overridden by gsettings if available)
theme = "ocean"

# Enable/disable all sounds
enabled = true

# Battery warning thresholds (percentage)
battery_low_percent = 15
battery_critical_percent = 5
```

gsettings takes priority over the config file for the theme name, so if you're on GNOME or a gsettings-compatible desktop, it'll pick up your system theme automatically.

## Sound theme lookup

Sounds are resolved following the freedesktop spec search order:

1. `~/.local/share/sounds/<theme>/stereo/`
2. `/usr/share/sounds/<theme>/stereo/`
3. Fallback to `freedesktop` theme if the configured theme doesn't have the sound
4. Supported formats: `.oga`, `.ogg`, `.wav`

## License

MIT
