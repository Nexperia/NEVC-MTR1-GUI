// ---------------------------------------------------------------------------
// Firmware & Configuration panel (combined Stage 5 + 6 tab)
// ---------------------------------------------------------------------------

use iced::widget::{button, column, container, row, scrollable, text, text_input, tooltip};
use iced::{Element, Length};

/// Segoe UI Symbol ships on all Windows versions and covers the ⚠ glyph
/// absent from the bundled Ubuntu font.
const SYM_FONT: iced::Font = iced::Font {
    family: iced::font::Family::Name("Segoe UI Symbol"),
    weight: iced::font::Weight::Normal,
    style: iced::font::Style::Normal,
    stretch: iced::font::Stretch::Normal,
};

use crate::app::{FlashStatus, FwConfigSource, Message, NevcApp};
use crate::serial::ConnectionState;

// ---------------------------------------------------------------------------
// Parameter metadata
// ---------------------------------------------------------------------------

/// How to render / validate an individual parameter input.
#[derive(Clone, Copy)]
pub enum ParamKind {
    UInt,
    SInt,
    Float,               // real-unit input (A, V) converted to/from ADC in mod.rs
    Bool,
    TurnOffMode,         // 0=COAST, 1=RAMP
    SpeedControlMethod,  // 0=OPEN_LOOP, 1=CLOSED_LOOP
}

pub struct ParamMeta {
    pub label: &'static str,
    pub unit: &'static str,
    pub help: &'static str,
    pub kind: ParamKind,
}

/// All 26 parameter descriptors, in IDN serial index order.
pub const PARAMS: &[ParamMeta] = &[
    // --- Motor ---
    ParamMeta { label: "Motor Poles",            unit: "",    help: "Number of poles in the motor (42BLS40-24-01 has 8). Must be a positive even number.",  kind: ParamKind::UInt },
    ParamMeta { label: "Switching Freq",         unit: "Hz",  help: "Gate switching frequency. Valid range: 7183\u{2013}100000 Hz.",                           kind: ParamKind::UInt },
    ParamMeta { label: "Dead Time",              unit: "ns",  help: "Dead time inserted between complementary gate signals to prevent shoot-through. Valid range: 350\u{2013}1875 ns.", kind: ParamKind::UInt },
    ParamMeta { label: "Emulate Hall",           unit: "",    help: "Generate hall sensor output signals for testing gate outputs without a real motor. Debug use only \u{2014} do not connect a real motor with hall sensors simultaneously.", kind: ParamKind::Bool },
    ParamMeta { label: "Emulated Motor Freq",    unit: "Hz",  help: "Electrical rotation frequency used when Emulate Hall is enabled. Valid range: 1\u{2013}1000 Hz. Only active when Emulate Hall is TRUE.", kind: ParamKind::UInt },
    ParamMeta { label: "Stopped Threshold",      unit: "ticks",help: "Number of PWM ticks without a hall edge before the motor is declared stopped. Must be > 0.", kind: ParamKind::UInt },
    ParamMeta { label: "Turn-Off Mode",          unit: "",    help: "How phases are deactivated when the motor is stopped: COAST lets the motor free-wheel; RAMP applies a controlled deceleration.", kind: ParamKind::TurnOffMode },
    // --- Phase Current ---
    ParamMeta { label: "Phase Current Gain",     unit: "",    help: "Amplifier gain of the in-line phase current sense circuit (NEVB-MTR1-I56-1 default: 20).", kind: ParamKind::UInt },
    ParamMeta { label: "Phase Sense Resistor",   unit: "\u{b5}Ohm",help: "Phase current sense shunt resistor value in micro-ohms (NEVB-MTR1-I56-1 default: 2500 \u{b5}\u{3a9}).", kind: ParamKind::UInt },
    // --- Bus Current ---
    ParamMeta { label: "Bus Current Gain",       unit: "",    help: "Amplifier gain of the hi-side bus current sense circuit (NEVB-MTR1-C-1 default: 50).",   kind: ParamKind::UInt },
    ParamMeta { label: "Bus Sense Resistor",     unit: "\u{b5}Ohm",help: "Bus current sense shunt resistor value in micro-ohms (NEVB-MTR1-C-1 default: 4000 \u{b5}\u{3a9}).", kind: ParamKind::UInt },
    ParamMeta { label: "Bus Warn Threshold",     unit: "A",   help: "Bus current level at which a warning is flagged. Enter the physical current in amperes; the ADC equivalent is shown below.", kind: ParamKind::Float },
    ParamMeta { label: "Bus Fault Enable",       unit: "",    help: "When TRUE, all PWM outputs are disabled immediately when bus current exceeds the error threshold.", kind: ParamKind::Bool },
    ParamMeta { label: "Bus Error Threshold",    unit: "A",   help: "Bus current level at which an error is triggered (and PWM disabled if Bus Fault Enable is TRUE). Enter in amperes; ADC equivalent shown below.", kind: ParamKind::Float },
    // --- Speed Control ---
    ParamMeta { label: "Speed Control Method",   unit: "",    help: "OPEN LOOP: speed set by duty cycle directly. CLOSED LOOP: speed regulated by PID controller using hall sensor feedback.", kind: ParamKind::SpeedControlMethod },
    ParamMeta { label: "Speed Loop Time Base",   unit: "ticks",help: "Number of PWM ticks between each speed-loop iteration. Valid range: 1\u{2013}255 ticks.", kind: ParamKind::UInt },
    ParamMeta { label: "Max Speed Delta",        unit: "",    help: "Maximum change in speed reference per loop iteration, used for open-loop ramping. Must be > 0.", kind: ParamKind::UInt },
    ParamMeta { label: "Max Speed",              unit: "hall Hz",help: "Maximum motor speed setpoint for closed-loop control, measured in hall edge frequency. Must be > 0.", kind: ParamKind::UInt },
    // --- PID ---
    ParamMeta { label: "PID Kp",                 unit: "",    help: "PID proportional gain (16-bit signed integer). Active in closed-loop mode only. Range: -32768\u{2013}32767.", kind: ParamKind::SInt },
    ParamMeta { label: "PID Ki",                 unit: "",    help: "PID integral gain (16-bit signed integer). Active in closed-loop mode only. Range: -32768\u{2013}32767.", kind: ParamKind::SInt },
    ParamMeta { label: "PID Kd Enable",          unit: "",    help: "Enable the derivative term in the PID controller. Only active in closed-loop mode.",     kind: ParamKind::Bool },
    ParamMeta { label: "PID Kd",                 unit: "",    help: "PID derivative gain (16-bit signed integer). Active in closed-loop mode only. Range: -32768\u{2013}32767.", kind: ParamKind::SInt },
    ParamMeta { label: "PID Max I Term",         unit: "",    help: "Anti-windup clamp on the PID integrator. Limits the maximum absolute value of the integral accumulator. Active in closed-loop mode only.", kind: ParamKind::UInt },
    ParamMeta { label: "PID Output Max",         unit: "",    help: "Ceiling on the PID controller output, capping the maximum speed reference it can produce. Active in closed-loop mode only.", kind: ParamKind::UInt },
    // --- VBUS Sense ---
    ParamMeta { label: "VBUS Top Resistor",      unit: "Ohm", help: "Top (high-side) resistor of the VBUS voltage divider (NEVB-MTR1-C-1 default: 100\u{202f}000 \u{3a9}).", kind: ParamKind::UInt },
    ParamMeta { label: "VBUS Bottom Resistor",   unit: "Ohm", help: "Bottom (low-side) resistor of the VBUS voltage divider (NEVB-MTR1-C-1 default: 6\u{202f}200 \u{3a9}).", kind: ParamKind::UInt },
    ParamMeta { label: "VBUS Min Threshold",     unit: "V",   help: "Minimum bus voltage required before the motor can be enabled. Enter in volts; ADC equivalent shown below.", kind: ParamKind::Float },
    // --- System ---
    ParamMeta { label: "Wait for Board",         unit: "",    help: "When TRUE, the firmware waits for the NEVB-MTR1-C-1 inverter board to assert its ready signal before enabling the motor.", kind: ParamKind::Bool },
    ParamMeta { label: "Remote Debug Mode",      unit: "",    help: "When TRUE, SCPI errors are pushed to the serial port immediately without waiting for a query. This breaks the SCPI request/response protocol \u{2014} do not use with this GUI.", kind: ParamKind::Bool },
];

pub const GROUP_LABELS: &[(&str, std::ops::Range<usize>)] = &[
    ("Motor",           0..7),
    ("Phase Current",   7..9),
    ("Bus Current",     9..14),
    ("Speed Control",   14..18),
    ("PID Controller",  18..24),
    ("VBUS Sense",      24..27),
    ("System",          27..29),
];

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

pub fn view(app: &NevcApp) -> Element<'_, Message> {
    // -----------------------------------------------------------------------
    // Source buttons
    // -----------------------------------------------------------------------
    let can_load_device = app.connection == ConnectionState::Connected
        && app.idn_serial.is_some();

    let load_github_btn = button(text("Load Defaults from GitHub").size(13))
        .style(iced::theme::Button::Secondary)
        .on_press(Message::FwSourceChanged(FwConfigSource::Repo))
        .padding([6, 14]);

    let load_device_btn = {
        let b = button(text("Load Defaults from Device").size(13))
            .style(iced::theme::Button::Secondary)
            .padding([6, 14]);
        if can_load_device {
            b.on_press(Message::FwSourceChanged(FwConfigSource::Device))
        } else {
            b
        }
    };

    let source_hint = match app.firmware_config_source {
        FwConfigSource::Device if can_load_device =>
            text("Values loaded from connected device IDN serial field.").size(12),
        FwConfigSource::Device =>
            text("Device not connected - connect and query IDN to load device values.").size(12),
        FwConfigSource::Repo =>
            text("Values loaded from repo defaults (main/config.h).").size(12),
    };

    let source_row = row![
        load_github_btn,
        load_device_btn,
    ]
    .spacing(8)
    .align_items(iced::Alignment::Center);

    // -----------------------------------------------------------------------
    // Parameter groups
    // -----------------------------------------------------------------------
    // PID params (indices 18-23) are only active when closed-loop is selected.
    let is_closed_loop = app.fw_param_inputs
        .get(14)
        .map(|v| v == "1" || v.to_uppercase().contains("CLOSED"))
        .unwrap_or(false);
    let emulate_hall_on = app.fw_param_inputs
        .get(3)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);
    let bus_fault_enable_on = app.fw_param_inputs
        .get(12)  // Bus Fault Enable is at index 12
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);

    let mut param_groups: Vec<Element<Message>> = Vec::new();

    for (group_label, range) in GROUP_LABELS {
        // Dim the entire PID group when open-loop is selected
        let group_dimmed = group_label == &"PID Controller" && !is_closed_loop;

        let mut rows: Vec<Element<Message>> = Vec::new();
        let group_label_widget = if group_dimmed {
            text(format!("{} (open loop - not used)", group_label))
                .size(15)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.6)))
        } else {
            text(*group_label).size(15)
        };
        rows.push(
            container(group_label_widget)
                .padding([6, 0, 2, 0])
                .into(),
        );

        for idx in range.clone() {
            let meta = &PARAMS[idx];
            let input_val = app.fw_param_inputs.get(idx).map(|s| s.as_str()).unwrap_or("");
            // idx 4 (Emulated Motor Freq) only active when Emulate Hall is TRUE
            let param_dimmed = group_dimmed || (idx == 4 && !emulate_hall_on) || (idx == 13 && !bus_fault_enable_on);

            let dim_color = iced::Color::from_rgb(0.6, 0.6, 0.6);
            let label_text = if param_dimmed {
                let label = if meta.unit.is_empty() {
                    meta.label.to_string()
                } else {
                    format!("{} ({})", meta.label, meta.unit)
                };
                text(label).size(13).style(iced::theme::Text::Color(dim_color))
            } else if meta.unit.is_empty() {
                text(meta.label.to_string()).size(13)
            } else {
                text(format!("{} ({})", meta.label, meta.unit)).size(13)
            };

            let input_widget: Element<Message> = match meta.kind {
                ParamKind::Bool => {
                    let is_true = input_val.to_lowercase() == "true" || input_val == "1";
                    let r = row![
                        {
                            let b = button(text("TRUE").size(12))
                                .style(if is_true { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                                .padding([3, 10]);
                            if param_dimmed { b } else { b.on_press(Message::FwParamChanged(idx, "true".to_string())) }
                        },
                        {
                            let b = button(text("FALSE").size(12))
                                .style(if !is_true { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                                .padding([3, 10]);
                            if param_dimmed { b } else { b.on_press(Message::FwParamChanged(idx, "false".to_string())) }
                        },
                    ]
                    .spacing(4);
                    r.into()
                }
                ParamKind::TurnOffMode => {
                    let is_ramp = input_val == "1" || input_val.to_uppercase().contains("RAMP");
                    row![
                        button(text("RAMP").size(12))
                            .style(if is_ramp { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                            .on_press(Message::FwParamChanged(idx, "1".to_string()))
                            .padding([3, 10]),
                        button(text("COAST").size(12))
                            .style(if !is_ramp { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                            .on_press(Message::FwParamChanged(idx, "0".to_string()))
                            .padding([3, 10]),
                    ]
                    .spacing(4)
                    .into()
                }
                ParamKind::SpeedControlMethod => {
                    let is_closed = input_val == "1" || input_val.to_uppercase().contains("CLOSED");
                    row![
                        button(text("OPEN LOOP").size(12))
                            .style(if !is_closed { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                            .on_press(Message::FwParamChanged(idx, "0".to_string()))
                            .padding([3, 10]),
                        button(text("CLOSED LOOP").size(12))
                            .style(if is_closed { iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton)) } else { iced::theme::Button::Secondary })
                            .on_press(Message::FwParamChanged(idx, "1".to_string()))
                            .padding([3, 10]),
                    ]
                    .spacing(4)
                    .into()
                }
                ParamKind::UInt | ParamKind::SInt | ParamKind::Float => {
                    let i = text_input("", input_val)
                        .width(120)
                        .padding([4, 6]);
                    if param_dimmed {
                        i.into()
                    } else {
                        i.on_input(move |s| Message::FwParamChanged(idx, s)).into()
                    }
                }
            };

            // Wrap input in a hover tooltip showing the help text
            let tooltip_content = container(text(meta.help).size(11))
                .padding([6, 10])
                .max_width(340)
                .style(iced::theme::Container::Box);
            let input_with_tip: Element<Message> = tooltip(
                input_widget,
                tooltip_content,
                tooltip::Position::Right,
            )
            .into();

            let param_row = row![
                container(label_text)
                    .width(220),
                input_with_tip,
            ]
            .spacing(6)
            .align_items(iced::Alignment::Center);

            rows.push(param_row.into());
            rows.push(iced::widget::Space::with_height(4).into());

            // ADC estimate for bus current threshold params (enter A, show ADC)
            if idx == 11 || idx == 13 {
                let a_val: Option<f64> = input_val.trim().parse::<f64>().ok();
                let gain: Option<f64> = app.fw_param_inputs.get(9)
                    .and_then(|s| s.trim().parse::<f64>().ok());
                let resistor_uohm: Option<f64> = app.fw_param_inputs.get(10)
                    .and_then(|s| s.trim().parse::<f64>().ok());

                if let (Some(a), Some(g), Some(r)) = (a_val, gain, resistor_uohm) {
                    let adc_est = if g > 0.0 && r > 0.0 {
                        a * g * r / (0.004888 * 1_000_000.0)
                    } else {
                        0.0
                    };
                    let overflow = !(0.0..=1023.0).contains(&adc_est);
                    let hint_color = if overflow {
                        iced::Color::from_rgb(0.85, 0.2, 0.2)
                    } else {
                        iced::Color::from_rgb(0.35, 0.55, 0.75)
                    };
                    let hint_row: Element<Message> = if overflow {
                        row![
                            iced::widget::Space::with_width(226),
                            text("⚠").size(11).font(SYM_FONT)
                                .style(iced::theme::Text::Color(hint_color)),
                            text(format!(" ≈ {:.0} ADC  out of range (0–1023)", adc_est)).size(11)
                                .style(iced::theme::Text::Color(hint_color)),
                        ]
                        .align_items(iced::Alignment::Center)
                        .into()
                    } else {
                        row![
                            iced::widget::Space::with_width(226),
                            text(format!("≈ {:.0} ADC", adc_est)).size(11)
                                .style(iced::theme::Text::Color(hint_color)),
                        ]
                        .into()
                    };
                    rows.push(hint_row);
                    rows.push(iced::widget::Space::with_height(2).into());
                }
            }
            // ADC estimate for VBUS min threshold (enter V, show ADC)
            if idx == 26 {
                let v_val: Option<f64> = input_val.trim().parse::<f64>().ok();
                let rtop: Option<f64> = app.fw_param_inputs.get(24)
                    .and_then(|s| s.trim().parse::<f64>().ok());
                let rbottom: Option<f64> = app.fw_param_inputs.get(25)
                    .and_then(|s| s.trim().parse::<f64>().ok());

                if let (Some(v), Some(rt), Some(rb)) = (v_val, rtop, rbottom) {
                    let adc_est = if rb > 0.0 {
                        v * rb / (rt + rb) / 5.0 * 1023.0
                    } else {
                        0.0
                    };
                    let overflow = !(0.0..=1023.0).contains(&adc_est);
                    let hint_color = if overflow {
                        iced::Color::from_rgb(0.85, 0.2, 0.2)
                    } else {
                        iced::Color::from_rgb(0.35, 0.55, 0.75)
                    };
                    let hint_row2: Element<Message> = if overflow {
                        row![
                            iced::widget::Space::with_width(226),
                            text("⚠").size(11).font(SYM_FONT)
                                .style(iced::theme::Text::Color(hint_color)),
                            text(format!(" ≈ {:.0} ADC  out of range (0–1023)", adc_est)).size(11)
                                .style(iced::theme::Text::Color(hint_color)),
                        ]
                        .align_items(iced::Alignment::Center)
                        .into()
                    } else {
                        row![
                            iced::widget::Space::with_width(226),
                            text(format!("≈ {:.0} ADC", adc_est)).size(11)
                                .style(iced::theme::Text::Color(hint_color)),
                        ]
                        .into()
                    };
                    rows.push(hint_row2);
                    rows.push(iced::widget::Space::with_height(2).into());
                }
            }

            // Emulate Hall safety warning (idx 3)
            if idx == 3 {
                let is_on = input_val == "true";
                if is_on {
                    rows.push(
                        container(
                            row![
                                text("⚠").size(11).font(SYM_FONT),
                                text("  Debug use only. Do NOT connect a motor with both phase AND hall sensor\n   connections at the same time. Use this mode only to verify gate outputs\n   are correct during the different commutation stages.").size(11),
                            ]
                            .align_items(iced::Alignment::Start)
                        )
                        .padding([5, 12])
                        .style(iced::theme::Container::Box)
                        .into()
                    );
                    rows.push(iced::widget::Space::with_height(4).into());
                }
            }

            // Live validation error for this parameter
            if let Some(Some(err_msg)) = app.fw_param_errors.get(idx) {
                let err_color = iced::Color::from_rgb(0.85, 0.2, 0.2);
                rows.push(
                    row![
                        iced::widget::Space::with_width(226),
                        text("⚠").size(11).font(SYM_FONT)
                            .style(iced::theme::Text::Color(err_color)),
                        text(format!("  {}", err_msg)).size(11)
                            .style(iced::theme::Text::Color(err_color)),
                    ]
                    .align_items(iced::Alignment::Center)
                    .into()
                );
                rows.push(iced::widget::Space::with_height(2).into());
            }
        }

        for w in rows {
            param_groups.push(w);
        }
        param_groups.push(iced::widget::Space::with_height(8).into());
    }

    // -----------------------------------------------------------------------
    // Compile & Upload section
    // -----------------------------------------------------------------------
    let flash_busy = matches!(app.flash_status, FlashStatus::Busy(_));

    let compile_btn = {
        let mut b = button(
            text(if flash_busy { "Flashing..." } else { "Compile & Upload" }).size(14),
        )
        .style(if flash_busy {
            iced::theme::Button::Secondary
        } else {
            iced::theme::Button::Custom(Box::new(crate::ui::style::FilledButton))
        })
        .padding([8, 18]);
        if !flash_busy {
            b = b.on_press(Message::FwCompileAndUpload);
        }
        b
    };

    let flash_status_text: Element<Message> = match &app.flash_status {
        FlashStatus::Idle => text("").size(13).into(),
        FlashStatus::Busy(step) => text(format!("  {}", step)).size(13).into(),
        FlashStatus::Done => text("  Flash complete.").size(13).into(),
        FlashStatus::Failed(e) => text(format!("  Error: {}", e)).size(13).into(),
    };

    let port_hint: Element<Message> = if app.connection == ConnectionState::Connected {
        let port = app.selected_port.as_deref().unwrap_or("?");
        text(format!("Upload target: {}  (will be reset to bootloader for upload)", port))
            .size(14)
            .into()
    } else {
        let warning_port = app.selected_port.as_deref().unwrap_or("(none)");
        column![
            row![
                text("⚠").size(14).font(SYM_FONT),
                text("  No device connected.").size(14),
            ]
            .align_items(iced::Alignment::Center),
            text(format!(
                "Currently selected port: {}. Go to the Connection tab to select the correct COM port before uploading.",
                warning_port
            )).size(13),
        ]
        .spacing(4)
        .into()
    };

    // Flash log - selectable/copyable
    let log_section: Element<Message> = iced::widget::text_editor(&app.flash_log_content)
        .on_action(Message::FwLogAction)
        .height(200)
        .font(iced::Font::MONOSPACE)
        .into();

    let flash_section = column![
        text("Compile & Upload").size(18),
        iced::widget::Space::with_height(4),
        text("The sketch will be compiled with your parameters and uploaded to the\nLeonardo via USB. Arduino CLI will be downloaded automatically if needed.").size(12),
        iced::widget::Space::with_height(6),
        port_hint,
        iced::widget::Space::with_height(8),
        row![compile_btn, iced::widget::Space::with_width(16), flash_status_text]
            .align_items(iced::Alignment::Center),
        iced::widget::Space::with_height(12),
        text("Log:").size(13),
        iced::widget::Space::with_height(4),
        log_section,
    ]
    .spacing(2);

    // -----------------------------------------------------------------------
    // Compose full panel
    // -----------------------------------------------------------------------
    let mut content_children: Vec<Element<Message>> = vec![
        source_row.into(),
        source_hint.into(),
        iced::widget::Space::with_height(16).into(),
        text("Configuration Parameters").size(18).into(),
        iced::widget::Space::with_height(8).into(),
    ];
    content_children.extend(param_groups);
    content_children.push(iced::widget::Space::with_height(16).into());
    content_children.push(
        container(iced::widget::horizontal_rule(1))
            .width(Length::Fill)
            .into(),
    );
    content_children.push(iced::widget::Space::with_height(12).into());
    content_children.push(flash_section.into());

    let content = column(content_children).spacing(0).max_width(900);

    scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

