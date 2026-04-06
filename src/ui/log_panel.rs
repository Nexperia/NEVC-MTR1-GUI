use iced::widget::{button, column, row, text, text_editor};
use iced::{Element, Length};

use crate::app::{Message, NevcApp};

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    let clear_btn = button(text("Clear").size(13))
        .on_press(Message::ClearLog)
        .style(iced::theme::Button::Secondary)
        .padding([4, 10]);

    let download_btn = button(text("Download Log").size(13))
        .on_press(Message::DownloadLog)
        .style(iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)))
        .padding([4, 10]);

    let header_row = row![
        iced::widget::Space::with_width(Length::Fill),
        download_btn,
        iced::widget::Space::with_width(8),
        clear_btn,
    ]
    .align_items(iced::Alignment::Center);

    let editor = text_editor(&app.log_content)
        .on_action(Message::LogAction)
        .font(iced::Font::MONOSPACE)
        .height(Length::Fill);

    column![
        header_row,
        iced::widget::Space::with_height(10),
        editor,
    ]
    .spacing(0)
    .into()
}
