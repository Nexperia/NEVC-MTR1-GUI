#![allow(dead_code)]

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
// Serial connection handle (Stage 2 will add the real port handle)
// ---------------------------------------------------------------------------

pub struct SerialConnection {
    pub port_name: String,
    // Stage 2: Box<dyn serialport::SerialPort>
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
