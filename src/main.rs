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

    static UBUNTU_FONT: &[u8] = include_bytes!("../assets/fonts/Ubuntu-Regular.ttf");
    static UBUNTU_BOLD: &[u8] = include_bytes!("../assets/fonts/Ubuntu-Bold.ttf");

    app::NevcApp::run(Settings {
        fonts: vec![
            std::borrow::Cow::Borrowed(UBUNTU_FONT),
            std::borrow::Cow::Borrowed(UBUNTU_BOLD),
        ],
        default_font: iced::Font::with_name("Ubuntu"),
        window: iced::window::Settings {
            exit_on_close_request: false,
            icon: window_icon,
            ..Default::default()
        },
        ..Settings::default()
    })
}
