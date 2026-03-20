//! soundthemed-config — GTK4/Libadwaita configuration GUI
//!
//! Provides a preferences window for:
//!   - Selecting the active sound theme
//!   - Overriding individual event sounds (default / none / custom)
//!   - Previewing sounds
//!   - Creating new themes from audio folders

mod theme_creator_ui;
mod window;

use adw::prelude::*;

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let app = adw::Application::builder()
        .application_id("org.freedesktop.SoundThemed.Config")
        .build();

    app.connect_activate(|app| {
        window::build(app);
    });

    app.run();
}
