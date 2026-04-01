use iced::widget::{button, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Element, Length};

use crate::app::{Message, NevcApp};
use crate::serial::ConnectionState;

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    // -----------------------------------------------------------------------
    // Port picker
    // -----------------------------------------------------------------------

    // Build display strings to show in the pick_list.
    // We store just the port name in `selected_port` and build the display.
    let port_display_strings: Vec<String> = app
        .available_ports
        .iter()
        .map(|p| {
            if p.is_arduino {
                format!("{} — Arduino Leonardo", p.name)
            } else {
                format!("{} — {}", p.name, p.description)
            }
        })
        .collect();

    // Reconstruct the selected display string from the stored port name.
    let selected_display: Option<String> = app.selected_port.as_ref().and_then(|name| {
        app.available_ports
            .iter()
            .zip(port_display_strings.iter())
            .find(|(p, _)| &p.name == name)
            .map(|(_, display)| display.clone())
    });

    let port_widget: Element<Message> = if port_display_strings.is_empty() {
        text("No COM ports detected.").size(14).into()
    } else {
        pick_list(
            port_display_strings,
            selected_display,
            Message::PortSelected,
        )
        .placeholder("Select a COM port…")
        .into()
    };

    let refresh_btn = button(text("Refresh").size(13))
        .on_press(Message::RefreshPorts)
        .padding([5, 10]);

    // -----------------------------------------------------------------------
    // Connect / disconnect button
    // -----------------------------------------------------------------------
    let is_connected = app.connection == ConnectionState::Connected;
    let is_connecting = app.connection == ConnectionState::Connecting;
    let can_connect = app.selected_port.is_some() && !is_connected && !is_connecting;

    let connect_btn: Element<Message> = if is_connected {
        button(text("Disconnect").size(13))
            .on_press(Message::DisconnectPressed)
            .style(iced::theme::Button::Destructive)
            .padding([5, 14])
            .into()
    } else if is_connecting {
        button(text("Connecting…").size(13))
            .padding([5, 14])
            .into()
    } else {
        let mut b = button(text("Connect").size(13))
            .style(iced::theme::Button::Primary)
            .padding([5, 14]);
        if can_connect {
            b = b.on_press(Message::ConnectPressed);
        }
        b.into()
    };

    let controls = row![port_widget, refresh_btn, connect_btn]
        .spacing(8)
        .align_items(Alignment::Center);

    // -----------------------------------------------------------------------
    // Connection status
    // -----------------------------------------------------------------------
    let status_text = match app.connection {
        ConnectionState::Disconnected => "● Disconnected",
        ConnectionState::Connecting => "○ Connecting…",
        ConnectionState::Connected => "● Connected",
    };

    // -----------------------------------------------------------------------
    // Firmware info (populated after *IDN?)
    // -----------------------------------------------------------------------
    let firmware_section: Element<Message> = if app.connection == ConnectionState::Connected {
        let fw_ver = app
            .firmware_version
            .as_deref()
            .unwrap_or("(querying…)");
        let manufacturer = app.idn_manufacturer.as_deref().unwrap_or("");
        let model = app.idn_model.as_deref().unwrap_or("");
        let serial = app.idn_serial.as_deref().unwrap_or("");

        column![
            text("Device Information").size(16),
            text(format!("Manufacturer : {}", manufacturer)).size(13),
            text(format!("Model        : {}", model)).size(13),
            text(format!("Firmware     : {}", fw_ver)).size(13),
            text(format!("Serial field : {}", serial)).size(13),
        ]
        .spacing(4)
        .into()
    } else {
        text("Connect to a device to view firmware information.").size(13).into()
    };

    // -----------------------------------------------------------------------
    // Detected Arduino ports summary
    // -----------------------------------------------------------------------
    let arduino_ports: Vec<&str> = app
        .available_ports
        .iter()
        .filter(|p| p.is_arduino)
        .map(|p| p.name.as_str())
        .collect();

    let arduino_info: Element<Message> = if arduino_ports.is_empty() {
        column![
            text("No Arduino Leonardo detected.").size(13),
            text("Tips:").size(13),
            text("  • Connect the board via USB and click Refresh.").size(12),
            text("  • Verify the USB driver is installed (Windows Device Manager).").size(12),
            text("  • Press the reset button on the board.").size(12),
            text("  • For firmware upload the board uses AVR109 (1200-baud reset trick).").size(12),
        ]
        .spacing(3)
        .into()
    } else {
        text(format!(
            "Arduino Leonardo detected on: {}",
            arduino_ports.join(", ")
        ))
        .size(13)
        .into()
    };

    // -----------------------------------------------------------------------
    // Compose the panel
    // -----------------------------------------------------------------------
    let content = column![
        text("Connection").size(24),
        iced::widget::Space::with_height(16),
        text("COM Port").size(14),
        iced::widget::Space::with_height(6),
        controls,
        iced::widget::Space::with_height(8),
        text(status_text).size(15),
        iced::widget::Space::with_height(24),
        firmware_section,
        iced::widget::Space::with_height(20),
        arduino_info,
    ]
    .spacing(0)
    .max_width(700);

    scrollable(
        container(content).width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
