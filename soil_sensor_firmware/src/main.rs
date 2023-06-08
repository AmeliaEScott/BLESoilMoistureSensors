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
use defmt::{debug, info, warn, error, unwrap};

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use core::fmt::Error;
    use nrf52810_hal::prelude::OutputPin;
    use super::*;

    type SDRef = &'static mut Softdevice;

    #[shared]
    struct Shared {
        a: u16,
        //p: hal::pac::Peripherals
    }

    #[local]
    struct Local {
        count: u32,
        peripherals: setup::Peripherals,
        //softdevice: &'static mut Softdevice,
        //gatt_server: bluetooth::Server
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        defmt::debug!("Init! Look, I'm initializing!!! Isn't that so cool???");

        let mut p = hal::pac::Peripherals::take().unwrap();
        let mut peripherals = setup::Peripherals::new(p, &mut cx.core).unwrap();
        peripherals.probe_enable.set_high();

        ble_service::spawn().unwrap();

        (
            Shared {
                a: 1 ,
                //p: p
            },
            Local {
                count: 0,
                peripherals
                //softdevice,
                //gatt_server: server
            }
        )
    }

    #[idle(local = [])]
    fn idle(cx: idle::Context) -> ! {
        defmt::debug!("Now I am idling");
        loop {
            asm::nop();
        }
    }

    #[task(binds = RTC1, local = [count, peripherals])]
    fn timer_callback(cx: timer_callback::Context) {
        let clock : &mut pac::RTC1 = &mut cx.local.peripherals.rtc;
        //clock.reset_event(hal::rtc::RtcInterrupt::Compare1);
        clock.events_compare[3].write(|w| w.events_compare().clear_bit());
        clock.tasks_clear.write(|w| w.tasks_clear().set_bit());

        let count : &mut u32 = cx.local.count;
        *count += 1;
        info!("Current count is {}", count);

        let timer1 : &mut pac::TIMER1 = &mut cx.local.peripherals.timer;
        let probe : u32 = timer1.cc[3].read().cc().bits();
        timer1.tasks_clear.write(|w| w.tasks_clear().set_bit());
        info!("Probe timer count is {}Hz", probe);
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
