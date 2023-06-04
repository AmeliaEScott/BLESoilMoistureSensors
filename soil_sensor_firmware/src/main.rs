#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

mod setup;
mod config;
mod bluetooth;

use rtic::app;
use cortex_m::asm;

use defmt_rtt as _;
use panic_probe as _;
use defmt;

use nrf52810_hal as hal;
use hal::pac;

use nrf_softdevice::Softdevice;
use defmt::{debug, info, warn, error, unwrap};

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use super::*;

    type SDRef = &'static mut Softdevice;

    #[shared]
    struct Shared {
        a: u16,
    }

    #[local]
    struct Local {
        clock: hal::rtc::Rtc<pac::RTC1>,
        count: u32,
        //softdevice: &'static mut Softdevice,
        //gatt_server: bluetooth::Server
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        defmt::debug!("Init! Look, I'm initializing!!! Isn't that so cool???");

        // TODO: Move this elsewhere
        //  RTIC disables interrupts in init, but Softdevice needs interrupts enabled.
        //  https://github.com/embassy-rs/nrf-softdevice/issues/16#issuecomment-691761433
        //  Take further inspiration from this:
        //  https://github.com/embassy-rs/nrf-softdevice/blob/9204516365eed2e72013bfbd970f65b3a51508f1/examples/src/bin/rtic.rs
        //  Make local resources Option, fill them in with idle task
        // let (softdevice, server) = unwrap!(bluetooth::setup_bluetooth());

        let clock = setup::setup_timer(&mut cx.core).unwrap();

        ble_service::spawn().unwrap();

        (
            Shared { a: 1 },
            Local {
                clock: clock,
                count: 0,
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

    #[task(binds = RTC1, local = [clock, count])]
    fn timer_callback(cx: timer_callback::Context) {
        let clock : &mut hal::rtc::Rtc<pac::RTC1> = cx.local.clock;
        clock.reset_event(hal::rtc::RtcInterrupt::Compare1);
        clock.clear_counter();

        let count : &mut u32 = cx.local.count;
        *count += 1;
        defmt::info!("Current count is {}", count);
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
