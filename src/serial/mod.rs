#![allow(dead_code)]

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serialport::SerialPortType;

// ---------------------------------------------------------------------------
// Port information
// ---------------------------------------------------------------------------

/// Metadata about a detected serial port.
#[derive(Debug, Clone, PartialEq)]
pub struct PortInfo {
    /// OS-level port name, e.g. "COM3"
    pub name: String,
    /// Human-readable description (USB product string or type)
    pub description: String,
    /// True when the port matches the Arduino Leonardo USB VID/PID
    pub is_arduino: bool,
}

impl std::fmt::Display for PortInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_arduino {
            write!(f, "{} — Arduino Leonardo", self.name)
        } else {
            write!(f, "{} — {}", self.name, self.description)
        }
    }
}

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

// ---------------------------------------------------------------------------
// Serial handle — wraps an open port in Arc<Mutex> for safe sharing across tasks
// ---------------------------------------------------------------------------

/// Thread-safe handle to an open serial port.
/// Clone is cheap (just increments the Arc reference count).
#[derive(Clone)]
pub struct SerialHandle(pub Arc<Mutex<Box<dyn serialport::SerialPort + Send>>>);

impl std::fmt::Debug for SerialHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SerialHandle")
    }
}

// ---------------------------------------------------------------------------
// Open / close
// ---------------------------------------------------------------------------

/// Open the SCPI serial port with correct settings per spec:
/// 115200 baud, 8N1, no flow control, DTR+RTS enabled, 500 ms read timeout.
/// This is a blocking call — run via `tokio::task::spawn_blocking`.
pub fn open_port(port_name: &str) -> Result<SerialHandle, String> {
    let mut port = serialport::new(port_name, 115_200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|e| format!("Failed to open {}: {}", port_name, e))?;

    port.write_data_terminal_ready(true)
        .map_err(|e| format!("DTR error: {}", e))?;
    port.write_request_to_send(true)
        .map_err(|e| format!("RTS error: {}", e))?;

    Ok(SerialHandle(Arc::new(Mutex::new(port))))
}

// ---------------------------------------------------------------------------
// SCPI I/O — blocking, run via spawn_blocking
// ---------------------------------------------------------------------------

/// Send a SCPI query command and return the response line.
/// Strips trailing CR/LF and leading/trailing whitespace.
pub fn scpi_query(handle: &SerialHandle, cmd: &str) -> Result<String, String> {
    let mut port = handle
        .0
        .lock()
        .map_err(|_| String::from("Serial port mutex poisoned"))?;
    // Drain any stale bytes in the receive buffer (e.g. a previous late response)
    // before sending a new command, so we always read a fresh reply.
    let _ = port.clear(serialport::ClearBuffer::Input);
    // Write command terminated with LF
    let line = format!("{}\n", cmd);
    port.write_all(line.as_bytes())
        .map_err(|e| format!("Write error: {}", e))?;

    // Read bytes until LF or timeout
    let mut buf = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    loop {
        match port.read(&mut byte) {
            Ok(1) => {
                if byte[0] == b'\n' {
                    break;
                }
                if byte[0] != b'\r' {
                    buf.push(byte[0]);
                }
            }
            Ok(_) => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(format!("Read error: {}", e)),
        }
    }

    String::from_utf8(buf)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("Invalid UTF-8 in SCPI response: {}", e))
}

/// Send a SCPI command that produces no response (set commands).
pub fn scpi_send(handle: &SerialHandle, cmd: &str) -> Result<(), String> {
    let mut port = handle
        .0
        .lock()
        .map_err(|_| String::from("Serial port mutex poisoned"))?;
    let line = format!("{}\n", cmd);
    port.write_all(line.as_bytes())
        .map_err(|e| format!("Write error: {}", e))
}

// ---------------------------------------------------------------------------
// Arduino Leonardo USB identifiers
// ---------------------------------------------------------------------------

/// Nexperia / Arduino VID
const ARDUINO_VID: u16 = 0x2341;
/// Arduino Leonardo normal operation PID
const ARDUINO_LEONARDO_PID: u16 = 0x8036;
/// Arduino Leonardo in AVR109 bootloader mode PID
const ARDUINO_BOOTLOADER_PID: u16 = 0x0036;

// ---------------------------------------------------------------------------
// Port enumeration
// ---------------------------------------------------------------------------

/// Return all available serial ports, flagging Arduino Leonardo entries.
/// Called synchronously — wrap in `tokio::task::spawn_blocking` for async contexts.
pub fn list_ports() -> Vec<PortInfo> {
    let raw = match serialport::available_ports() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    raw.into_iter()
        .map(|p| {
            let is_arduino = is_arduino_leonardo(&p.port_type);
            let description = port_description(&p.port_type);
            PortInfo {
                name: p.port_name,
                description,
                is_arduino,
            }
        })
        .collect()
}

fn is_arduino_leonardo(port_type: &SerialPortType) -> bool {
    if let SerialPortType::UsbPort(usb) = port_type {
        usb.vid == ARDUINO_VID
            && (usb.pid == ARDUINO_LEONARDO_PID || usb.pid == ARDUINO_BOOTLOADER_PID)
    } else {
        false
    }
}

fn port_description(port_type: &SerialPortType) -> String {
    match port_type {
        SerialPortType::UsbPort(usb) => usb
            .product
            .clone()
            .unwrap_or_else(|| format!("USB {:04X}:{:04X}", usb.vid, usb.pid)),
        SerialPortType::BluetoothPort => String::from("Bluetooth"),
        SerialPortType::PciPort => String::from("PCI"),
        SerialPortType::Unknown => String::from("Unknown"),
    }
}
