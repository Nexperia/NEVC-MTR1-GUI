// ---------------------------------------------------------------------------
// Firmware management — avrdude integration and bootloader reset
// Stage 5 will add full implementation.
// ---------------------------------------------------------------------------

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Manages firmware flashing via `avrdude`.
pub struct FirmwareManager {
    /// Path to `avrdude.exe` (may be on PATH or bundled alongside the binary).
    avrdude_path: PathBuf,
    /// Path to the bundled `.hex` firmware file.
    firmware_hex_path: Option<PathBuf>,
}

impl FirmwareManager {
    pub fn new() -> Self {
        // Look for a bundled avrdude next to the executable first; fall back to PATH.
        let avrdude_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("avrdude.exe")))
            .filter(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from("avrdude"));

        let firmware_hex_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("firmware.hex")))
            .filter(|p| p.exists());

        Self {
            avrdude_path,
            firmware_hex_path,
        }
    }

    /// Return true if `avrdude` can be found and responds to `-?`.
    pub fn avrdude_available(&self) -> bool {
        std::process::Command::new(&self.avrdude_path)
            .arg("-?")
            .output()
            .is_ok()
    }

    /// Return true if a bundled firmware `.hex` is available.
    pub fn firmware_available(&self) -> bool {
        self.firmware_hex_path.is_some()
    }

    /// Return the bundled firmware path (if present).
    pub fn firmware_hex_path(&self) -> Option<&Path> {
        self.firmware_hex_path.as_deref()
    }

    /// Perform the 1200-baud reset trick on the given COM port to trigger
    /// the Arduino Leonardo's AVR109 bootloader.
    ///
    /// After the reset, the board re-enumerates with a new COM port (bootloader).
    /// The caller must wait and re-scan ports to find the new bootloader port.
    ///
    /// # Stage 5 TODO
    /// This is a placeholder — Stage 5 will implement the full async version
    /// using the `serialport` crate.
    pub fn trigger_bootloader_reset(&self, port_name: &str) -> Result<(), String> {
        // Open at 1200 baud, then immediately close — this signals the bootloader
        serialport::new(port_name, 1200)
            .timeout(std::time::Duration::from_millis(200))
            .open()
            .map_err(|e| format!("Could not open {} for reset: {}", port_name, e))?;
        // Closing happens automatically when the Box goes out of scope
        Ok(())
    }

    /// Flash firmware to a bootloader-mode port in the background.
    ///
    /// Progress lines are returned via the `on_progress` callback.
    ///
    /// # Stage 5 TODO — currently a stub that returns immediately.
    pub fn flash(
        &self,
        bootloader_port: &str,
        on_progress: impl Fn(String),
    ) -> Result<(), String> {
        let hex = self
            .firmware_hex_path
            .as_ref()
            .ok_or_else(|| String::from("No firmware .hex file found next to the application."))?;

        if !self.avrdude_available() {
            return Err(format!(
                "avrdude not found at '{}'. \
                 Install the Arduino IDE/CLI or place avrdude.exe next to this application.",
                self.avrdude_path.display()
            ));
        }

        on_progress(format!(
            "Flashing {} → {} using {}",
            hex.display(),
            bootloader_port,
            self.avrdude_path.display()
        ));

        // TODO Stage 5: stream avrdude output line-by-line
        on_progress(String::from("(Stage 5 implementation pending)"));

        Ok(())
    }
}

impl Default for FirmwareManager {
    fn default() -> Self {
        Self::new()
    }
}
