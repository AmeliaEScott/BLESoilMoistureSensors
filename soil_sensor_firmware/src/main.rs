#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

mod setup;
mod bluetooth;

use rtic::app;
use cortex_m::asm;

use defmt_rtt as _;
use panic_probe as _;
use defmt;

use nrf52810_hal as hal;
use hal::pac;
use hal::prelude::ConfigurablePpi;

use nrf_softdevice::Softdevice;
use defmt::{debug, info, warn, error, unwrap, Format};

#[derive(Debug, Format)]
pub enum Event {
    RTC,
    ADC
}

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use core::fmt::Error;
    use nrf52810_hal::prelude::OutputPin;
    use super::*;

    type SDRef = &'static mut Softdevice;

    #[shared]
    struct Shared {
        peripherals: setup::Peripherals,
    }

    #[local]
    struct Local {}

    #[init(local = [dma_buffer : [i16; 1] = [0; 1]])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        debug!("Init! Look, I'm initializing!!! Isn't that so cool???");
        let mut p = pac::Peripherals::take().unwrap();
        let dma_buffer : &'static mut [i16] = cx.local.dma_buffer.as_mut_slice();
        let mut peripherals = setup::Peripherals::new(
            p, &mut cx.core, dma_buffer).unwrap();
        peripherals.probe_enable.set_high();

        ble_service::spawn().unwrap();

        (
            Shared {
                peripherals
            },
            Local {}
        )
    }

    #[idle]
    fn idle(cx: idle::Context) -> ! {
        defmt::debug!("Now I am idling");
        loop {
            asm::nop();
        }
    }

    #[task(binds = RTC1, shared = [peripherals])]
    fn timer_callback(mut cx: timer_callback::Context)
    {
        debug!("Timer interrupt!");
        cx.shared.peripherals.lock(|p : &mut setup::Peripherals|{
            p.rtc.events_compare[3].reset();
            p.rtc.tasks_clear.write(|w| w.tasks_clear().set_bit());

            let probe : u32 = p.timer.cc[3].read().cc().bits();
            p.timer.tasks_clear.write(|w| w.tasks_clear().set_bit());
            info!("Probe timer count is {}Hz", probe);

            p.adc.tasks_start.write(|w| w.tasks_start().set_bit());
            p.adc.tasks_sample.write(|w| w.tasks_sample().set_bit());
        });
    }

    #[task(binds = SAADC, shared = [peripherals])]
    fn adc_callback(mut cx: adc_callback::Context)
    {
        debug!("ADC interrupt!");
        cx.shared.peripherals.lock(|p : &mut setup::Peripherals|{
            p.adc.events_end.reset();
            let adc_measurement = p.adc_buffer[0];
            let adc_measurement_mv = (adc_measurement as i32 * 3300i32) / 16384i32;
            info!("ADC Measurements: {}mV", adc_measurement_mv);
        });
    }

    #[task(priority = 1)]
    async fn softdevice_runner(cx: softdevice_runner::Context) {
        // TODO: Is there a better way to do this?
        unsafe {
            Softdevice::steal().run().await;
        }
    }

    #[task(priority = 1)]
    async fn ble_service(cx: ble_service::Context) {
        let (softdevice, mut server) = unwrap!(bluetooth::setup_bluetooth());

        softdevice_runner::spawn();

        bluetooth::run_bluetooth(softdevice, &mut server).await;


    }
}
