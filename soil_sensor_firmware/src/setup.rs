use thiserror_no_std::Error;
use crate::config;

use nrf52810_hal as hal;
use nrf52810_hal::rtc;

#[derive(Error, Debug)]
pub enum SetupError {
    PeripheralAccess,
    CorePeripheralAccess,
    RtcCreation(#[from] rtc::Error),
}

pub fn setup_timer() -> Result<rtc::Rtc<hal::pac::RTC0>, SetupError>
{
    let mut cp = hal::pac::CorePeripherals::take()
        .ok_or(SetupError::CorePeripheralAccess)?;
    let p = hal::pac::Peripherals::take()
        .ok_or(SetupError::PeripheralAccess)?;
    let clocks = hal::clocks::Clocks::new(p.CLOCK);
    clocks.start_lfclk();

    let mut rtc = rtc::Rtc::new(p.RTC0, 0)?;
    rtc.set_compare(rtc::RtcCompareReg::Compare0, config::TIMER_PERIOD)?;
    rtc.enable_event(rtc::RtcInterrupt::Compare0);
    rtc.enable_interrupt(rtc::RtcInterrupt::Compare0, Some(&mut cp.NVIC));
    Ok(rtc)
}