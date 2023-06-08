use cortex_m;
use thiserror_no_std::Error;

use nrf52810_hal::{rtc, gpio, gpiote, ppi, ppi::Ppi, pac, clocks};
use gpio::{Input, Output, PullDown, PushPull};
use nrf52810_hal::prelude::ConfigurablePpi;
use nrf52810_hal::pac::timer1::{bitmode::BITMODE_A, mode::MODE_A};

#[derive(Error, Debug)]
pub enum SetupError {
    PeripheralAccess,
    RtcCreation(#[from] rtc::Error),
}

pub struct Peripherals {
    pub rtc: pac::RTC1,
    pub clocks: clocks::Clocks<clocks::ExternalOscillator, clocks::ExternalOscillator, clocks::LfOscStarted>,
    pub probe_enable: gpio::Pin<Output<PushPull>>,
    pub probe_signal: gpio::Pin<Input<PullDown>>,
    pub gpiote: gpiote::Gpiote,
    pub timer: pac::TIMER1,
    pub adc: pac::SAADC,
    pub ppi: ppi::Parts
}

impl Peripherals {
    pub fn new(mut p: pac::Peripherals, core: &mut cortex_m::Peripherals) -> Result<Self, SetupError>
    {
        setup_interrupt_priority(core);

        let rtc = setup_rtc1(p.RTC1, core)?;
        let clocks = setup_clocks(p.CLOCK);
        let (probe_enable, probe_signal) = setup_gpio(p.P0);
        let gpiote = setup_gpiote(&probe_signal, p.GPIOTE);
        setup_counter(&mut p.TIMER1);
        //setup_adc(&mut p.SAADC);
        let ppi = ppi::Parts::new(p.PPI);

        let mut peripherals = Self {
            rtc, clocks, probe_enable, probe_signal, gpiote,
            timer: p.TIMER1,
            adc: p.SAADC,
            ppi
        };

        peripherals.setup_ppi();

        Ok(peripherals)
    }

    /// TODO
    pub fn setup_ppi(&mut self)
    {
        let ppi0 = &mut self.ppi.ppi0;
        ppi0.set_task_endpoint(&self.timer.tasks_count);
        ppi0.set_event_endpoint(self.gpiote.channel0().event());
        ppi0.enable();

        let ppi1 = &mut self.ppi.ppi1;
        ppi1.set_event_endpoint(&self.rtc.events_compare[3]);
        ppi1.set_task_endpoint(&self.timer.tasks_capture[3]);
        ppi1.enable();

        let ppi2 = &mut self.ppi.ppi2;
        ppi2.set_event_endpoint(&self.rtc.events_compare[2]);
        ppi2.set_task_endpoint(&self.timer.tasks_clear);
        ppi2.enable();
    }
}

/// Configure real-time counter 1 (RTC1)
///
/// Sets the following registers:
///  - Prescaler = 2^12 - 1: Sets the `tick` event to a frequency of 8Hz
///  - Compare1 = 2: Compare1 event happens 0.25s after Overflow
///  - Compare2 = 4: Compare2 event happens 0.25s after Compare1
///  - Compare3 = 12: Compare3 event happens 1s after Compare2
///
/// Enables events for Compare1, Compare2, and Compare3.
///
/// Enables interrupts for Compare3.
fn setup_rtc1(rtc1: pac::RTC1, core: &mut cortex_m::Peripherals)
                  -> Result<pac::RTC1, SetupError>
{
    let prescaler: u32 = 0xFFF;
    let mut rtc1 = rtc::Rtc::new(rtc1, prescaler)?;
    rtc1.set_compare(rtc::RtcCompareReg::Compare1, 2)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare1);
    rtc1.set_compare(rtc::RtcCompareReg::Compare2, 4)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare2);
    rtc1.set_compare(rtc::RtcCompareReg::Compare3, 4 + clocks::LFCLK_FREQ / (prescaler + 1))?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare3);
    rtc1.enable_interrupt(rtc::RtcInterrupt::Compare3, Some(&mut core.NVIC));
    rtc1.enable_counter();

    Ok(rtc1.release())
}

/// Configure clocks to enable both external oscillators (High-frequency and low-frequency),
/// and starting the low-frequency clock.
///
/// I think the Softdevice might do all of this anyway, but better safe than sorry!
fn setup_clocks(clocks: pac::CLOCK)
                    -> clocks::Clocks<clocks::ExternalOscillator, clocks::ExternalOscillator, clocks::LfOscStarted>
{
    // NoExternalNoBypass does NOT mean "No external oscillator": It means no external signal
    // provided to the external oscillator. Normal operation (with the circuit I am using,
    // ripped straight from Nordic's reference circuitry) needs no external signal.
    // See Table 16 on Page 87 of the NRF52810 datasheet.
    clocks::Clocks::new(clocks)
        .set_lfclk_src_external(clocks::LfOscConfiguration::NoExternalNoBypass)
        .enable_ext_hfosc()
        .start_lfclk()
}

/// Explicitly set the interrupt priority of the RTC1 interrupt as low as possible.
///
/// Higher-priority interrupts can interfere with the Softdevice:
///   https://github.com/embassy-rs/nrf-softdevice#interrupt-priority
///
/// But my interrupt priority seems to default to 7 (lowest priority) anyway?
/// IDK, just setting it explicitly in case that default behavior ever changes
fn setup_interrupt_priority(core: &mut cortex_m::Peripherals)
{
    unsafe {
        // Interrupt priorities are stored in the top 3 bits:
        //  https://community.arm.com/arm-community-blogs/b/embedded-blog/posts/cutting-through-the-confusion-with-arm-cortex-m-interrupt-priorities
        core.NVIC.set_priority(pac::Interrupt::RTC1, 3 << 5);
    }
}

/// Configure 2 pins:
///  - P0_30: Push-pull output to enable / disable moisture probe oscillator
///  - P0_31: PullDown input to read signal from moisture probe oscillator
fn setup_gpio(p0: pac::P0) -> (gpio::Pin<Output<PushPull>>, gpio::Pin<Input<PullDown>>)
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

    (probe_enable, probe_input)
}

/// Configure GPIOTE (GPIO Tasks and Events) to output an event for a rising edge from
/// the moisture probe timer pin, P0_31. This task will be connected to the counter/timer TIMER1
/// with PPI.
fn setup_gpiote(pin: &gpio::Pin<Input<PullDown>>, gpiote: pac::GPIOTE) -> gpiote::Gpiote
{
    // Setup Gpiote to output an event for probe pulse input rising edge
    let gpiote = gpiote::Gpiote::new(gpiote);
    gpiote.channel0()
        .input_pin(pin)
        .lo_to_hi();

    gpiote
}

/// Configure TIMER1 in Counter mode, to count pulses from the moisture probe oscillator
fn setup_counter(timer1: &mut pac::TIMER1)
{
    // Setup counter to count pulses from probe timer
    // Use TIMER1 because Softdevice uses TIMER0
    // The higher-level Timer HAL is incomplete, so we must get nasty with the PAC
    timer1.bitmode.write(|w| w.bitmode().variant(BITMODE_A::_32BIT));
    timer1.mode.write(|w| w.mode().variant(MODE_A::LOW_POWER_COUNTER));
    timer1.tasks_start.write(|w| w.tasks_start().set_bit());
}

/// TODO
fn setup_adc(adc: &mut pac::SAADC)
{
    unimplemented!()
}
