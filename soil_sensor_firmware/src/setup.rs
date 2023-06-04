use cortex_m::Peripherals;
use thiserror_no_std::Error;
use crate::config;

use nrf52810_hal as hal;
use nrf52810_hal::{rtc, uarte};

#[derive(Error, Debug)]
pub enum ClockSetupError {
    PeripheralAccess,
    RtcCreation(#[from] rtc::Error),
}

#[derive(Error, Debug)]
pub enum UartSetupError {
    PeripheralAccess,
    CorePeripheralAccess,
}

#[derive(Error, Debug)]
pub enum SetupError {
    Clock(#[from] ClockSetupError),
}

///
/// First-time configuration of the Real-Time Counter RTC1
///
/// Does all of the following:
///  - Start Low-Frequency Clock
///  - Set Prescaler to `config::TIMER_PRESCALER`
///  - Set Compare0 register to `config::TIMER_COMPARE`
///  - Enable Compare0 event
///  - Start the counter
///
pub fn setup_timer(core: &mut Peripherals) -> Result<rtc::Rtc<hal::pac::RTC1>, ClockSetupError>
{
    let p = hal::pac::Peripherals::take()
        .ok_or(ClockSetupError::PeripheralAccess)?;
    // TODO: Why is this not working with LfOscConfiguration::ExternalNoBypass?
    let clocks = hal::clocks::Clocks::new(p.CLOCK)
        .set_lfclk_src_external(hal::clocks::LfOscConfiguration::NoExternalNoBypass)
        .start_lfclk();

    let mut rtc = rtc::Rtc::new(p.RTC1, config::TIMER_PRESCALER)?;
    rtc.set_compare(rtc::RtcCompareReg::Compare0, config::TIMER_COMPARE)?;
    rtc.enable_event(rtc::RtcInterrupt::Compare0);
    rtc.enable_interrupt(rtc::RtcInterrupt::Compare0, Some(&mut core.NVIC));
    rtc.enable_counter();
    Ok(rtc)
}

// pub fn setup_uart(core: &mut Peripherals) -> Result<(), UartSetupError>
// {
//     let p = hal::pac::Peripherals::take()
//         .ok_or(UartSetupError::PeripheralAccess)?;
//
//     Ok(())
// }