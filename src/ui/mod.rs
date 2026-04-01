pub mod config;
pub mod connection;
pub mod firmware;
pub mod graphs;
pub mod motor;

/// Top-level navigation panels.
#[derive(Debug, Clone, PartialEq)]
pub enum Panel {
    Connection,
    Firmware,
    MotorControl,
    Graphs,
    Configuration,
}
