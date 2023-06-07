use nrf52810_hal as hal;

/// Timer configuration:
/// Base clock is low-frequency clock at 32,768Hz.
/// Prescaler is a 12-bit integer. Setting this greater than 2^12 - 1 will result in
/// a panic on startup.
pub const TIMER_PRESCALER: u32 = 0x000;
/// Compare is a 24-bit integer. Setting this greater than 2^24 - 1 will result in
/// a panic on startup.
pub const TIMER_COMPARE: u32 = 0x00_FF_FF;

// Vcap: P0.02 / AIN0
// NRST: P0.21
// Probe timer enable: P0.30
// Probe timer input: P0.31