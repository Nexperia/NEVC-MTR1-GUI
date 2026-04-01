mod app;
mod firmware;
mod scpi;
mod serial;
mod ui;

use iced::{Application, Settings};

fn main() -> iced::Result {
    // Platform check: only Windows is supported in this release.
    // On other platforms the app still compiles and launches but shows a notice.
    app::NevcApp::run(Settings::default())
}
