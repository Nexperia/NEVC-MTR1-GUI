use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::app::{LogLevel, Message, NevcApp};

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    let clear_btn = button(text("Clear").size(13))
        .on_press(Message::ClearLog)
        .padding([4, 10]);

    let header_row = row![
        text("Log").size(24),
        iced::widget::Space::with_width(Length::Fill),
        clear_btn,
    ]
    .align_items(iced::Alignment::Center);

    let entries: Vec<Element<Message>> = if app.log.is_empty() {
        vec![text("No events yet.").size(13).into()]
    } else {
        app.log
            .iter()
            .rev() // newest first
            .map(|entry| {
                let prefix = match entry.level {
                    LogLevel::Info  => "[INFO ]",
                    LogLevel::Warn  => "[WARN ]",
                    LogLevel::Error => "[ERROR]",
                };
                row![
                    text(&entry.timestamp).size(12).width(Length::Fixed(62.0)),
                    text(prefix).size(12).width(Length::Fixed(60.0)),
                    text(&entry.message).size(12),
                ]
                .spacing(6)
                .into()
            })
            .collect()
    };

    let log_list = scrollable(
        container(column(entries).spacing(3).padding(4))
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    column![
        header_row,
        iced::widget::Space::with_height(10),
        log_list,
    ]
    .spacing(0)
    .into()
}
