# Implementation Plan: Nexperia Motor Driver GUI (NEVC-MTR1)

## Overview
A Windows-only Rust GUI application for controlling the Nexperia NEVC-MTR1 motor driver board
via SCPI over USB-serial (Arduino Leonardo). Built with `iced` for the GUI, `serialport` for
serial communication, and `avrdude` for firmware flashing.

---

## Stage 1 — Project Foundation
**Goal:** Compilable skeleton with full module structure and working tab navigation.

- [x] `Cargo.toml` with all required dependencies (`iced`, `serialport`, `tokio`, `anyhow`, `serde`)
- [x] Module scaffold: `serial`, `scpi`, `firmware`, `ui` (with sub-panels)
- [x] Basic `iced::Application` with tab-based navigation
- [x] Serial port detection using `serialport::available_ports()`
  - Filter for Arduino Leonardo USB VID/PID (0x2341 / 0x8036 or 0x0036)
- [x] Connection panel UI: port picker, refresh, connect/disconnect
- [x] Placeholder panels for Firmware, Motor Control, Graphs, Configuration
- [x] Status bar showing current state

**Outcome:** App launches, detects COM ports, navigates between panels.

---

## Stage 2 — SCPI Communication Layer
**Goal:** Establish a live serial connection and send/receive SCPI commands.

- [x] Open serial port with correct settings (115200 baud, 8N1, LF, DTR+RTS)
- [x] Async send/receive via `tokio::task::spawn_blocking` + `Arc<Mutex<SerialPort>>`
- [x] `*IDN?` query → parse `<Manufacturer>,<Model>,<Serial>,<FirmwareVersion>`
  - Display firmware version in Connection and Firmware panels
- [x] `SYSTem:ERRor?` / `SYSTem:ERRor:COUNt?` — error queue polled after connect
- [x] Error display in status bar
- [x] Connection state machine: Disconnected → Connecting → Connected → Disconnected

**Outcome:** App connects to board, reads firmware version, reports SCPI errors.

---

## Stage 3 — Motor Control Panel
**Goal:** Full motor control and measurement queries.

- [x] Enable/disable motor (`CONFigure:ENABle ON/OFF`) with toggle
- [x] Set frequency (`CONFigure:FREQuency <Hz>`) with input + slider (7183–100000 Hz)
  - Input validation with error message
- [x] Set direction (`CONFigure:DIREction FORWard/REVErse`) with buttons/dropdown
- [x] Query all measurements on demand and on a polling timer:
  - `MEASure:SPEEd?` → RPM
  - `MEASure:CURRent:IBUS?` / `IPHU?` / `IPHV?` / `IPHW?` → Amperes
  - `MEASure:DIREction?` → FORWard / REVErse / UNKNown
  - `MEASure:DUTYcycle?` → %
  - `MEASure:VOLTage?` → V
- [x] Query confirmation (`CONFigure:ENABle?`, `CONFigure:FREQuency?`, `CONFigure:DIREction?`)

**Outcome:** Full motor control and live measurement reading.

---

## Stage 4 — Live Graphing Panel
**Goal:** Configurable live plots of selected measurements.

- [x] Variable selection (checkboxes): Speed, IBUS, IPHU, IPHV, IPHW, Duty Cycle, Voltage
- [x] Configurable polling frequency (1–50 Hz slider)
- [x] Background tokio polling task with ring-buffer history (last N seconds)
- [x] iced `canvas`-based live scrolling plots
  - Auto-scaling Y axis
  - Time-scrolling X axis
  - Legend / axis labels
- [x] Start / stop polling button

**Outcome:** Live multi-channel measurement graphs.

---

## Stages 5 + 6 — Firmware & Configuration (Combined Tab)
**Goal:** Edit all 26 firmware parameters and compile+upload directly from the GUI.

Firmware source: `https://github.com/Nexperia/NEVC-MTR1-t01` branch `dev/scpi-gui-compatible`
Config file: `main/config.h` — 26 user-settable `#define` constants.
IDN serial: `*IDN?` 3rd field — 26 hyphen-separated hex values mapping 1:1 to config parameters.

### 5a — Firmware Config Data Model
- [x] `FirmwareConfig` struct — 26 typed fields with repo default values hard-coded
- [x] `FirmwareConfig::from_idn_serial(serial)` — parse hex IDN serial → struct
- [x] `patch_config_h(source, config)` — modify `#define PARAM value` in config.h text

### 5b — Toolchain Auto-Bootstrap
- [x] Auto-download `arduino-cli.exe` from GitHub releases if not present
  - Stored in `%APPDATA%\nevc_mtr1_gui\tools\`
  - Uses GitHub releases API to find latest Windows 64-bit zip
- [x] Auto-install `arduino:avr` core via `arduino-cli core install arduino:avr`
- [x] Auto-download firmware ZIP from GitHub (`dev/scpi-gui-compatible` branch)
  - Stored in `%APPDATA%\nevc_mtr1_gui\firmware\`
  - `main/` directory extracted; presence of `main/main.ino` checked before re-download

### 5c — Compile & Upload Flow
- [x] Patch `main/config.h` with current GUI parameter values
- [x] `arduino-cli compile --fqbn arduino:avr:leonardo main/` — compile sketch
- [x] 1200-baud bootloader reset trick on the connected (or selected) Leonardo port
  - Open port at 1200 baud with DTR asserted → close → wait for bootloader port (PID 0x0036)
- [x] `arduino-cli upload --fqbn arduino:avr:leonardo -p <port> main/` — upload
- [x] Multi-step progress log shown in GUI during each stage

### 6 — Config Editing UI (Combined with Firmware Tab)
- [x] Source toggle: **Repo defaults** (hard-coded from config.h) vs **From device** (parsed IDN serial)
- [x] Grouped, labelled parameter inputs for all 26 parameters:
  - Motor config, Phase current, Bus current, Speed control, PID, Voltage sense, System
- [x] Input validation (parse on compile, highlight invalid fields)
- [x] "Load Defaults" button to reset inputs to selected source
- [x] "Compile & Upload" button — triggers full tool-bootstrap + compile + upload pipeline
- [x] Step-by-step progress shown in flash log panel

**Outcome:** Full in-GUI firmware configuration editor with one-click compile + flash.

---

## Future Enhancements (post-Stage 6)
- Cross-platform support (Linux/macOS)
- Auto-download latest firmware from GitHub
- Native AVR109 flashing (replace avrdude)
- Advanced plotting (multi-channel, zoom, export CSV)
- Motor tuning interface
- Full SCPI command coverage (any future commands added to firmware)

---

## Architecture Summary

```
nevc_mtr1_gui/
├── Cargo.toml
└── src/
    ├── main.rs           — entry point, platform check
    ├── app.rs            — iced Application, root state, Messages, update, view
    ├── serial/
    │   └── mod.rs        — port detection, SerialConnection, ConnectionState
    ├── scpi/
    │   └── mod.rs        — SCPI command constants, IDN parsing, validation
    ├── firmware/
    │   └── mod.rs        — avrdude integration, bootloader reset, flash
    └── ui/
        ├── mod.rs        — Panel enum, re-exports
        ├── connection.rs — Connection panel view
        ├── motor.rs      — Motor Control panel view
        ├── firmware.rs   — Firmware panel view
        ├── graphs.rs     — Graphs panel view
        └── config.rs     — Configuration panel view
```

### Key Crates
| Crate | Purpose |
|-------|---------|
| `iced` | GUI framework (declarative, reactive) |
| `serialport` | Serial port enumeration and I/O |
| `tokio` | Async runtime for background tasks |
| `anyhow` | Ergonomic error handling |
| `serde` + `serde_json` | Config serialization |
