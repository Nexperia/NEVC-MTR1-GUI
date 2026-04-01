use iced::widget::{button, column, row, scrollable, text, text_input, slider};
use iced::{Alignment, Element, Length};

use crate::app::{Direction, Message, NevcApp};
use crate::serial::ConnectionState;
use crate::scpi::{FREQ_MIN_HZ, FREQ_MAX_HZ};

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    if app.connection != ConnectionState::Connected {
        return not_connected_view();
    }

    // -----------------------------------------------------------------------
    // Enable / Disable
    // -----------------------------------------------------------------------
    let enable_label = if app.motor_enabled {
        "Motor: ENABLED"
    } else {
        "Motor: DISABLED"
    };
    let enable_btn_style = if app.motor_enabled {
        iced::theme::Button::Destructive
    } else {
        iced::theme::Button::Positive
    };
    let enable_btn = button(text(enable_label).size(14))
        .on_press(Message::EnableChanged(!app.motor_enabled))
        .style(enable_btn_style)
        .padding([8, 20]);

    // -----------------------------------------------------------------------
    // Frequency
    // -----------------------------------------------------------------------
    let freq_slider = slider(
        FREQ_MIN_HZ as f32..=FREQ_MAX_HZ as f32,
        app.motor_frequency,
        Message::FrequencyChanged,
    )
    .step(1.0)
    .width(Length::Fixed(300.0));

    let freq_input = text_input("20000", &app.motor_frequency_input)
        .on_input(Message::FrequencyInputChanged)
        .on_submit(Message::FrequencySubmit)
        .width(Length::Fixed(90.0))
        .padding(5);

    let freq_row = row![
        text("Frequency:").size(14),
        iced::widget::Space::with_width(8),
        freq_slider,
        iced::widget::Space::with_width(8),
        freq_input,
        iced::widget::Space::with_width(4),
        text("Hz").size(14),
        iced::widget::Space::with_width(4),
        text(format!("({} – {} Hz)", FREQ_MIN_HZ, FREQ_MAX_HZ)).size(12),
    ]
    .align_items(Alignment::Center)
    .spacing(0);

    // -----------------------------------------------------------------------
    // Direction
    // -----------------------------------------------------------------------
    let fwd_style = if app.motor_direction == Direction::Forward {
        iced::theme::Button::Primary
    } else {
        iced::theme::Button::Secondary
    };
    let rev_style = if app.motor_direction == Direction::Reverse {
        iced::theme::Button::Primary
    } else {
        iced::theme::Button::Secondary
    };

    let direction_row = row![
        text("Direction:").size(14),
        iced::widget::Space::with_width(8),
        button(text("Forward").size(13))
            .on_press(Message::DirectionChanged(Direction::Forward))
            .style(fwd_style)
            .padding([5, 12]),
        button(text("Reverse").size(13))
            .on_press(Message::DirectionChanged(Direction::Reverse))
            .style(rev_style)
            .padding([5, 12]),
    ]
    .spacing(8)
    .align_items(Alignment::Center);

    // -----------------------------------------------------------------------
    // Measurements
    // -----------------------------------------------------------------------
    let meas_section = column![
        text("Measurements").size(18),
        measurement_row("Speed",          app.speed_rpm,          "RPM"),
        measurement_row("Bus current",    app.bus_current,        "A"),
        measurement_row("Phase U",        app.phase_u_current,    "A"),
        measurement_row("Phase V",        app.phase_v_current,    "A"),
        measurement_row("Phase W",        app.phase_w_current,    "A"),
        measurement_row("Duty cycle",     app.duty_cycle,         "%"),
        measurement_row("Gate voltage",   app.gate_voltage,       "V"),
        row![
            text("Direction (meas.):").size(13).width(Length::Fixed(160.0)),
            text(app.measured_direction.as_deref().unwrap_or("—")).size(13),
        ]
        .spacing(8),
        iced::widget::Space::with_height(8),
        button(text("Refresh Measurements").size(13))
            .on_press(Message::QueryMeasurements)
            .padding([5, 12]),
    ]
    .spacing(4);

    // -----------------------------------------------------------------------
    // Compose
    // -----------------------------------------------------------------------
    let content = column![
        text("Motor Control").size(24),
        iced::widget::Space::with_height(16),
        enable_btn,
        iced::widget::Space::with_height(16),
        freq_row,
        iced::widget::Space::with_height(12),
        direction_row,
        iced::widget::Space::with_height(28),
        meas_section,
    ]
    .spacing(0)
    .max_width(800);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn not_connected_view<'a>() -> Element<'a, Message> {
    column![
        text("Motor Control").size(24),
        iced::widget::Space::with_height(20),
        text("Not connected — go to the Connection panel and connect to the board first.").size(14),
    ]
    .spacing(0)
    .into()
}

fn measurement_row<'a>(label: &'a str, value: Option<f32>, unit: &'a str) -> Element<'a, Message> {
    let value_str = match value {
        Some(v) => format!("{:.3} {}", v, unit),
        None => String::from("—"),
    };
    row![
        text(format!("{}:", label)).size(13).width(Length::Fixed(160.0)),
        text(value_str).size(13),
    ]
    .spacing(8)
    .into()
}
