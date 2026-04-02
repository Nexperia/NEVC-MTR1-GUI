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
    // Frequency (locked while motor is enabled)
    // -----------------------------------------------------------------------
    let freq_locked = app.motor_enabled;

    let freq_slider: Element<'_, Message> = {
        let s = slider(
            FREQ_MIN_HZ as f32..=FREQ_MAX_HZ as f32,
            app.motor_frequency,
            Message::FrequencyChanged,
        )
        .step(1.0)
        .width(Length::Fixed(300.0));
        if !freq_locked {
            s.on_release(Message::FrequencySubmit).into()
        } else {
            s.into()
        }
    };

    let freq_input: Element<'_, Message> = {
        let i = text_input("20000", &app.motor_frequency_input)
            .width(Length::Fixed(90.0))
            .padding(5);
        if !freq_locked {
            i.on_input(Message::FrequencyInputChanged)
                .on_submit(Message::FrequencySubmit)
                .into()
        } else {
            i.into()
        }
    };

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

    let freq_section: Element<'_, Message> = if freq_locked {
        column![
            freq_row,
            text("Disable motor before changing frequency.").size(11),
        ]
        .spacing(2)
        .into()
    } else {
        column![freq_row].spacing(0).into()
    };

    // -----------------------------------------------------------------------
    // Direction (buttons disabled while a direction-change is in flight)
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

    let fwd_btn: Element<'_, Message> = {
        let b = button(text("Forward").size(13))
            .style(fwd_style)
            .padding([5, 12]);
        if !app.motor_busy {
            b.on_press(Message::DirectionChanged(Direction::Forward)).into()
        } else {
            b.into()
        }
    };
    let rev_btn: Element<'_, Message> = {
        let b = button(text("Reverse").size(13))
            .style(rev_style)
            .padding([5, 12]);
        if !app.motor_busy {
            b.on_press(Message::DirectionChanged(Direction::Reverse)).into()
        } else {
            b.into()
        }
    };
    let dir_status: Element<'_, Message> = if app.motor_busy {
        text("Changing direction…").size(11).into()
    } else {
        iced::widget::Space::with_width(0).into()
    };

    let direction_row = row![
        text("Direction:").size(14),
        iced::widget::Space::with_width(8),
        fwd_btn,
        rev_btn,
        dir_status,
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
        measurement_row("System Voltage",   app.gate_voltage,       "V"),
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
        freq_section,
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
