use cortex_m;
use thiserror_no_std::Error;

use nrf52810_hal::{rtc, gpio, gpiote, ppi, ppi::Ppi, pac, clocks};
use gpio::{Input, Output, PullDown, PushPull};
use nrf52810_hal::pac::rtc0::tasks_trigovrflw::TASKS_TRIGOVRFLW_AW;
use nrf52810_hal::pac::saadc::{resolution, oversample, ch::{pselp, config as adc_config}};
use nrf52810_hal::prelude::{ConfigurablePpi, OutputPin};
use nrf52810_hal::pac::timer1::{bitmode as timer_bitmode, mode as timer_mode};

use soil_sensor_common::Measurement;
use void::ResultVoidExt;

/// Statically parse the string from environment variable "SENSOR_ID" into a u16.
/// Compilation will fail if SENSOR_ID is not a valid 4-digit hexadecimal number
const fn get_id() -> u16 {
    let string: &'static str = env!("SENSOR_ID");
    let mut res: u16 = 0;
    // assert!(string.is_ascii());
    let mut bytes = string.as_bytes();
    assert!(bytes.len() == 4);
    while let [byte, rest @ ..] = bytes {
        assert!(res <= 0x0FFF);
        bytes = rest;
        let digit = match byte {
            b'0'..=b'9' => *byte - b'0',
            b'a'..=b'f' => *byte - b'a' + 10,
            b'F'..=b'F' => *byte - b'A' + 10,
            _ => panic!("Environment variable SENSOR_ID is not a valid hex number")
        };
        res = (res * 16) + digit as u16;
    }
    res
}

pub const SENSOR_ID: u16 = get_id();

#[derive(Error, Debug)]
pub enum SetupError {
    PeripheralAccess,
    RtcCreation(#[from] rtc::Error),
}

pub struct Peripherals {
    rtc: pac::RTC1,
    probe_enable: gpio::Pin<Output<PushPull>>,
    #[allow(dead_code)]
    probe_signal: gpio::Pin<Input<PullDown>>,
    gpiote: gpiote::Gpiote,
    counter: pac::TIMER1,
    adc: pac::SAADC,
    adc_buffer : &'static mut [i16],
    ppi: ppi::Parts,
    temp: pac::TEMP,
    temp_buffer: i32,
    sequence: u16,
}

impl Peripherals {
    pub fn new(mut p: pac::Peripherals, core: &mut cortex_m::Peripherals, dma_buffer: &'static mut [i16]) -> Result<Self, SetupError>
    {
        setup_interrupt_priority(core);
        p.POWER.dcdcen.write(|w| w.dcdcen().set_bit());

        let rtc = setup_rtc1(p.RTC1, core)?;
        setup_clocks(p.CLOCK);
        let (probe_enable, probe_signal) = setup_gpio(p.P0);
        let gpiote = setup_gpiote(&probe_signal, p.GPIOTE);
        setup_counter(&mut p.TIMER1);
        setup_adc(&mut p.SAADC, dma_buffer);
        let ppi = ppi::Parts::new(p.PPI);

        let mut peripherals = Self {
            rtc, probe_enable, probe_signal, gpiote,
            counter: p.TIMER1,
            adc: p.SAADC,
            adc_buffer: dma_buffer,
            ppi,
            temp: p.TEMP,
            temp_buffer: i32::MIN,
            sequence: u16::MAX,
        };

        peripherals.setup_ppi();

        Ok(peripherals)
    }

    /// Configure PPI to automatically do the following:
    ///  - Use counter TIMER1 to count pulses from moisture probe oscillator output
    ///  - On RTC1 Overflow, start ADC and take a sample
    ///  - Between RTC1.Compare0 and RTC1.Compare1 (Normally exactly 1s), count the pulses from the probe
    fn setup_ppi(&mut self)
    {
        // TODO: Delete this!
        //  Use Compare3 to trigger early Overflow
        {
            // PPI channel 16 is the last I can use
            let ppi = &mut self.ppi.ppi16;
            ppi.set_event_endpoint(&self.rtc.events_compare[3]);
            ppi.set_task_endpoint(&self.rtc.tasks_trigovrflw);
            ppi.set_fork_task_endpoint(self.gpiote.channel1().task_clr());
            ppi.enable();
        }

        // Connect probe input GPIO to counter
        {
            let ppi = &mut self.ppi.ppi0;
            ppi.set_event_endpoint(self.gpiote.channel0().event());
            ppi.set_task_endpoint(&self.counter.tasks_count);
            ppi.enable();
        }

        // On clock overflow, startup the ADC, AND take a temperature measurement
        {
            let ppi = &mut self.ppi.ppi1;
            ppi.set_event_endpoint(&self.rtc.events_ovrflw);
            ppi.set_task_endpoint(&self.adc.tasks_start);
            ppi.set_fork_task_endpoint(&self.temp.tasks_start);
            ppi.enable();
        }

        // As soon as ADC is started, take a sample
        {
            let ppi = &mut self.ppi.ppi2;
            ppi.set_event_endpoint(&self.adc.events_started);
            ppi.set_task_endpoint(&self.adc.tasks_sample);
            ppi.enable();
        }

        // On RTC Compare0, clear the counter
        {
            let ppi = &mut self.ppi.ppi4;
            ppi.set_event_endpoint(&self.rtc.events_compare[0]);
            ppi.set_task_endpoint(&self.counter.tasks_clear);
            ppi.enable();
        }

        // On RTC Compare1, capture the counter (This also triggers an interrupt)
        // Also turn off the probe timer
        {
            let ppi = &mut self.ppi.ppi5;
            ppi.set_event_endpoint(&self.rtc.events_compare[1]);
            ppi.set_task_endpoint(&self.counter.tasks_capture[0]);
            // TODO: Configure GPIOTE for this
            // ppi.set_fork_task_endpoint(self.gpiote.channel1().task_clr());
            ppi.enable();
        }
    }

    pub fn get_adc_measurement(&self) -> i16 {
        self.adc_buffer[0]
    }

    pub fn reset_adc_event(&self) {
        self.adc.events_end.reset();
    }

    pub fn reset_rtc_event(&self) {
        self.rtc.events_compare[1].reset()
    }

    // TODO: Documentation
    pub fn read_temp(&mut self) -> Result<(), ()> {
        if self.temp.events_datardy.read().events_datardy().bit() {
            self.temp.events_datardy.reset();
            self.temp_buffer = self.temp.temp.read().temp().bits() as i32;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get_measurement(&mut self) -> Measurement {
        (self.sequence, _) = self.sequence.overflowing_add(1);
        Measurement {
            id: SENSOR_ID,
            capacitor_voltage: self.adc_buffer[0],
            moisture_frequency: self.counter.cc[0].read().cc().bits(),
            temperature: self.temp_buffer,
            sequence: self.sequence
        }
    }

    pub fn trigger_rtc_overflow(&self) {
        self.rtc.tasks_trigovrflw.write(|w| w.tasks_trigovrflw().variant(TASKS_TRIGOVRFLW_AW::TRIGGER));
    }

    pub fn enable_probe(&mut self) {
        self.probe_enable.set_high().void_unwrap()
    }

    pub fn disable_probe(&mut self) {
        self.probe_enable.set_low().void_unwrap()
    }
}

/// Configure real-time counter 1 (RTC1)
///
/// Sets the following registers:
///  - Prescaler = 2^12 - 1: Sets the `tick` event to a frequency of 8Hz
///  - Enable Overflow event
///    - On Overflow, begin taking ADC measurement
///    - Enable probe measurement pin (depending on ADC measurement) immediately after ADC
///      measurement is available
///  - Compare0 = 2: Compare0 event happens 0.25s after Overflow
///    - On Compare0, reset counter (Gives time for oscillator to settle)
///  - Compare1 = 10: Compare1 event happens 1s after Compare0
///    - On Compare1, after 1s measurement time, trigger capture, and interrupt
///  - Compare3 = Whatever
///    - On Compare3, trigger RTC overflow (Useful for debugging)
///
/// Enables events for Compare0, Compare1, and maybe Compare3.
///
/// Enables interrupts for Compare3.
fn setup_rtc1(rtc1: pac::RTC1, core: &mut cortex_m::Peripherals) -> Result<pac::RTC1, SetupError>
{
    // const PRESCALER: u32 = 0xFFF;
    const PRESCALER: u32 = 0x007;
    const ONE_SECOND: u32 = clocks::LFCLK_FREQ / (PRESCALER + 1);
    const QUARTER_SECOND: u32 = ONE_SECOND / 4;

    let mut rtc1 = rtc::Rtc::new(rtc1, PRESCALER)?;
    rtc1.set_compare(rtc::RtcCompareReg::Compare0, QUARTER_SECOND)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare0);
    rtc1.set_compare(rtc::RtcCompareReg::Compare1, ONE_SECOND + QUARTER_SECOND)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare1);

    rtc1.set_compare(rtc::RtcCompareReg::Compare3, ONE_SECOND * 30)?;
    rtc1.enable_event(rtc::RtcInterrupt::Compare3);
    rtc1.enable_interrupt(rtc::RtcInterrupt::Compare1, Some(&mut core.NVIC));
    rtc1.enable_event(rtc::RtcInterrupt::Overflow);
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
fn setup_gpiote(input_pin: &gpio::Pin<Input<PullDown>>, gpiote: pac::GPIOTE) -> gpiote::Gpiote
{
    // Setup Gpiote to output an event for probe pulse input rising edge
    let gpiote = gpiote::Gpiote::new(gpiote);
    gpiote.channel0()
        .input_pin(input_pin)
        .lo_to_hi();

    gpiote
}

/// Configure TIMER1 in Counter mode, to count pulses from the moisture probe oscillator
fn setup_counter(timer1: &mut pac::TIMER1)
{
    // Setup counter to count pulses from probe timer
    // Use TIMER1 because Softdevice uses TIMER0
    // The higher-level Timer HAL is incomplete, so we must get nasty with the PAC
    timer1.bitmode.write(|w| w.bitmode().variant(timer_bitmode::BITMODE_A::_32BIT));
    timer1.mode.write(|w| w.mode().variant(timer_mode::MODE_A::LOW_POWER_COUNTER));
    timer1.tasks_start.write(|w| w.tasks_start().set_bit());
}

/// Sets up a single ADC channel for measuring the voltage on the main capacitor.
fn setup_adc(adc: &mut pac::SAADC, dma_buffer: &mut [i16])
{
    adc.resolution.write(|w| w.val().variant(resolution::VAL_A::_14BIT));
    adc.oversample.write(|w| w.oversample().variant(oversample::OVERSAMPLE_A::OVER256X));

    // VCap is P0_02 / AIN0
    adc.ch[0].pselp.write(|w|  w.pselp().variant(pselp::PSELP_A::ANALOG_INPUT0));
    adc.ch[0].config.write(|w| {
        w
           .mode().variant(adc_config::MODE_A::SE)
           .gain().variant(adc_config::GAIN_A::GAIN1_4)
           .refsel().variant(adc_config::REFSEL_A::VDD1_4)
           .tacq().variant(adc_config::TACQ_A::_40US)
           .burst().set_bit()
    });

    adc.result.ptr.write(|w| w.ptr().variant(dma_buffer.as_ptr() as u32));
    adc.result.maxcnt.write(|w| w.maxcnt().variant(dma_buffer.len() as u16));

    adc.inten.write(|w| {
        w.end().set_bit()
    });
    adc.enable.write(|w| w.enable().set_bit() );
}
