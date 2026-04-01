use iced::widget::{button, column, scrollable, text};
use iced::{Element, Length};

use crate::app::{Message, NevcApp};
use crate::serial::ConnectionState;

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    // -----------------------------------------------------------------------
    // Firmware version section
    // -----------------------------------------------------------------------
    let version_section: Element<Message> = if app.connection == ConnectionState::Connected {
        let fw = app.firmware_version.as_deref().unwrap_or("unknown");
        column![
            text("Current Firmware").size(16),
            text(format!("Version: {}", fw)).size(14),
        ]
        .spacing(4)
        .into()
    } else {
        column![
            text("Current Firmware").size(16),
            text("Connect to the device to read the firmware version.").size(13),
        ]
        .spacing(4)
        .into()
    };

    // -----------------------------------------------------------------------
    // Flash firmware button
    // -----------------------------------------------------------------------
    let can_flash = app.connection == ConnectionState::Connected
        || app
            .available_ports
            .iter()
            .any(|p| p.is_arduino);

    let flash_btn: Element<Message> = {
        let mut b = button(text("Flash Latest Firmware").size(14))
            .style(iced::theme::Button::Primary)
            .padding([8, 18]);
        if can_flash {
            b = b.on_press(Message::FlashFirmwarePressed);
        }
        b.into()
    };

    // -----------------------------------------------------------------------
    // Flash log
    // -----------------------------------------------------------------------
    let log_entries: Vec<Element<Message>> = app
        .flash_log
        .iter()
        .map(|entry| text(entry.as_str()).size(12).into())
        .collect();

    let log_section: Element<Message> = if log_entries.is_empty() {
        text("Flash log will appear here.").size(12).into()
    } else {
        column(log_entries).spacing(2).into()
    };

    // -----------------------------------------------------------------------
    // avrdude availability notice
    // -----------------------------------------------------------------------
    let avrdude_notice = column![
        text("Requirements").size(15),
        text("• avrdude.exe — bundled alongside the application, or install Arduino IDE/CLI.").size(12),
        text("• firmware.hex — bundled alongside the application.").size(12),
        text("• Arduino Leonardo connected via USB.").size(12),
    ]
    .spacing(3);

    // -----------------------------------------------------------------------
    // Compose
    // -----------------------------------------------------------------------
    let content = column![
        text("Firmware").size(24),
        iced::widget::Space::with_height(16),
        version_section,
        iced::widget::Space::with_height(20),
        text("Upload Firmware").size(18),
        iced::widget::Space::with_height(6),
        avrdude_notice,
        iced::widget::Space::with_height(10),
        flash_btn,
        iced::widget::Space::with_height(16),
        text("Log:").size(13),
        iced::widget::Space::with_height(4),
        log_section,
    ]
    .spacing(0)
    .max_width(700);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
