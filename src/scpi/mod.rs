// ---------------------------------------------------------------------------
// SCPI layer — command strings, response parsing, validation
// Stage 2 will add the actual send/receive implementation.
// ---------------------------------------------------------------------------

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// IDN response
// ---------------------------------------------------------------------------

/// Parsed `*IDN?` response: `<Manufacturer>,<Model>,<Serial>,<FirmwareVersion>`
#[derive(Debug, Clone)]
pub struct IdnResponse {
    pub manufacturer: String,
    pub model: String,
    /// 26 hyphen-separated hex values encoding firmware configuration constants
    pub serial: String,
    pub firmware_version: String,
}

impl IdnResponse {
    /// Parse a raw `*IDN?` response string.
    pub fn parse(response: &str) -> Option<Self> {
        let parts: Vec<&str> = response.trim().splitn(4, ',').collect();
        if parts.len() < 4 {
            return None;
        }
        Some(Self {
            manufacturer: parts[0].trim().to_string(),
            model: parts[1].trim().to_string(),
            serial: parts[2].trim().to_string(),
            firmware_version: parts[3].trim().to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// SCPI command strings
// ---------------------------------------------------------------------------

pub mod commands {
    // Standard IEEE 488.2
    pub const IDN: &str = "*IDN?";
    pub const RST: &str = "*RST";

    // System error queue
    pub const SYS_ERROR: &str = "SYSTem:ERRor?";
    pub const SYS_ERROR_COUNT: &str = "SYSTem:ERRor:COUNt?";

    // Motor enable
    pub const CONF_ENABLE_ON: &str = "CONFigure:ENABle ON";
    pub const CONF_ENABLE_OFF: &str = "CONFigure:ENABle OFF";
    pub const CONF_ENABLE_QUERY: &str = "CONFigure:ENABle?";

    // Gate drive frequency
    pub fn conf_frequency(hz: u32) -> String {
        format!("CONFigure:FREQuency {}", hz)
    }
    pub const CONF_FREQUENCY_QUERY: &str = "CONFigure:FREQuency?";

    // Motor direction
    pub const CONF_DIR_FORWARD: &str = "CONFigure:DIREction FORWard";
    pub const CONF_DIR_REVERSE: &str = "CONFigure:DIREction REVErse";
    pub const CONF_DIR_QUERY: &str = "CONFigure:DIREction?";

    // Measurements
    pub const MEAS_SPEED: &str = "MEASure:SPEEd?";
    pub const MEAS_CURRENT_IBUS: &str = "MEASure:CURRent:IBUS?";
    pub const MEAS_CURRENT_IPHU: &str = "MEASure:CURRent:IPHU?";
    pub const MEAS_CURRENT_IPHV: &str = "MEASure:CURRent:IPHV?";
    pub const MEAS_CURRENT_IPHW: &str = "MEASure:CURRent:IPHW?";
    pub const MEAS_DIRECTION: &str = "MEASure:DIREction?";
    pub const MEAS_DUTY_CYCLE: &str = "MEASure:DUTYcycle?";
    pub const MEAS_VOLTAGE: &str = "MEASure:VOLTage?";
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

pub const FREQ_MIN_HZ: u32 = 7_183;
pub const FREQ_MAX_HZ: u32 = 100_000;

/// Validate and convert a frequency value to a u32 Hz value within spec range.
pub fn validate_frequency(hz: f32) -> Result<u32, String> {
    if !hz.is_finite() || hz < 0.0 {
        return Err(String::from("Frequency must be a positive number."));
    }
    let hz_u = hz.round() as u32;
    if hz_u < FREQ_MIN_HZ || hz_u > FREQ_MAX_HZ {
        Err(format!(
            "Frequency must be between {} Hz and {} Hz.",
            FREQ_MIN_HZ, FREQ_MAX_HZ
        ))
    } else {
        Ok(hz_u)
    }
}

// ---------------------------------------------------------------------------
// Response parsers
// ---------------------------------------------------------------------------

/// Parse a SCPI boolean response ("0" → false, "1" → true).
pub fn parse_bool(response: &str) -> Option<bool> {
    match response.trim() {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

/// Parse a SCPI float response.
pub fn parse_float(response: &str) -> Option<f32> {
    response.trim().parse::<f32>().ok()
}
