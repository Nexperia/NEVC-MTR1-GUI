use iced::{Application, Command, Element, Length, Theme};
use iced::widget::{button, column, container, row, text};

use crate::serial::{ConnectionState, PortInfo, SerialHandle};
use crate::ui::{Panel, config, connection, firmware, graphs, log_panel, motor};

// ---------------------------------------------------------------------------
// Log entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

// ---------------------------------------------------------------------------
// Root application state
// ---------------------------------------------------------------------------

pub struct NevcApp {
    pub active_panel: Panel,

    // Serial / connection
    pub connection: ConnectionState,
    pub available_ports: Vec<PortInfo>,
    pub selected_port: Option<String>,
    pub status_message: String,

    // Firmware info (populated after *IDN?)
    pub firmware_version: Option<String>,
    pub idn_manufacturer: Option<String>,
    pub idn_model: Option<String>,
    pub idn_serial: Option<String>,

    // Motor control state (write-side)
    pub motor_enabled: bool,
    pub motor_frequency: f32,
    pub motor_frequency_input: String,
    pub motor_direction: Direction,

    // Measurement values (read-side, None until first poll)
    pub speed_rpm: Option<f32>,
    pub bus_current: Option<f32>,
    pub phase_u_current: Option<f32>,
    pub phase_v_current: Option<f32>,
    pub phase_w_current: Option<f32>,
    pub measured_direction: Option<String>,
    pub duty_cycle: Option<f32>,
    pub gate_voltage: Option<f32>,

    // Open serial connection (None when disconnected)
    pub serial_handle: Option<SerialHandle>,

    // Event log (shown in Log panel)
    pub log: Vec<LogEntry>,

    // Firmware panel
    pub flash_log: Vec<String>,
}

// ---------------------------------------------------------------------------
// Motor direction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Forward,
    Reverse,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Direction::Forward => write!(f, "Forward"),
            Direction::Reverse => write!(f, "Reverse"),
        }
    }
}

// ---------------------------------------------------------------------------
// Messages — every user interaction and async result
// ---------------------------------------------------------------------------

#[allow(dead_code)] // several variants are wired up in update() but not yet sent from the UI (Stage 2+)
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    TabSelected(Panel),

    // Connection panel
    RefreshPorts,
    PortsRefreshed(Vec<PortInfo>),
    PortSelected(String),
    ConnectPressed,
    DisconnectPressed,
    Connected(Result<SerialHandle, String>),
    IdnReceived(Result<crate::scpi::IdnResponse, String>),
    ErrorsChecked(String),

    // Motor control
    EnableChanged(bool),
    FrequencyChanged(f32),
    FrequencyInputChanged(String),
    FrequencySubmit,
    DirectionChanged(Direction),

    // Measurements
    QueryMeasurements,
    MeasurementsReceived {
        speed: Option<f32>,
        bus_current: Option<f32>,
        phase_u: Option<f32>,
        phase_v: Option<f32>,
        phase_w: Option<f32>,
        direction: Option<String>,
        duty_cycle: Option<f32>,
        voltage: Option<f32>,
    },

    // Firmware
    FlashFirmwarePressed,
    FlashLogEntry(String),

    // Generic status
    StatusMessage(String),
    ClearLog,
}

// ---------------------------------------------------------------------------
// iced Application impl
// ---------------------------------------------------------------------------

impl Application for NevcApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let app = Self {
            active_panel: Panel::Connection,
            connection: ConnectionState::Disconnected,
            available_ports: Vec::new(),
            selected_port: None,
            status_message: String::from("Not connected. Select a COM port and click Connect."),
            firmware_version: None,
            idn_manufacturer: None,
            idn_model: None,
            idn_serial: None,
            motor_enabled: false,
            motor_frequency: 20_000.0,
            motor_frequency_input: String::from("20000"),
            motor_direction: Direction::Forward,
            speed_rpm: None,
            bus_current: None,
            phase_u_current: None,
            phase_v_current: None,
            phase_w_current: None,
            measured_direction: None,
            duty_cycle: None,
            gate_voltage: None,
            flash_log: Vec::new(),
            serial_handle: None,
            log: Vec::new(),
        };

        // Detect COM ports immediately on startup
        let cmd = Command::perform(
            async { crate::serial::list_ports() },
            Message::PortsRefreshed,
        );

        (app, cmd)
    }

    fn title(&self) -> String {
        String::from("Nexperia Motor Driver GUI — NEVC-MTR1")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // ---- Navigation ----
            Message::TabSelected(panel) => {
                self.active_panel = panel;
                Command::none()
            }

            // ---- Port management ----
            Message::RefreshPorts => Command::perform(
                async { crate::serial::list_ports() },
                Message::PortsRefreshed,
            ),

            Message::PortsRefreshed(ports) => {
                let prev = self.selected_port.clone();
                self.available_ports = ports;
                // Keep previous selection if it still exists, otherwise pick first
                let still_valid = prev
                    .as_ref()
                    .map(|n| self.available_ports.iter().any(|p| &p.name == n))
                    .unwrap_or(false);
                if !still_valid {
                    self.selected_port =
                        self.available_ports.first().map(|p| p.name.clone());
                }
                Command::none()
            }

            Message::PortSelected(name) => {
                // The pick_list returns the display string; extract port name before " —"
                let port_name = name
                    .split(" —")
                    .next()
                    .unwrap_or(&name)
                    .trim()
                    .to_string();
                self.selected_port = Some(port_name);
                Command::none()
            }

            // ---- Connection ----
            Message::ConnectPressed => {
                let port_name = match self.selected_port.clone() {
                    Some(p) => p,
                    None => return Command::none(),
                };
                self.connection = ConnectionState::Connecting;
                self.status_message = format!("Connecting to {}\u{2026}", port_name);
                self.push_log(LogLevel::Info, format!("Connecting to {}\u{2026}", port_name));
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::serial::open_port(&port_name))
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r)
                    },
                    Message::Connected,
                )
            }

            Message::DisconnectPressed => {
                let port = self.selected_port.clone().unwrap_or_default();
                self.serial_handle = None;
                self.connection = ConnectionState::Disconnected;
                self.firmware_version = None;
                self.idn_manufacturer = None;
                self.idn_model = None;
                self.idn_serial = None;
                self.status_message = String::from("Disconnected.");
                self.push_log(LogLevel::Info, format!("Disconnected from {}.", port));
                Command::none()
            }

            Message::Connected(Ok(handle)) => {
                self.connection = ConnectionState::Connected;
                self.serial_handle = Some(handle.clone());
                self.status_message = String::from("Connected — querying firmware version…");
                // Fire *IDN? query
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::serial::scpi_query(&handle, crate::scpi::commands::IDN)
                                .and_then(|resp| {
                                    crate::scpi::IdnResponse::parse(&resp)
                                        .ok_or_else(|| format!("Could not parse IDN: '{}'", resp))
                                })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::IdnReceived,
                )
            }

            Message::Connected(Err(e)) => {
                self.connection = ConnectionState::Disconnected;
                self.status_message = format!("Connection failed: {}", e);
                Command::none()
            }

            Message::IdnReceived(Ok(idn)) => {
                self.firmware_version = Some(idn.firmware_version.clone());
                self.idn_manufacturer = Some(idn.manufacturer.clone());
                self.idn_model = Some(idn.model.clone());
                self.idn_serial = Some(idn.serial.clone());
                let msg = format!("Connected \u{2014} firmware v{}", idn.firmware_version);
                self.status_message = msg.clone();
                self.push_log(LogLevel::Info, format!(
                    "IDN: {} {} — firmware v{}",
                    idn.manufacturer, idn.model, idn.firmware_version
                ));
                // Poll the SCPI error queue so any startup errors are surfaced
                let handle = self.serial_handle.clone().unwrap();
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            // Read error count first
                            let count_str = crate::serial::scpi_query(
                                &handle,
                                crate::scpi::commands::SYS_ERROR_COUNT,
                            )?;
                            let count: u32 = count_str.trim().parse().unwrap_or(0);
                            if count == 0 {
                                return Ok(String::new());
                            }
                            // Drain up to `count` errors from queue
                            let mut messages = Vec::new();
                            for _ in 0..count {
                                let err = crate::serial::scpi_query(
                                    &handle,
                                    crate::scpi::commands::SYS_ERROR,
                                )?;
                                if !err.starts_with("0") {
                                    messages.push(err);
                                }
                            }
                            Ok(messages.join("; "))
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                        .unwrap_or_else(|e| format!("(error queue check failed: {})", e))
                    },
                    Message::ErrorsChecked,
                )
            }

            Message::IdnReceived(Err(e)) => {
                let msg = format!("IDN query failed: {}", e);
                self.push_log(LogLevel::Error, msg.clone());
                self.status_message = msg;
                Command::none()
            }

            Message::ErrorsChecked(errors) => {
                if !errors.is_empty() {
                    let msg = format!("SCPI errors on connect: {}", errors);
                    self.push_log(LogLevel::Warn, msg.clone());
                    self.status_message = msg;
                } else {
                    self.push_log(LogLevel::Info, "Error queue empty.".to_string());
                }
                Command::none()
            }

            // ---- Motor control ----
            Message::EnableChanged(en) => {
                self.motor_enabled = en;
                // TODO Stage 3: send SCPI CONFigure:ENABle ON/OFF
                Command::none()
            }

            Message::FrequencyChanged(freq) => {
                self.motor_frequency = freq;
                self.motor_frequency_input = format!("{:.0}", freq);
                // TODO Stage 3: send SCPI CONFigure:FREQuency
                Command::none()
            }

            Message::FrequencyInputChanged(s) => {
                self.motor_frequency_input = s;
                Command::none()
            }

            Message::FrequencySubmit => {
                if let Ok(hz) = self.motor_frequency_input.trim().parse::<f32>() {
                    match crate::scpi::validate_frequency(hz) {
                        Ok(valid_hz) => {
                            self.motor_frequency = valid_hz as f32;
                            self.status_message = format!("Frequency set to {} Hz", valid_hz);
                            // TODO Stage 3: send SCPI command
                        }
                        Err(e) => {
                            self.status_message = e;
                        }
                    }
                } else {
                    self.status_message =
                        String::from("Invalid frequency value — enter a number.");
                }
                Command::none()
            }

            Message::DirectionChanged(dir) => {
                self.motor_direction = dir;
                // TODO Stage 3: send SCPI CONFigure:DIREction
                Command::none()
            }

            // ---- Measurements ----
            Message::QueryMeasurements => {
                // TODO Stage 3: send all MEASure:* queries
                Command::none()
            }

            Message::MeasurementsReceived {
                speed,
                bus_current,
                phase_u,
                phase_v,
                phase_w,
                direction,
                duty_cycle,
                voltage,
            } => {
                self.speed_rpm = speed;
                self.bus_current = bus_current;
                self.phase_u_current = phase_u;
                self.phase_v_current = phase_v;
                self.phase_w_current = phase_w;
                self.measured_direction = direction;
                self.duty_cycle = duty_cycle;
                self.gate_voltage = voltage;
                Command::none()
            }

            // ---- Firmware ----
            Message::FlashFirmwarePressed => {
                self.flash_log.clear();
                self.flash_log
                    .push(String::from("Flash initiated — TODO Stage 5"));
                // TODO Stage 5: avrdude integration
                Command::none()
            }

            Message::FlashLogEntry(entry) => {
                self.flash_log.push(entry);
                Command::none()
            }

            // ---- Generic status ----
            Message::StatusMessage(msg) => {
                self.status_message = msg;
                Command::none()
            }

            Message::ClearLog => {
                self.log.clear();
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Platform guard — show notice on non-Windows builds at runtime
        #[cfg(not(target_os = "windows"))]
        {
            return container(
                text("This application requires Windows 10 or 11.")
                    .size(18),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into();
        }

        #[cfg(target_os = "windows")]
        self.view_windows()
    }
}

// ---------------------------------------------------------------------------
// Log helper
// ---------------------------------------------------------------------------

impl NevcApp {
    pub fn push_log(&mut self, level: LogLevel, msg: impl Into<String>) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let h = (secs / 3600) % 24;
        let m = (secs / 60) % 60;
        let s = secs % 60;
        let message = msg.into();
        // Mirror important log entries to the status bar
        match level {
            LogLevel::Warn | LogLevel::Error => {
                self.status_message = message.clone();
            }
            _ => {}
        }
        self.log.push(LogEntry {
            timestamp: format!("{:02}:{:02}:{:02}", h, m, s),
            level,
            message,
        });
    }
}

// ---------------------------------------------------------------------------
// Platform-specific view (Windows)
// ---------------------------------------------------------------------------

impl NevcApp {
    #[cfg(target_os = "windows")]
    fn view_windows(&self) -> Element<'_, Message> {
        let header = self.view_header();
        let tab_bar = self.view_tab_bar();

        let panel_content: Element<Message> = match self.active_panel {
            Panel::Connection => connection::view(self),
            Panel::Firmware => firmware::view(self),
            Panel::MotorControl => motor::view(self),
            Panel::Graphs => graphs::view(self),
            Panel::Configuration => config::view(self),
            Panel::Log => log_panel::view(self),
        };

        let content_area = container(panel_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(16);

        let status_bar = container(
            text(format!("  {}", self.status_message)).size(13),
        )
        .width(Length::Fill)
        .padding(6)
        .style(iced::theme::Container::Box);

        column![header, tab_bar, content_area, status_bar].into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        let connected_badge = match self.connection {
            ConnectionState::Disconnected => text("● Disconnected").size(13),
            ConnectionState::Connecting => text("○ Connecting…").size(13),
            ConnectionState::Connected => text("● Connected").size(13),
        };

        container(
            row![
                text("Nexperia Motor Driver GUI").size(22),
                iced::widget::Space::with_width(Length::Fill),
                connected_badge,
                iced::widget::Space::with_width(8),
                text("NEVC-MTR1").size(16),
            ]
            .spacing(4)
            .align_items(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding([10, 20])
        .style(iced::theme::Container::Box)
        .into()
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        let tabs: &[(Panel, &str)] = &[
            (Panel::Connection, "Connection"),
            (Panel::Firmware, "Firmware"),
            (Panel::MotorControl, "Motor Control"),
            (Panel::Graphs, "Graphs"),
            (Panel::Configuration, "Configuration"),
            (Panel::Log, "Log"),
        ];

        let buttons: Vec<Element<Message>> = tabs
            .iter()
            .map(|(panel, label)| {
                let is_active = *panel == self.active_panel;
                let style = if is_active {
                    iced::theme::Button::Primary
                } else {
                    iced::theme::Button::Secondary
                };
                button(text(*label).size(14))
                    .on_press(Message::TabSelected(panel.clone()))
                    .padding([6, 14])
                    .style(style)
                    .into()
            })
            .collect();

        container(
            row(buttons).spacing(4),
        )
        .width(Length::Fill)
        .padding([6, 12])
        .into()
    }
}
