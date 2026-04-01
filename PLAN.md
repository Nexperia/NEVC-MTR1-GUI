# Implementation Plan: Nexperia Motor Driver GUI (NEVC-MTR1)

## Overview
A Windows-only Rust GUI application for controlling the Nexperia NEVC-MTR1 motor driver board
via SCPI over USB-serial (Arduino Leonardo). Built with `iced` for the GUI, `serialport` for
serial communication, and `avrdude` for firmware flashing.

---

## Stage 1 — Project Foundation ✅ (current)
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

- [ ] Open serial port with correct settings (115200 baud, 8N1, LF, DTR+RTS)
- [ ] Async send/receive loop on a background tokio task
- [ ] `*IDN?` query → parse `<Manufacturer>,<Model>,<Serial>,<FirmwareVersion>`
  - Display firmware version in Connection and Firmware panels
- [ ] `SYSTem:ERRor?` / `SYSTem:ERRor:COUNt?` — error queue polling
- [ ] Error display in status bar / pop-up
- [ ] Connection state machine: Disconnected → Connecting → Connected → Disconnected

**Outcome:** App connects to board, reads firmware version, reports SCPI errors.

---

## Stage 3 — Motor Control Panel
**Goal:** Full motor control and measurement queries.

- [ ] Enable/disable motor (`CONFigure:ENABle ON/OFF`) with toggle
- [ ] Set frequency (`CONFigure:FREQuency <Hz>`) with input + slider (7183–100000 Hz)
  - Input validation with error message
- [ ] Set direction (`CONFigure:DIREction FORWard/REVErse`) with buttons/dropdown
- [ ] Query all measurements on demand and on a polling timer:
  - `MEASure:SPEEd?` → RPM
  - `MEASure:CURRent:IBUS?` / `IPHU?` / `IPHV?` / `IPHW?` → Amperes
  - `MEASure:DIREction?` → FORWard / REVErse / UNKNown
  - `MEASure:DUTYcycle?` → %
  - `MEASure:VOLTage?` → V
- [ ] Query confirmation (`CONFigure:ENABle?`, `CONFigure:FREQuency?`, `CONFigure:DIREction?`)

**Outcome:** Full motor control and live measurement reading.

---

## Stage 4 — Live Graphing Panel
**Goal:** Configurable live plots of selected measurements.

- [ ] Variable selection (checkboxes): Speed, IBUS, IPHU, IPHV, IPHW, Duty Cycle, Voltage
- [ ] Configurable polling frequency (1–50 Hz slider)
- [ ] Background tokio polling task with ring-buffer history (last N seconds)
- [ ] iced `canvas`-based live scrolling plots
  - Auto-scaling Y axis
  - Time-scrolling X axis
  - Legend / axis labels
- [ ] Start / stop polling button

**Outcome:** Live multi-channel measurement graphs.

---

## Stage 5 — Firmware Management Panel
**Goal:** Flash firmware to the Arduino Leonardo without leaving the GUI.

- [ ] Detect Arduino Leonardo specifically (USB VID=0x2341, PID=0x8036)
- [ ] Implement 1200-baud reset trick to enter bootloader (PID changes to 0x0036)
  - Close original port, wait for bootloader port to appear, open with 1200 baud, close
- [ ] Bundle `avrdude.exe` + required `.conf` in application resources
- [ ] Bundle compiled firmware `.hex` for `dev/scpi-gui-compatible` branch
- [ ] Call `avrdude` via `std::process::Command` with progress output streamed to GUI
- [ ] Display flash success / detailed error message
- [ ] Parse current firmware version from `*IDN?` response for upgrade comparison
- [ ] Fallback instructions if avrdude not found

**Outcome:** One-click firmware flashing.

---

## Stage 6 — Configuration Panel
**Goal:** Display and edit firmware constants parsed from `*IDN?` serial field.

- [ ] Parse the 26-field hyphen-separated hex serial in `*IDN?` → firmware constants
  - Map each field to its `scpi_config.h` constant (requires firmware documentation)
- [ ] Display constants in a table with descriptions
- [ ] Mark read-only vs user-configurable fields
- [ ] Allow editing user-configurable values with validation
- [ ] Serialize modified values back to `scpi_config.h` format
- [ ] Option to trigger recompile + flash (requires Arduino CLI installed)
- [ ] Warn before overwriting read-only fields

**Outcome:** In-app firmware parameter configuration.

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
