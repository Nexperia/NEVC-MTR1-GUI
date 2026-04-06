use iced::widget::{column, scrollable, text};
use iced::{Element, Length};

use crate::app::{Message, NevcApp};
use crate::serial::ConnectionState;

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    if app.connection != ConnectionState::Connected {
        return column![
            iced::widget::Space::with_height(20),
            text("Not connected - connect to the board to view firmware configuration.").size(14),
        ]
        .spacing(0)
        .into();
    }

    // Stage 6 will render a full editable config table parsed from the IDN serial field.
    let serial_info: Element<Message> = if let Some(serial) = &app.idn_serial {
        let fields: Vec<&str> = serial.split('-').collect();
        let mut field_col = column![
            text("IDN Serial Field (26 hex values):").size(14),
            iced::widget::Space::with_height(6),
        ]
        .spacing(2);

        for (i, f) in fields.iter().enumerate() {
            field_col = field_col.push(
                text(format!("  [{:02}] 0x{}", i, f)).size(12),
            );
        }
        field_col.into()
    } else {
        text("IDN serial field not yet received.").size(13).into()
    };

    let content = column![
        text("Firmware Constants").size(18),
        iced::widget::Space::with_height(8),
        serial_info,
        iced::widget::Space::with_height(16),
        text("Full constant mapping and editable fields - Stage 6.").size(13),
    ]
    .spacing(0)
    .max_width(700);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
