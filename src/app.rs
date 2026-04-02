use iced::{Application, Command, Element, Length, Subscription, Theme};
use iced::widget::{button, column, container, row, text};

use crate::serial::{ConnectionState, PortInfo, SerialHandle};
use crate::ui::{Panel, config, connection, firmware, graphs, log_panel, motor};

// ---------------------------------------------------------------------------
// Graph types
// ---------------------------------------------------------------------------

pub const NUM_CHANNELS: usize = 8;
/// Short display names for each graph channel.
pub const GRAPH_CHANNEL_NAMES: [&str; NUM_CHANNELS] = [
    "Speed",
    "System Current",
    "Phase U Current",
    "Phase V Current",
    "Phase W Current",
    "Duty Cycle",
    "System Voltage",
    "System Power",
];
/// SI unit string for each channel.
pub const GRAPH_CHANNEL_UNITS: [&str; NUM_CHANNELS] =
    ["RPM", "A", "A", "A", "A", "%", "V", "W"];
/// Y-axis unit group index for each channel (0=RPM, 1=A, 2=%, 3=V, 4=W).
pub const GRAPH_CHANNEL_UNIT_GROUP: [usize; NUM_CHANNELS] = [0, 1, 1, 1, 1, 2, 3, 4];
/// Display label for each unit group.
pub const UNIT_GROUP_NAMES: [&str; 5] = ["RPM", "A", "%", "V", "W"];

#[derive(Debug, Clone)]
pub struct GraphSample {
    /// Seconds elapsed since polling started.
    pub t: f32,
    /// Absolute wall-clock time in milliseconds since UNIX epoch.
    pub wall_time_ms: u64,
    /// Values indexed by channel (indices per GRAPH_CHANNEL_NAMES).
    pub values: [Option<f32>; NUM_CHANNELS],
}

// ---------------------------------------------------------------------------
// Graph display mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum GraphMode {
    Overlay,
    Individual,
}

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
    /// True while a direction-change async sequence is in flight.
    pub motor_busy: bool,

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

    // Graphs panel
    pub graph_channels: [bool; NUM_CHANNELS],
    pub graph_poll_hz: f32,
    pub graph_running: bool,
    pub graph_history: std::collections::VecDeque<GraphSample>,
    pub graph_start_secs: f64,
    pub graph_mode: GraphMode,
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
    // Confirmations from the board after setting a value
    EnableConfirmed(Result<bool, String>),
    FrequencyConfirmed(Result<u32, String>),
    DirectionConfirmed(Result<(Direction, bool), String>),
    /// Board state read back after connect.
    MotorStateRefreshed { enabled: bool, frequency: u32, direction: Direction },
    /// Physical disconnect detected via I/O error.
    DeviceDisconnected,

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

    // Graphs panel
    GraphChannelToggled(usize),
    GraphPollRateChanged(f32),
    GraphStartStop,
    GraphDownloadCsv,
    GraphModeToggled,

    // Window / shutdown
    /// Fired when the user clicks the OS close button on the window.
    CloseRequested,
    /// Stop motor via SCPI then perform a clean disconnect.
    StopThenDisconnect,
    /// Stop motor via SCPI then close the window.
    StopThenExit,
    /// Perform a clean disconnect without stopping the motor first.
    DoDisconnect,
    /// Close the window without stopping the motor first.
    DoExit,

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
            motor_busy: false,
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
            graph_channels: [false; NUM_CHANNELS],
            graph_poll_hz: 5.0,
            graph_running: false,
            graph_history: std::collections::VecDeque::new(),
            graph_start_secs: 0.0,
            graph_mode: GraphMode::Overlay,
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
                if self.motor_enabled {
                    // Ask user before pulling the connection out from under a running motor
                    return Command::perform(
                        async {
                            rfd::AsyncMessageDialog::new()
                                .set_title("Motor is running")
                                .set_description(
                                    "The motor is currently running.\n\
                                     Stop the motor before disconnecting?"
                                )
                                .set_buttons(rfd::MessageButtons::YesNo)
                                .set_level(rfd::MessageLevel::Warning)
                                .show()
                                .await
                        },
                        |result| {
                            if result == rfd::MessageDialogResult::Yes {
                                Message::StopThenDisconnect
                            } else {
                                Message::DoDisconnect
                            }
                        },
                    );
                }
                // Motor not running — disconnect immediately
                Command::perform(async {}, |_| Message::DoDisconnect)
            }

            Message::DoDisconnect => {
                let port = self.selected_port.clone().unwrap_or_default();
                self.serial_handle = None;
                self.connection = ConnectionState::Disconnected;
                self.firmware_version = None;
                self.idn_manufacturer = None;
                self.idn_model = None;
                self.idn_serial = None;
                self.motor_enabled = false;
                self.motor_busy = false;
                self.graph_running = false;
                self.graph_history.clear();
                self.speed_rpm = None;
                self.bus_current = None;
                self.phase_u_current = None;
                self.phase_v_current = None;
                self.phase_w_current = None;
                self.measured_direction = None;
                self.duty_cycle = None;
                self.gate_voltage = None;
                self.status_message = String::from("Disconnected.");
                self.push_log(LogLevel::Info, format!("Disconnected from {}.", port));
                Command::none()
            }

            Message::CloseRequested => {
                if self.motor_enabled {
                    return Command::perform(
                        async {
                            rfd::AsyncMessageDialog::new()
                                .set_title("Motor is running")
                                .set_description(
                                    "The motor is currently running.\n\
                                     Stop the motor before exiting?"
                                )
                                .set_buttons(rfd::MessageButtons::YesNo)
                                .set_level(rfd::MessageLevel::Warning)
                                .show()
                                .await
                        },
                        |result| {
                            if result == rfd::MessageDialogResult::Yes {
                                Message::StopThenExit
                            } else {
                                Message::DoExit
                            }
                        },
                    );
                }
                iced::window::close(iced::window::Id::MAIN)
            }

            Message::StopThenDisconnect => {
                let Some(handle) = self.serial_handle.clone() else {
                    return Command::perform(async {}, |_| Message::DoDisconnect);
                };
                Command::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            let _ = crate::serial::scpi_send(
                                &handle,
                                crate::scpi::commands::CONF_ENABLE_OFF,
                            );
                        })
                        .await;
                    },
                    |_| Message::DoDisconnect,
                )
            }

            Message::StopThenExit => {
                let Some(handle) = self.serial_handle.clone() else {
                    return iced::window::close(iced::window::Id::MAIN);
                };
                Command::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            let _ = crate::serial::scpi_send(
                                &handle,
                                crate::scpi::commands::CONF_ENABLE_OFF,
                            );
                        })
                        .await;
                    },
                    |_| Message::DoExit,
                )
            }

            Message::DoExit => iced::window::close(iced::window::Id::MAIN),

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
                // Read current motor state from board so UI reflects reality after reconnect.
                let Some(handle) = self.serial_handle.clone() else {
                    return Command::none();
                };
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            use crate::serial::scpi_query;
                            use crate::scpi::commands;
                            let en_resp = scpi_query(&handle, commands::CONF_ENABLE_QUERY)
                                .unwrap_or_default();
                            let enabled = en_resp.trim() == "1";
                            let freq_resp = scpi_query(&handle, commands::CONF_FREQUENCY_QUERY)
                                .unwrap_or_default();
                            let frequency: u32 = freq_resp.trim().parse().unwrap_or(20_000);
                            let dir_resp = scpi_query(&handle, commands::CONF_DIR_QUERY)
                                .unwrap_or_default();
                            let direction = if dir_resp.trim().to_uppercase().starts_with("REVE") {
                                Direction::Reverse
                            } else {
                                Direction::Forward
                            };
                            Ok::<_, String>((enabled, frequency, direction))
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |result| match result {
                        Ok((enabled, frequency, direction)) => {
                            Message::MotorStateRefreshed { enabled, frequency, direction }
                        }
                        Err(_) => Message::StatusMessage(
                            String::from("Could not read motor state from board."),
                        ),
                    },
                )
            }

            // ---- Motor control ----
            Message::EnableChanged(en) => {
                if self.motor_busy {
                    return Command::none();
                }
                self.motor_enabled = en;
                let Some(handle) = self.serial_handle.clone() else {
                    return Command::none();
                };
                let cmd = if en {
                    crate::scpi::commands::CONF_ENABLE_ON
                } else {
                    crate::scpi::commands::CONF_ENABLE_OFF
                };
                self.push_log(
                    LogLevel::Info,
                    format!("Enable → {}", if en { "ON" } else { "OFF" }),
                );
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::serial::scpi_send(&handle, cmd)?;
                            let resp = crate::serial::scpi_query(
                                &handle,
                                crate::scpi::commands::CONF_ENABLE_QUERY,
                            )?;
                            Ok(resp.trim() != "0")
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::EnableConfirmed,
                )
            }

            Message::FrequencyChanged(freq) => {
                // Ignore slider drag while motor is running — keeps UI in sync with board.
                if !self.motor_enabled {
                    self.motor_frequency = freq;
                    self.motor_frequency_input = format!("{:.0}", freq);
                }
                Command::none()
            }

            Message::FrequencyInputChanged(s) => {
                self.motor_frequency_input = s;
                Command::none()
            }

            Message::FrequencySubmit => {
                if self.motor_enabled {
                    self.status_message =
                        String::from("Disable the motor before changing frequency.");
                    return Command::none();
                }
                let parse_result = self
                    .motor_frequency_input
                    .trim()
                    .parse::<f32>()
                    .ok()
                    .and_then(|hz| crate::scpi::validate_frequency(hz).ok());
                match parse_result {
                    Some(valid_hz) => {
                        self.motor_frequency = valid_hz as f32;
                        self.motor_frequency_input = valid_hz.to_string();
                        let Some(handle) = self.serial_handle.clone() else {
                            return Command::none();
                        };
                        let cmd = crate::scpi::commands::conf_frequency(valid_hz);
                        self.push_log(LogLevel::Info, format!("Frequency → {} Hz", valid_hz));
                        Command::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    crate::serial::scpi_send(&handle, &cmd)?;
                                    let resp = crate::serial::scpi_query(
                                        &handle,
                                        crate::scpi::commands::CONF_FREQUENCY_QUERY,
                                    )?;
                                    resp.trim()
                                        .parse::<u32>()
                                        .map_err(|_| format!("Bad freq response: '{}'", resp))
                                })
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                            },
                            Message::FrequencyConfirmed,
                        )
                    }
                    None => {
                        self.status_message = format!(
                            "Invalid frequency — enter a value between {} and {} Hz.",
                            crate::scpi::FREQ_MIN_HZ,
                            crate::scpi::FREQ_MAX_HZ
                        );
                        Command::none()
                    }
                }
            }

            Message::DirectionChanged(dir) => {
                if self.motor_busy {
                    return Command::none();
                }
                self.motor_direction = dir.clone();
                self.motor_busy = true;
                let Some(handle) = self.serial_handle.clone() else {
                    self.motor_busy = false;
                    return Command::none();
                };
                let was_enabled = self.motor_enabled;
                let dir_cmd = match &dir {
                    Direction::Forward => crate::scpi::commands::CONF_DIR_FORWARD,
                    Direction::Reverse => crate::scpi::commands::CONF_DIR_REVERSE,
                };
                self.push_log(
                    LogLevel::Info,
                    format!("Direction → {} — cycling enable…", dir),
                );
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            use std::time::Duration;

                            // 1. Send direction command
                            crate::serial::scpi_send(&handle, dir_cmd)?;

                            // 2. Poll speed until motor stops (max ~1.5 s)
                            for _ in 0..10 {
                                std::thread::sleep(Duration::from_millis(150));
                                if let Ok(resp) = crate::serial::scpi_query(
                                    &handle,
                                    crate::scpi::commands::MEAS_SPEED,
                                ) {
                                    let speed: f32 = resp.trim().parse().unwrap_or(999.0);
                                    if speed.abs() < 10.0 {
                                        break;
                                    }
                                }
                            }

                            // 3. Ensure enable is off
                            crate::serial::scpi_send(
                                &handle,
                                crate::scpi::commands::CONF_ENABLE_OFF,
                            )?;

                            // 4. If motor was running, turn it back on
                            if was_enabled {
                                std::thread::sleep(Duration::from_millis(100));
                                crate::serial::scpi_send(
                                    &handle,
                                    crate::scpi::commands::CONF_ENABLE_ON,
                                )?;
                            }

                            // 5. Confirm direction (now settled)
                            let resp = crate::serial::scpi_query(
                                &handle,
                                crate::scpi::commands::CONF_DIR_QUERY,
                            )?;
                            let upper = resp.trim().to_uppercase();
                            let confirmed_dir = if upper.starts_with("FORW") {
                                Direction::Forward
                            } else if upper.starts_with("REVE") {
                                Direction::Reverse
                            } else {
                                return Err(format!("Unexpected direction: '{}'", resp));
                            };

                            Ok((confirmed_dir, was_enabled))
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    Message::DirectionConfirmed,
                )
            }

            // ---- Measurements ----
            Message::QueryMeasurements => {
                let Some(handle) = self.serial_handle.clone() else {
                    return Command::none();
                };
                // Compute effective channels: power(7) requires current(1) + voltage(6)
                let mut eff = self.graph_channels;
                if eff[7] { eff[1] = true; eff[6] = true; }
                // When Motor Control tab is active, poll all real measurement channels
                // (de-duplicates naturally: graph queries are already included in eff)
                if self.active_panel == Panel::MotorControl {
                    for i in 0..7 { eff[i] = true; }
                }
                let busy = self.motor_busy;
                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            use crate::serial::scpi_query;
                            use crate::scpi::commands;
                            let pf = |r: Result<String, String>| -> Option<f32> {
                                r.ok().and_then(|s| s.trim().parse().ok())
                            };
                            // Always query direction: disconnect probe + motor panel display.
                            let dir_result = scpi_query(&handle, commands::MEAS_DIRECTION);
                            if let Err(ref e) = dir_result {
                                let lower = e.to_lowercase();
                                if lower.starts_with("write error") || lower.starts_with("read error") {
                                    return Err(format!("io:{}", e));
                                }
                            }
                            let direction = dir_result.ok().map(|s| s.trim().to_string());
                            // Only query channels that are selected (or needed)
                            let speed = if eff[0] || busy {
                                pf(scpi_query(&handle, commands::MEAS_SPEED))
                            } else { None };
                            let bus_current = if eff[1] {
                                pf(scpi_query(&handle, commands::MEAS_CURRENT_IBUS))
                            } else { None };
                            let phase_u = if eff[2] {
                                pf(scpi_query(&handle, commands::MEAS_CURRENT_IPHU))
                            } else { None };
                            let phase_v = if eff[3] {
                                pf(scpi_query(&handle, commands::MEAS_CURRENT_IPHV))
                            } else { None };
                            let phase_w = if eff[4] {
                                pf(scpi_query(&handle, commands::MEAS_CURRENT_IPHW))
                            } else { None };
                            let duty_cycle = if eff[5] {
                                pf(scpi_query(&handle, commands::MEAS_DUTY_CYCLE))
                            } else { None };
                            let voltage = if eff[6] {
                                pf(scpi_query(&handle, commands::MEAS_VOLTAGE))
                            } else { None };
                            Ok::<Message, String>(Message::MeasurementsReceived {
                                speed,
                                bus_current,
                                phase_u,
                                phase_v,
                                phase_w,
                                direction,
                                duty_cycle,
                                voltage,
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |result| match result {
                        Ok(msg) => msg,
                        Err(e) if e.starts_with("io:") => Message::DeviceDisconnected,
                        Err(e) => Message::StatusMessage(format!("Measurement error: {}", e)),
                    },
                )
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
                // Record graph sample
                if self.graph_running {
                    let dur = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default();
                    let wall_time_ms = dur.as_millis() as u64;
                    let t = (dur.as_secs_f64() - self.graph_start_secs) as f32;
                    let power = match (self.bus_current, self.gate_voltage) {
                        (Some(i), Some(v)) if self.graph_channels[7] => Some(i * v),
                        _ => None,
                    };
                    let ch = &self.graph_channels;
                    self.graph_history.push_back(GraphSample {
                        t,
                        wall_time_ms,
                        values: [
                            if ch[0] { self.speed_rpm } else { None },
                            if ch[1] { self.bus_current } else { None },
                            if ch[2] { self.phase_u_current } else { None },
                            if ch[3] { self.phase_v_current } else { None },
                            if ch[4] { self.phase_w_current } else { None },
                            if ch[5] { self.duty_cycle } else { None },
                            if ch[6] { self.gate_voltage } else { None },
                            power,
                        ],
                    });
                    const MAX_SAMPLES: usize = 3_000;
                    while self.graph_history.len() > MAX_SAMPLES {
                        self.graph_history.pop_front();
                    }
                }
                Command::none()
            }

            // ---- Motor control confirmations ----
            Message::EnableConfirmed(Ok(actual)) => {
                self.motor_enabled = actual;
                self.push_log(
                    LogLevel::Info,
                    format!("Enable confirmed: {}", if actual { "ON" } else { "OFF" }),
                );
                Command::none()
            }
            Message::EnableConfirmed(Err(e)) => {
                self.push_log(LogLevel::Error, format!("Enable command failed: {}", e));
                Command::none()
            }

            Message::FrequencyConfirmed(Ok(hz)) => {
                self.motor_frequency = hz as f32;
                self.motor_frequency_input = hz.to_string();
                self.push_log(LogLevel::Info, format!("Frequency confirmed: {} Hz", hz));
                Command::none()
            }
            Message::FrequencyConfirmed(Err(e)) => {
                self.push_log(LogLevel::Error, format!("Frequency command failed: {}", e));
                Command::none()
            }

            Message::DirectionConfirmed(Ok((dir, enabled))) => {
                self.motor_direction = dir.clone();
                self.motor_enabled = enabled;
                self.motor_busy = false;
                self.push_log(
                    LogLevel::Info,
                    format!(
                        "Direction confirmed: {} (motor {})",
                        dir,
                        if enabled { "ON" } else { "OFF" }
                    ),
                );
                Command::none()
            }
            Message::DirectionConfirmed(Err(e)) => {
                self.motor_busy = false;
                self.push_log(LogLevel::Error, format!("Direction change failed: {}", e));
                Command::none()
            }

            Message::MotorStateRefreshed { enabled, frequency, direction } => {
                self.motor_enabled = enabled;
                self.motor_frequency = frequency as f32;
                self.motor_frequency_input = frequency.to_string();
                self.motor_direction = direction.clone();
                self.push_log(
                    LogLevel::Info,
                    format!(
                        "Motor state on connect: {} | {} Hz | {}",
                        if enabled { "ON" } else { "OFF" },
                        frequency,
                        direction,
                    ),
                );
                Command::none()
            }

            Message::DeviceDisconnected => {
                let port = self.selected_port.clone().unwrap_or_default();
                self.serial_handle = None;
                self.connection = ConnectionState::Disconnected;
                self.firmware_version = None;
                self.idn_manufacturer = None;
                self.idn_model = None;
                self.idn_serial = None;
                self.motor_enabled = false;
                self.motor_busy = false;
                self.graph_running = false;
                self.graph_history.clear();
                self.speed_rpm = None;
                self.bus_current = None;
                self.phase_u_current = None;
                self.phase_v_current = None;
                self.phase_w_current = None;
                self.measured_direction = None;
                self.duty_cycle = None;
                self.gate_voltage = None;
                let msg = format!("Device disconnected unexpectedly ({})", port);
                self.push_log(LogLevel::Error, msg.clone());
                self.status_message = msg;
                Command::none()
            }

            // ---- Graphs panel ----
            Message::GraphChannelToggled(idx) => {
                if idx < NUM_CHANNELS {
                    self.graph_channels[idx] ^= true;
                    // System Power (7) requires System Current (1) and System Voltage (6)
                    if idx == 7 && self.graph_channels[7] {
                        self.graph_channels[1] = true;
                        self.graph_channels[6] = true;
                    }
                }
                Command::none()
            }
            Message::GraphPollRateChanged(hz) => {
                self.graph_poll_hz = hz;
                Command::none()
            }
            Message::GraphStartStop => {
                if self.graph_running {
                    self.graph_running = false;
                } else {
                    self.graph_running = true;
                    self.graph_history.clear();
                    self.graph_start_secs = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64();
                }
                Command::none()
            }
            Message::GraphModeToggled => {
                self.graph_mode = match self.graph_mode {
                    GraphMode::Overlay => GraphMode::Individual,
                    GraphMode::Individual => GraphMode::Overlay,
                };
                Command::none()
            }
            Message::GraphDownloadCsv => {
                if self.graph_history.is_empty() {
                    self.status_message = String::from("No data to export.");
                    return Command::none();
                }
                let history = self.graph_history.clone();
                Command::perform(
                    async move {
                        use std::fmt::Write as FmtWrite;

                        // Build CSV with snake_case headers
                        let mut csv = String::new();
                        writeln!(
                            csv,
                            "wall_time_ms,t_s,speed_rpm,system_current_a,\
                             phase_u_current_a,phase_v_current_a,phase_w_current_a,\
                             duty_cycle_pct,system_voltage_v,system_power_w"
                        ).unwrap();
                        for sample in &history {
                            write!(csv, "{},{:.4}", sample.wall_time_ms, sample.t).unwrap();
                            for v in &sample.values {
                                match v {
                                    Some(f) => write!(csv, ",{:.6}", f).unwrap(),
                                    None => write!(csv, ",").unwrap(),
                                }
                            }
                            writeln!(csv).unwrap();
                        }

                        // Show native save-file dialog
                        let ts = history.front().map(|s| s.wall_time_ms).unwrap_or(0);
                        let default_name = format!("nevc_mtr1_{}.csv", ts);
                        let handle = rfd::AsyncFileDialog::new()
                            .set_title("Save CSV")
                            .set_file_name(&default_name)
                            .add_filter("CSV files", &["csv"])
                            .save_file()
                            .await;

                        let Some(handle) = handle else {
                            return Err("cancelled".to_string());
                        };

                        tokio::fs::write(handle.path(), csv.as_bytes())
                            .await
                            .map(|_| handle.path().to_string_lossy().into_owned())
                            .map_err(|e| e.to_string())
                    },
                    |result| match result {
                        Ok(path) => Message::StatusMessage(format!("Exported: {}", path)),
                        Err(e) if e == "cancelled" => Message::StatusMessage(String::from("Export cancelled.")),
                        Err(e) => Message::StatusMessage(format!("Export failed: {}", e)),
                    },
                )
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

    fn subscription(&self) -> Subscription<Message> {
        let window_events = iced::event::listen_with(|event, _status| {
            if let iced::Event::Window(_id, iced::window::Event::CloseRequested) = event {
                Some(Message::CloseRequested)
            } else {
                None
            }
        });

        if self.connection == ConnectionState::Connected {
            let hz = if self.graph_running {
                self.graph_poll_hz.clamp(1.0, 50.0)
            } else if self.active_panel == Panel::MotorControl {
                2.0 // 2 Hz while on Motor Control tab
            } else {
                0.5 // 2-second disconnect probe on other tabs
            };
            let poll = iced::time::every(std::time::Duration::from_secs_f32(1.0 / hz))
                .map(|_| Message::QueryMeasurements);
            Subscription::batch([window_events, poll])
        } else {
            window_events
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
