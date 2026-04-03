mod app;
mod firmware;
mod scpi;
mod serial;
mod ui;

use iced::{Application, Settings};

fn main() -> iced::Result {
    // Platform check: only Windows is supported in this release.
    // On other platforms the app still compiles and launches but shows a notice.

    static ICON_BYTES: &[u8] = include_bytes!("../assets/icon/nexperia_x_logo_light.ico");
    let window_icon = iced::window::icon::from_file_data(ICON_BYTES, None).ok();

    app::NevcApp::run(Settings {
        window: iced::window::Settings {
            exit_on_close_request: false,
            icon: window_icon,
            ..Default::default()
        },
        ..Settings::default()
    })
}
