use cortex_m;
use thiserror_no_std::Error;
use crate::config;

use nrf52810_hal as hal;
use nrf52810_hal::{rtc, gpio, gpiote, ppi, ppi::Ppi, pac, timer};
use nrf52810_hal::prelude::ConfigurablePpi;
use nrf52810_hal::pac::timer1::{bitmode::BITMODE_A, mode::MODE_A};

use defmt::{debug};

#[derive(Error, Debug)]
pub enum ClockSetupError {
    PeripheralAccess,
    RtcCreation(#[from] rtc::Error),
}

#[derive(Error, Debug)]
pub enum GpioSetupError {
    PeripheralAccess,
    CorePeripheralAccess,
}

#[derive(Error, Debug)]
pub enum SetupError {
    Clock(#[from] ClockSetupError),
}

// TODO:
//  Restructure this setup module, such that:
//   - Each function takes only one owned peripheral, returns owned wrapped peripheral
//     - GPIO, GPIOTE, PPI, RTC1, TIMER1
//   - One main setup() function which does peripherals.take() and delegates all the setup
//     - Return a struct with all relevant, necessary peripherals


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
pub fn setup_timer(ppi_channels: &mut ppi::Parts, rtc1: pac::RTC1, timer1: &pac::TIMER1, clocks: pac::CLOCK,
                   core: &mut cortex_m::Peripherals)
    -> Result<rtc::Rtc<hal::pac::RTC1>, ClockSetupError>
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

    // TODO: Delete this, or else move it to the right place
    let mut ppi1 = &mut ppi_channels.ppi1;
    ppi1.set_task_endpoint(&timer1.tasks_capture[3]);
    ppi1.set_event_endpoint(&rtc1.events_compare[1]);
    ppi1.enable();

    // NoExternalNoBypass does NOT mean "No external oscillator": It means no external signal
    // provided to the external oscillator. Normal operation (with the circuit I am using,
    // ripped straight from Nordic's reference circuitry) needs no external signal.
    // See Table 16 on Page 87 of the NRF52810 datasheet.
    hal::clocks::Clocks::new(clocks)
        .set_lfclk_src_external(hal::clocks::LfOscConfiguration::NoExternalNoBypass)
        .start_lfclk();

    // Softdevice uses RTC0, so we must use RTC1
    let mut rtc1 = rtc::Rtc::new(rtc1, config::TIMER_PRESCALER)?;
    rtc1.set_compare(rtc::RtcCompareReg::Compare1, config::TIMER_COMPARE)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare1);
    rtc1.enable_interrupt(rtc::RtcInterrupt::Compare1, Some(&mut core.NVIC));
    rtc1.enable_counter();



    Ok(rtc1)
}

pub fn setup_probe_timer(p0: pac::P0, gpiote: pac::GPIOTE, timer1: &mut pac::TIMER1, ppi_channels: &mut ppi::Parts, core: &mut cortex_m::Peripherals) -> Result<(), GpioSetupError>
{
    // Configure probe enable output pin
    let p0 = gpio::p0::Parts::new(p0);
    let probe_enable = p0.p0_30
        .into_push_pull_output(gpio::Level::Low)
        .degrade();

    // Configure probe pulse input pin
    let probe_input = p0.p0_31
        .into_pulldown_input() // TODO: Test as floating?
        .degrade();

    // Setup Gpiote to output an event for probe pulse input rising edge
    let gpiote = gpiote::Gpiote::new(gpiote);
    gpiote.channel0()
        .input_pin(&probe_input)
        .lo_to_hi();

    // Setup counter to count pulses from probe timer
    // Use TIMER1 because Softdevice uses TIMER0
    // The higher-level Timer HAL is incomplete, so we must get nasty with the PAC
    timer1.bitmode.write(|w| w.bitmode().variant(BITMODE_A::_32BIT));
    timer1.mode.write(|w| w.mode().variant(MODE_A::LOW_POWER_COUNTER));
    timer1.tasks_start.write(|w| w.tasks_start().set_bit());

    // Softdevice uses PPI channels 17-31:
    //  https://github.com/embassy-rs/nrf-softdevice/blob/3b3eabb5383ae16a7772924f5301e6a79d0a591f/softdevice/s112/headers/nrf_soc.h#L102-L118
    // Configure PPI Channel 0:
    //  GPIO probe timer pin -> TIMER1 COUNT
    let mut ppi0 = &mut ppi_channels.ppi0;
    ppi0.set_task_endpoint(&timer1.tasks_count);
    ppi0.set_event_endpoint(gpiote.channel0().event());
    ppi0.enable();

    Ok(())
}