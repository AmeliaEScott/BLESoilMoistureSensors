use cortex_m::Peripherals;
use thiserror_no_std::Error;
use crate::config;

use nrf52810_hal as hal;
use nrf52810_hal::{rtc, uarte};

use defmt::{debug};

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
    // I am explicitly doing this because of this documentation:
    //   https://github.com/embassy-rs/nrf-softdevice#interrupt-priority
    // But my interrupt priority seems to default to 7 (lowest priority) anyway?
    // IDK, just setting it explicitly in case that default behavior ever changes
    unsafe {
        // Interrupt priorities are stored in the top 3 bits:
        //  https://community.arm.com/arm-community-blogs/b/embedded-blog/posts/cutting-through-the-confusion-with-arm-cortex-m-interrupt-priorities
        core.NVIC.set_priority(hal::pac::Interrupt::RTC1, 3 << 5);
    }
    let p = hal::pac::Peripherals::take()
        .ok_or(ClockSetupError::PeripheralAccess)?;

    // NoExternalNoBypass does NOT mean "No external oscillator": It means no external signal
    // provided to the external oscillator. Normal operation (with the circuit I am using,
    // ripped straight from Nordic's reference circuitry) needs no external signal.
    // See Table 16 on Page 87 of the NRF52810 datasheet.
    hal::clocks::Clocks::new(p.CLOCK)
        .set_lfclk_src_external(hal::clocks::LfOscConfiguration::NoExternalNoBypass)
        .start_lfclk();

    let mut rtc = rtc::Rtc::new(p.RTC1, config::TIMER_PRESCALER)?;
    rtc.set_compare(rtc::RtcCompareReg::Compare1, config::TIMER_COMPARE)?;
    rtc.enable_event(rtc::RtcInterrupt::Compare1);
    rtc.enable_interrupt(rtc::RtcInterrupt::Compare1, Some(&mut core.NVIC));
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