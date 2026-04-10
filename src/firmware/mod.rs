// ---------------------------------------------------------------------------
// Firmware management - FirmwareConfig, Arduino CLI, compile + upload
// ---------------------------------------------------------------------------

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::io;

// ---------------------------------------------------------------------------
// FirmwareConfig - 26 user-settable parameters from main/config.h
// ---------------------------------------------------------------------------

/// All 26 user-configurable parameters in the NEVC-MTR1 firmware.
/// The field order matches the IDN serial hex-field index order.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct FirmwareConfig {
    // [0] Motor
    pub motor_poles: u32,
    // [1] Gate switching frequency (Hz)
    pub f_mosfet: u32,
    // [2] Dead-time between switching actions (ns)
    pub dead_time: u32,
    // [3] Emulate hall effect sensor signals (bool)
    pub emulate_hall: bool,
    // [4] Electrical rotational frequency for emulated motor (Hz)
    pub tim3_freq: u32,
    // [5] Ticks without hall change before motor considered stopped
    pub commutation_ticks_stopped: u32,
    // [6] Turn-off mode: 0=COAST, 1=RAMP
    pub turn_off_mode: u32,
    // [7] In-line phase current sense amplifier gain
    pub iphase_gain: u32,
    // [8] Phase current sense resistor value (µΩ)
    pub iphase_sense_resistor: u32,
    // [9] Hi-side bus current sense amplifier gain
    pub ibus_gain: u32,
    // [10] Bus current sense resistor value (µΩ)
    pub ibus_sense_resistor: u32,
    // [11] Bus current warning threshold (ADC register, 0–1023)
    pub ibus_warning_threshold: u32,
    // [12] Bus current error threshold (ADC register, 0–1023)
    pub ibus_error_threshold: u32,
    // [13] Enable protective action when error threshold exceeded (bool)
    pub ibus_fault_enable: bool,
    // [14] Speed control method: 0=OPEN_LOOP, 1=CLOSED_LOOP
    pub speed_control_method: u32,
    // [15] Speed controller loop period (PWM ticks, 1–255)
    pub speed_controller_time_base: u32,
    // [16] Max speed reference change per loop iteration (open loop)
    pub speed_controller_max_delta: u32,
    // [17] Maximum motor speed setpoint for closed-loop control (hall Hz)
    pub speed_controller_max_speed: u32,
    // [18] PID proportional gain (i16 range)
    pub pid_k_p: i32,
    // [19] PID integral gain (i16 range)
    pub pid_k_i: i32,
    // [20] Enable derivative term in PID (bool)
    pub pid_k_d_enable: bool,
    // [21] PID derivative gain (i16 range)
    pub pid_k_d: i32,
    // [22] VBUS divider top resistor (Ω)
    pub vbus_rtop: u32,
    // [23] VBUS divider bottom resistor (Ω)
    pub vbus_rbottom: u32,
    // [24] Wait for inverter board connection before starting (bool)
    pub wait_for_board: bool,
    // [25] Report errors immediately over serial without query (bool)
    pub remote_debug_mode: bool,
}

impl Default for FirmwareConfig {
    /// Repo defaults from main/config.h on the main branch.
    fn default() -> Self {
        Self {
            motor_poles: 8,
            f_mosfet: 20000,
            dead_time: 350,
            emulate_hall: false,
            tim3_freq: 200,
            commutation_ticks_stopped: 6000,
            turn_off_mode: 1,        // TURN_OFF_MODE_RAMP
            iphase_gain: 20,
            iphase_sense_resistor: 2500,
            ibus_gain: 50,
            ibus_sense_resistor: 4000,
            ibus_warning_threshold: 307,
            ibus_error_threshold: 410,
            ibus_fault_enable: true,
            speed_control_method: 0, // SPEED_CONTROL_OPEN_LOOP
            speed_controller_time_base: 200,
            speed_controller_max_delta: 1,
            speed_controller_max_speed: 400,
            pid_k_p: 100,
            pid_k_i: 10,
            pid_k_d_enable: true,
            pid_k_d: 0,
            vbus_rtop: 100000,
            vbus_rbottom: 6200,
            wait_for_board: true,
            remote_debug_mode: false,
        }
    }
}

impl FirmwareConfig {
    /// Parse a 26-field hyphen-separated hex IDN serial string.
    /// Each hex field maps 1:1 to the struct fields in index order.
    pub fn from_idn_serial(serial: &str) -> Option<Self> {
        let fields: Vec<&str> = serial.split('-').collect();
        if fields.len() < 26 {
            return None;
        }
        let pu = |s: &str| u32::from_str_radix(s.trim(), 16).ok();
        // Signed i16 values in two's complement hex
        let ps = |s: &str| -> Option<i32> {
            u32::from_str_radix(s.trim(), 16)
                .ok()
                .map(|v| (v as u16) as i16 as i32)
        };
        let pb = |s: &str| pu(s).map(|v| v != 0);

        Some(Self {
            motor_poles:               pu(fields[0])?,
            f_mosfet:                  pu(fields[1])?,
            dead_time:                 pu(fields[2])?,
            emulate_hall:              pb(fields[3])?,
            tim3_freq:                 pu(fields[4])?,
            commutation_ticks_stopped: pu(fields[5])?,
            turn_off_mode:             pu(fields[6])?,
            iphase_gain:               pu(fields[7])?,
            iphase_sense_resistor:     pu(fields[8])?,
            ibus_gain:                 pu(fields[9])?,
            ibus_sense_resistor:       pu(fields[10])?,
            ibus_warning_threshold:    pu(fields[11])?,
            ibus_error_threshold:      pu(fields[12])?,
            ibus_fault_enable:         pb(fields[13])?,
            speed_control_method:      pu(fields[14])?,
            speed_controller_time_base: pu(fields[15])?,
            speed_controller_max_delta: pu(fields[16])?,
            speed_controller_max_speed: pu(fields[17])?,
            pid_k_p:                   ps(fields[18])?,
            pid_k_i:                   ps(fields[19])?,
            pid_k_d_enable:            pb(fields[20])?,
            pid_k_d:                   ps(fields[21])?,
            vbus_rtop:                 pu(fields[22])?,
            vbus_rbottom:              pu(fields[23])?,
            wait_for_board:            pb(fields[24])?,
            remote_debug_mode:         pb(fields[25])?,
        })
    }

    /// Produce the 26-field hex IDN serial string for this config.
    pub fn to_idn_serial(&self) -> String {
        let bool_hex = |b: bool| if b { "1" } else { "0" };
        format!(
            "{:X}-{:X}-{:X}-{}-{:X}-{:X}-{:X}-{:X}-{:X}-{:X}-{:X}-{:X}-{:X}-{}-{:X}-{:X}-{:X}-{:X}-{:X}-{:X}-{}-{:X}-{:X}-{:X}-{}-{}",
            self.motor_poles,
            self.f_mosfet,
            self.dead_time,
            bool_hex(self.emulate_hall),
            self.tim3_freq,
            self.commutation_ticks_stopped,
            self.turn_off_mode,
            self.iphase_gain,
            self.iphase_sense_resistor,
            self.ibus_gain,
            self.ibus_sense_resistor,
            self.ibus_warning_threshold,
            self.ibus_error_threshold,
            bool_hex(self.ibus_fault_enable),
            self.speed_control_method,
            self.speed_controller_time_base,
            self.speed_controller_max_delta,
            self.speed_controller_max_speed,
            (self.pid_k_p as i16) as u16,
            (self.pid_k_i as i16) as u16,
            bool_hex(self.pid_k_d_enable),
            (self.pid_k_d as i16) as u16,
            self.vbus_rtop,
            self.vbus_rbottom,
            bool_hex(self.wait_for_board),
            bool_hex(self.remote_debug_mode),
        )
    }

    /// Convert the config into a 26-element Vec of display strings for UI text inputs.
    pub fn to_input_strings(&self) -> Vec<String> {
        let b = |v: bool| if v { "true".to_string() } else { "false".to_string() };
        vec![
            self.motor_poles.to_string(),
            self.f_mosfet.to_string(),
            self.dead_time.to_string(),
            b(self.emulate_hall),
            self.tim3_freq.to_string(),
            self.commutation_ticks_stopped.to_string(),
            self.turn_off_mode.to_string(),
            self.iphase_gain.to_string(),
            self.iphase_sense_resistor.to_string(),
            self.ibus_gain.to_string(),
            self.ibus_sense_resistor.to_string(),
            self.ibus_warning_threshold.to_string(),
            self.ibus_error_threshold.to_string(),
            b(self.ibus_fault_enable),
            self.speed_control_method.to_string(),
            self.speed_controller_time_base.to_string(),
            self.speed_controller_max_delta.to_string(),
            self.speed_controller_max_speed.to_string(),
            self.pid_k_p.to_string(),
            self.pid_k_i.to_string(),
            b(self.pid_k_d_enable),
            self.pid_k_d.to_string(),
            self.vbus_rtop.to_string(),
            self.vbus_rbottom.to_string(),
            b(self.wait_for_board),
            b(self.remote_debug_mode),
        ]
    }

    /// Try to parse 26 input strings back into a `FirmwareConfig`.
    /// Returns `Err` with index and message of the first field that fails to parse.
    pub fn try_from_inputs(inputs: &[String]) -> Result<Self, (usize, String)> {
        if inputs.len() < 26 {
            return Err((0, "Not enough parameter inputs".to_string()));
        }
        let pu = |idx: usize| -> Result<u32, (usize, String)> {
            inputs[idx].trim().parse::<u32>().map_err(|_| {
                (idx, format!("'{}' is not a valid unsigned integer", inputs[idx]))
            })
        };
        let ps = |idx: usize| -> Result<i32, (usize, String)> {
            let s = inputs[idx].trim();
            s.parse::<i32>()
                .map_err(|_| (idx, format!("'{}' is not a valid integer", inputs[idx])))
        };
        let pb = |idx: usize| -> Result<bool, (usize, String)> {
            let s = inputs[idx].trim().to_lowercase();
            match s.as_str() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err((idx, format!("'{}' is not true/false", inputs[idx]))),
            }
        };

        Ok(Self {
            motor_poles:               pu(0)?,
            f_mosfet:                  pu(1)?,
            dead_time:                 pu(2)?,
            emulate_hall:              pb(3)?,
            tim3_freq:                 pu(4)?,
            commutation_ticks_stopped: pu(5)?,
            turn_off_mode:             pu(6)?,
            iphase_gain:               pu(7)?,
            iphase_sense_resistor:     pu(8)?,
            ibus_gain:                 pu(9)?,
            ibus_sense_resistor:       pu(10)?,
            ibus_warning_threshold:    pu(11)?,
            ibus_error_threshold:      pu(12)?,
            ibus_fault_enable:         pb(13)?,
            speed_control_method:      pu(14)?,
            speed_controller_time_base: pu(15)?,
            speed_controller_max_delta: pu(16)?,
            speed_controller_max_speed: pu(17)?,
            pid_k_p:                   ps(18)?,
            pid_k_i:                   ps(19)?,
            pid_k_d_enable:            pb(20)?,
            pid_k_d:                   ps(21)?,
            vbus_rtop:                 pu(22)?,
            vbus_rbottom:              pu(23)?,
            wait_for_board:            pb(24)?,
            remote_debug_mode:         pb(25)?,
        })
    }
}

// ---------------------------------------------------------------------------
// config.h patching
// ---------------------------------------------------------------------------

/// Apply the config values to a config.h source file string, replacing
/// each `#define PARAM value` line with the new value.
pub fn patch_config_h(source: &str, config: &FirmwareConfig) -> String {
    // (param_name, replacement_value_text)
    let patches: &[(&str, String)] = &[
        ("MOTOR_POLES",                 config.motor_poles.to_string()),
        ("F_MOSFET",                    format!("{}UL", config.f_mosfet)),
        ("DEAD_TIME",                   format!("{}UL", config.dead_time)),
        ("EMULATE_HALL",                bool_define(config.emulate_hall)),
        ("TIM3_FREQ",                   format!("{}UL", config.tim3_freq)),
        ("COMMUTATION_TICKS_STOPPED",   config.commutation_ticks_stopped.to_string()),
        ("TURN_OFF_MODE",               if config.turn_off_mode == 0 {
                                            "TURN_OFF_MODE_COAST".to_string()
                                        } else {
                                            "TURN_OFF_MODE_RAMP".to_string()
                                        }),
        ("IPHASE_GAIN",                 config.iphase_gain.to_string()),
        ("IPHASE_SENSE_RESISTOR",       config.iphase_sense_resistor.to_string()),
        ("IBUS_GAIN",                   config.ibus_gain.to_string()),
        ("IBUS_SENSE_RESISTOR",         config.ibus_sense_resistor.to_string()),
        ("IBUS_WARNING_THRESHOLD",      config.ibus_warning_threshold.to_string()),
        ("IBUS_ERROR_THRESHOLD",        config.ibus_error_threshold.to_string()),
        ("IBUS_FAULT_ENABLE",           bool_define(config.ibus_fault_enable)),
        ("SPEED_CONTROL_METHOD",        if config.speed_control_method == 0 {
                                            "SPEED_CONTROL_OPEN_LOOP".to_string()
                                        } else {
                                            "SPEED_CONTROL_CLOSED_LOOP".to_string()
                                        }),
        ("SPEED_CONTROLLER_TIME_BASE",  config.speed_controller_time_base.to_string()),
        ("SPEED_CONTROLLER_MAX_DELTA",  config.speed_controller_max_delta.to_string()),
        ("SPEED_CONTROLLER_MAX_SPEED",  config.speed_controller_max_speed.to_string()),
        ("PID_K_P",                     config.pid_k_p.to_string()),
        ("PID_K_I",                     config.pid_k_i.to_string()),
        ("PID_K_D_ENABLE",              bool_define(config.pid_k_d_enable)),
        ("PID_K_D",                     config.pid_k_d.to_string()),
        ("VBUS_RTOP",                   config.vbus_rtop.to_string()),
        ("VBUS_RBOTTOM",                config.vbus_rbottom.to_string()),
        ("WAIT_FOR_BOARD",              bool_define(config.wait_for_board)),
        ("REMOTE_DEBUG_MODE",           bool_define(config.remote_debug_mode)),
    ];

    let mut lines: Vec<String> = source.lines().map(|l| l.to_string()).collect();

    for (name, new_val) in patches {
        let prefix = format!("#define {} ", name);
        for line in lines.iter_mut() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&prefix) {
                let indent_len = line.len() - trimmed.len();
                let indent = &line[..indent_len];
                let after_prefix = &trimmed[prefix.len()..];
                // Keep everything after the old value (comments, etc.)
                let old_val_end = after_prefix
                    .find(|c: char| c == '/' || c == ' ' || c == '\t')
                    .unwrap_or(after_prefix.len());
                let tail = &after_prefix[old_val_end..];
                *line = format!("{}{}{}{}", indent, prefix, new_val, tail);
                break;
            }
        }
    }

    let joined = lines.join("\n");
    if source.ends_with('\n') {
        joined + "\n"
    } else {
        joined
    }
}

fn bool_define(v: bool) -> String {
    if v { "TRUE".to_string() } else { "FALSE".to_string() }
}

// ---------------------------------------------------------------------------
// Application data directory
// ---------------------------------------------------------------------------

pub fn app_data_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("nevc_mtr1_gui")
}

pub fn tools_dir() -> PathBuf {
    app_data_dir().join("tools")
}

pub fn firmware_dir() -> PathBuf {
    app_data_dir().join("firmware")
}

// ---------------------------------------------------------------------------
// Arduino CLI - download and manage
// ---------------------------------------------------------------------------

const ARDUINO_CLI_EXE: &str = "arduino-cli.exe";
const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/arduino/arduino-cli/releases/latest";

/// Returns the path to arduino-cli.exe, downloading it if necessary.
/// Calls `progress(msg)` to report download or install steps.
pub fn ensure_arduino_cli(mut progress: impl FnMut(&str)) -> anyhow::Result<PathBuf> {
    let dir = tools_dir();
    let exe = dir.join(ARDUINO_CLI_EXE);

    if exe.exists() {
        progress("arduino-cli.exe already present.");
        return Ok(exe);
    }

    progress("arduino-cli.exe not found - fetching download URL from GitHub...");
    std::fs::create_dir_all(&dir)?;

    // Fetch latest release JSON
    let client = reqwest::blocking::Client::builder()
        .user_agent("nevc_mtr1_gui/1.0")
        .build()?;
    let release: serde_json::Value = client.get(GITHUB_RELEASES_API).send()?.json()?;

    // Find asset URL for Windows 64-bit
    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No assets in release JSON"))?;
    let asset_url = assets
        .iter()
        .find_map(|a| {
            let name = a["name"].as_str().unwrap_or("");
            if name.contains("Windows") && name.contains("64bit") && name.ends_with(".zip") {
                a["browser_download_url"].as_str().map(|s| s.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find Windows 64-bit arduino-cli zip in release"))?;

    progress(&format!("Downloading: {}", asset_url));
    let zip_bytes = client.get(&asset_url).send()?.bytes()?;

    progress("Extracting arduino-cli.exe…");
    extract_file_from_zip(&zip_bytes, "arduino-cli.exe", &exe)?;

    progress(&format!("arduino-cli.exe saved to {}", exe.display()));
    Ok(exe)
}

/// Install the arduino:avr core if not already installed.
pub fn ensure_avr_core(cli: &Path, mut progress: impl FnMut(&str)) -> anyhow::Result<()> {
    progress("Checking arduino:avr core…");

    // Check if already installed
    let list = std::process::Command::new(cli)
        .args(["core", "list", "--format", "text"])
        .output();

    let already = match list {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("arduino:avr"),
        Err(_) => false,
    };

    if already {
        progress("arduino:avr core already installed.");
        return Ok(());
    }

    progress("Installing arduino:avr core (this may take 1–2 minutes)…");
    let output = std::process::Command::new(cli)
        .args(["core", "install", "arduino:avr"])
        .output()
        .map_err(|e| anyhow::anyhow!("arduino-cli core install failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Core install error: {}", stderr));
    }
    progress("arduino:avr core installed.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Firmware source - download and cache
// ---------------------------------------------------------------------------

const FIRMWARE_REPO_ZIP_URL: &str =
    "https://github.com/Nexperia/NEVC-MTR1-t01/archive/refs/heads/main.zip";

/// Expected top-level directory name inside the downloaded ZIP archive.
const FIRMWARE_ZIP_ROOT: &str = "NEVC-MTR1-t01-main";

/// Download and extract the firmware source if not already cached.
/// Returns the path to the `main/` sketch directory.
pub fn ensure_firmware_source(mut progress: impl FnMut(&str)) -> anyhow::Result<PathBuf> {
    let dest = firmware_dir().join(FIRMWARE_ZIP_ROOT);
    let sketch = dest.join("main");
    let sentinel = sketch.join("main.ino");

    if sentinel.exists() {
        progress("Firmware source already cached.");
        return Ok(sketch);
    }

    progress("Downloading firmware source from GitHub…");
    std::fs::create_dir_all(&firmware_dir())?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("nevc_mtr1_gui/1.0")
        .build()?;
    let zip_bytes = client
        .get(FIRMWARE_REPO_ZIP_URL)
        .send()
        .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?
        .bytes()
        .map_err(|e| anyhow::anyhow!("Download read failed: {}", e))?;

    progress("Extracting firmware source…");
    extract_zip_to_dir(&zip_bytes, &firmware_dir())?;

    if !sentinel.exists() {
        return Err(anyhow::anyhow!(
            "Extraction completed but main/main.ino not found at expected location: {}",
            sentinel.display()
        ));
    }

    progress(&format!("Firmware source ready at {}", sketch.display()));
    Ok(sketch)
}

// ---------------------------------------------------------------------------
// Compile
// ---------------------------------------------------------------------------

/// Compile the sketch with arduino-cli. Returns combined stdout+stderr output.
pub fn compile_sketch(cli: &Path, sketch_dir: &Path, mut progress: impl FnMut(&str)) -> anyhow::Result<String> {
    progress(&format!("Compiling sketch at {}…", sketch_dir.display()));

    let output = std::process::Command::new(cli)
        .args([
            "compile",
            "--fqbn", "arduino:avr:leonardo",
            sketch_dir.to_str().unwrap_or("."),
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("arduino-cli compile failed to start: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    if !output.status.success() {
        return Err(anyhow::anyhow!("Compilation failed:\n{}", combined));
    }

    progress("Compilation successful.");
    Ok(combined)
}

// ---------------------------------------------------------------------------
// 1200-baud bootloader reset
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Upload
// ---------------------------------------------------------------------------

/// Upload the compiled sketch to the board.
///
/// Pass the **application port** (e.g. COM4) - `arduino-cli` handles the
/// 1200-baud reset and bootloader port detection internally for Leonardo/Caterina,
/// exactly the same way Arduino IDE does.
pub fn upload_sketch(
    cli: &Path,
    sketch_dir: &Path,
    port: &str,
    mut progress: impl FnMut(&str),
) -> anyhow::Result<()> {
    progress(&format!("Uploading to {} (arduino-cli will reset to bootloader)…", port));

    let output = std::process::Command::new(cli)
        .args([
            "upload",
            "--fqbn", "arduino:avr:leonardo",
            "-p", port,
            sketch_dir.to_str().unwrap_or("."),
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("arduino-cli upload failed to start: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        return Err(anyhow::anyhow!("Upload failed:\n{}{}", stdout, stderr));
    }

    progress("Upload successful.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Full pipeline
// ---------------------------------------------------------------------------

/// Run the full pipeline: ensure tools, patch config.h, compile, reset, upload.
/// Each step calls `progress` with a status string.
/// Returns `Err` with a detailed message if any step fails.
pub fn full_flash_pipeline(
    config: &FirmwareConfig,
    port: &str,
    mut progress: impl FnMut(&str),
) -> anyhow::Result<()> {
    // Step 1: ensure arduino-cli
    let cli = ensure_arduino_cli(&mut progress)?;

    // Step 2: ensure arduino:avr core
    ensure_avr_core(&cli, &mut progress)?;

    // Step 3: ensure firmware source
    let sketch_dir = ensure_firmware_source(&mut progress)?;

    // Step 4: patch config.h
    progress("Applying configuration to config.h…");
    let config_h_path = sketch_dir.join("config.h");
    let original = std::fs::read_to_string(&config_h_path)
        .map_err(|e| anyhow::anyhow!("Could not read config.h: {}", e))?;
    let patched = patch_config_h(&original, config);
    std::fs::write(&config_h_path, patched.as_bytes())
        .map_err(|e| anyhow::anyhow!("Could not write config.h: {}", e))?;
    progress("config.h updated.");

    // Step 5: compile
    compile_sketch(&cli, &sketch_dir, &mut progress)?;

    // Step 6: upload (arduino-cli handles the 1200-baud reset internally)
    upload_sketch(&cli, &sketch_dir, port, &mut progress)?;

    progress("Done! Firmware flashed successfully.");
    Ok(())
}

// ---------------------------------------------------------------------------
// ZIP helpers
// ---------------------------------------------------------------------------

fn extract_file_from_zip(zip_bytes: &[u8], file_name: &str, dest: &Path) -> anyhow::Result<()> {
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| anyhow::anyhow!("Could not open zip: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let base = Path::new(&entry_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if base == file_name {
            let parent = dest.parent().unwrap_or(dest);
            std::fs::create_dir_all(parent)?;
            let mut out = std::fs::File::create(dest)?;
            io::copy(&mut entry, &mut out)?;
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "'{}' not found in zip archive",
        file_name
    ))
}

fn extract_zip_to_dir(zip_bytes: &[u8], dest_dir: &Path) -> anyhow::Result<()> {
    let cursor = io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| anyhow::anyhow!("Could not open zip: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let raw_name = entry.name().to_string();
        // Sanitise path to prevent directory traversal
        let rel = raw_name
            .split('/')
            .filter(|c| !c.is_empty() && *c != ".." && *c != ".")
            .collect::<Vec<_>>()
            .join(std::path::MAIN_SEPARATOR_STR);
        let out_path = dest_dir.join(&rel);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            io::copy(&mut entry, &mut out)?;
        }
    }

    Ok(())
}
