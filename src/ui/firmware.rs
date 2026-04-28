// ---------------------------------------------------------------------------
// Firmware & Configuration panel (combined Stage 5 + 6 tab)
// ---------------------------------------------------------------------------

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

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
    ParamMeta { label: "Motor Poles",            unit: "",    help: "Number of poles in the motor (42BLS40-24-01 has 8)",           kind: ParamKind::UInt },
    ParamMeta { label: "Switching Freq",         unit: "Hz",  help: "Gate switching frequency (7183-100000 Hz)",                   kind: ParamKind::UInt },
    ParamMeta { label: "Dead Time",              unit: "ns",  help: "Dead time between switching actions (350-1875 ns)",           kind: ParamKind::UInt },
    ParamMeta { label: "Emulate Hall",           unit: "",    help: "Generate hall sensor output signals (do not connect real sensors)", kind: ParamKind::Bool },
    ParamMeta { label: "Emulated Motor Freq",    unit: "Hz",  help: "Electrical rotational frequency for emulated motor (if enabled)", kind: ParamKind::UInt },
    ParamMeta { label: "Stopped Threshold",      unit: "ticks",help: "Hall ticks without change before motor is considered stopped", kind: ParamKind::UInt },
    ParamMeta { label: "Turn-Off Mode",          unit: "",    help: "How to turn off the motor: COAST (free-wheel) or RAMP",       kind: ParamKind::TurnOffMode },
    // --- Phase Current ---
    ParamMeta { label: "Phase Current Gain",     unit: "",    help: "In-line phase current sense amplifier gain (NEVB-MTR1-I56-1: 20)", kind: ParamKind::UInt },
    ParamMeta { label: "Phase Sense Resistor",   unit: "uOhm",help: "Phase current sense resistor value in micro-ohms (NEVB: 2500)", kind: ParamKind::UInt },
    // --- Bus Current ---
    ParamMeta { label: "Bus Current Gain",       unit: "",    help: "Hi-side bus current sense amplifier gain (NEVB: 50 or 20)",   kind: ParamKind::UInt },
    ParamMeta { label: "Bus Sense Resistor",     unit: "uOhm",help: "Bus current sense resistor value in micro-ohms (NEVB: 4000)", kind: ParamKind::UInt },
    ParamMeta { label: "Bus Warn Threshold",     unit: "ADC", help: "Bus current warning threshold (ADC 0-1023; 307 = ~7.5 A)",    kind: ParamKind::UInt },
    ParamMeta { label: "Bus Error Threshold",    unit: "ADC", help: "Bus current error threshold (ADC 0-1023; 410 = ~10 A)",       kind: ParamKind::UInt },
    ParamMeta { label: "Bus Fault Enable",       unit: "",    help: "Disable all PWM when bus current error threshold exceeded",   kind: ParamKind::Bool },
    // --- Speed Control ---
    ParamMeta { label: "Speed Control Method",   unit: "",    help: "Speed control: OPEN LOOP (duty cycle) or CLOSED LOOP (PID)", kind: ParamKind::SpeedControlMethod },
    ParamMeta { label: "Speed Loop Time Base",   unit: "ticks",help: "PWM ticks between each speed-loop iteration (1-255)",       kind: ParamKind::UInt },
    ParamMeta { label: "Max Speed Delta",        unit: "",    help: "Maximum speed reference change per loop iteration (open loop)", kind: ParamKind::UInt },
    ParamMeta { label: "Max Speed",              unit: "hall Hz",help: "Maximum motor speed setpoint for closed-loop control",    kind: ParamKind::UInt },
    // --- PID ---
    ParamMeta { label: "PID Kp",                 unit: "",    help: "PID proportional gain constant (closed-loop only, i16)",     kind: ParamKind::SInt },
    ParamMeta { label: "PID Ki",                 unit: "",    help: "PID integral gain constant (closed-loop only, i16)",         kind: ParamKind::SInt },
    ParamMeta { label: "PID Kd Enable",          unit: "",    help: "Enable the derivative term in the PID controller",           kind: ParamKind::Bool },
    ParamMeta { label: "PID Kd",                 unit: "",    help: "PID derivative gain constant (closed-loop only, i16)",       kind: ParamKind::SInt },
    // --- Voltage Sense ---
    ParamMeta { label: "VBUS Top Resistor",      unit: "Ohm", help: "Top resistor of the VBUS potential divider (NEVB-MTR1-C-1: 100 kOhm)", kind: ParamKind::UInt },
    ParamMeta { label: "VBUS Bottom Resistor",   unit: "Ohm", help: "Bottom resistor of the VBUS potential divider (NEVB: 6.2 kOhm)", kind: ParamKind::UInt },
    // --- System ---
    ParamMeta { label: "Wait for Board",         unit: "",    help: "Wait for the inverter board to be detected before enabling motor", kind: ParamKind::Bool },
    ParamMeta { label: "Remote Debug Mode",      unit: "",    help: "Send errors to serial immediately without waiting for query", kind: ParamKind::Bool },
];

pub const GROUP_LABELS: &[(&str, std::ops::Range<usize>)] = &[
    ("Motor",           0..7),
    ("Phase Current",   7..9),
    ("Bus Current",     9..14),
    ("Speed Control",   14..18),
    ("PID Controller",  18..22),
    ("Voltage Sense",   22..24),
    ("System",          24..26),
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
    // PID params (indices 18-21) are only active when closed-loop is selected.
    let is_closed_loop = app.fw_param_inputs
        .get(14)
        .map(|v| v == "1" || v.to_uppercase().contains("CLOSED"))
        .unwrap_or(false);
    const PID_RANGE: std::ops::Range<usize> = 18..22;

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
            let param_dimmed = group_dimmed || (PID_RANGE.contains(&idx) && !is_closed_loop);

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
                ParamKind::UInt | ParamKind::SInt => {
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

            let help_text: Element<Message> = if param_dimmed {
                text(meta.help).size(11).style(iced::theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.6))).into()
            } else {
                text(meta.help).size(11).into()
            };

            let param_row = row![
                container(label_text)
                    .width(220),
                input_widget,
                iced::widget::Space::with_width(12),
                help_text,
            ]
            .spacing(6)
            .align_items(iced::Alignment::Center);

            rows.push(param_row.into());
            rows.push(iced::widget::Space::with_height(4).into());

            // Amps hint for bus current threshold params (indices 11 and 12)
            if idx == 11 || idx == 12 {
                let adc_val: Option<f64> = input_val.trim().parse::<f64>().ok();
                let gain: Option<f64> = app.fw_param_inputs.get(9)
                    .and_then(|s| s.trim().parse::<f64>().ok());
                let resistor_uohm: Option<f64> = app.fw_param_inputs.get(10)
                    .and_then(|s| s.trim().parse::<f64>().ok());

                if let (Some(adc), Some(g), Some(r)) = (adc_val, gain, resistor_uohm) {
                    let current_a = if g > 0.0 && r > 0.0 {
                        adc * 0.004888 * 1_000_000.0 / (g * r)
                    } else {
                        0.0
                    };
                    rows.push(
                        row![
                            iced::widget::Space::with_width(226),
                            text(format!("≈ {:.2} A", current_a)).size(11)
                                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.35, 0.55, 0.75))),
                        ]
                        .into()
                    );
                    rows.push(iced::widget::Space::with_height(2).into());
                }
            }

            // Emulate Hall safety warning (idx 3)
            if idx == 3 {
                let is_on = input_val == "true";
                if is_on {
                    rows.push(
                        container(
                            text("⚠  Debug use only. Do NOT connect a motor with both phase AND hall sensor\n   connections at the same time. Use this mode only to verify gate outputs\n   are correct during the different commutation stages.").size(11)
                        )
                        .padding([5, 12])
                        .style(iced::theme::Container::Box)
                        .into()
                    );
                    rows.push(iced::widget::Space::with_height(4).into());
                }
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
        text(format!("Port: {}  (will be reset to bootloader)", port))
            .size(12)
            .into()
    } else {
        text("Not connected. Select port in Connection tab (port will still be used for flashing).")
            .size(12)
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

