use iced::widget::{column, scrollable, text};
use iced::{Element, Length};

use crate::app::{Message, NevcApp};
use crate::serial::ConnectionState;

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    if app.connection != ConnectionState::Connected {
        return column![
            text("Graphs").size(24),
            iced::widget::Space::with_height(20),
            text("Not connected — connect to the board to enable live graphing.").size(14),
        ]
        .spacing(0)
        .into();
    }

    // Stage 4 will replace this with a canvas-based live plot.
    let placeholder = column![
        text("Graphs").size(24),
        iced::widget::Space::with_height(16),
        text("Live graphing — Stage 4").size(18),
        iced::widget::Space::with_height(12),
        text("Variables that will be selectable:").size(14),
        text("  • Speed (RPM)").size(13),
        text("  • Bus current IBUS (A)").size(13),
        text("  • Phase U current IPHU (A)").size(13),
        text("  • Phase V current IPHV (A)").size(13),
        text("  • Phase W current IPHW (A)").size(13),
        text("  • Duty cycle (%)").size(13),
        text("  • Gate voltage (V)").size(13),
        iced::widget::Space::with_height(12),
        text("Configurable poll rate: 1–50 Hz").size(13),
        iced::widget::Space::with_height(12),
        text("(Implementation in Stage 4)").size(12),
    ]
    .spacing(3)
    .max_width(600);

    scrollable(placeholder)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
