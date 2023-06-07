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
use hal::prelude::ConfigurablePpi;

use nrf_softdevice::Softdevice;
use defmt::{debug, info, warn, error, unwrap};

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use core::fmt::Error;
    use super::*;

    type SDRef = &'static mut Softdevice;

    #[shared]
    struct Shared {
        a: u16,
        //p: hal::pac::Peripherals
    }

    #[local]
    struct Local {
        clock: hal::rtc::Rtc<pac::RTC1>,
        count: u32,
        timer1: pac::TIMER1,
        //softdevice: &'static mut Softdevice,
        //gatt_server: bluetooth::Server
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        defmt::debug!("Init! Look, I'm initializing!!! Isn't that so cool???");

        let mut p = hal::pac::Peripherals::take().unwrap();
        let mut ppi_channels = hal::ppi::Parts::new(p.PPI);

        let clock = setup::setup_timer(&mut ppi_channels, p.RTC1, &p.TIMER1, p.CLOCK, &mut cx.core).unwrap();
        let _ = setup::setup_probe_timer(p.P0, p.GPIOTE, &mut p.TIMER1, &mut ppi_channels, &mut cx.core).unwrap();

        ble_service::spawn().unwrap();

        (
            Shared {
                a: 1 ,
                //p: p
            },
            Local {
                clock: clock,
                count: 0,
                timer1: p.TIMER1
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

    #[task(binds = RTC1, local = [clock, count, timer1])]
    fn timer_callback(cx: timer_callback::Context) {
        let clock : &mut hal::rtc::Rtc<pac::RTC1> = cx.local.clock;
        clock.reset_event(hal::rtc::RtcInterrupt::Compare1);
        clock.clear_counter();

        let count : &mut u32 = cx.local.count;
        *count += 1;
        defmt::info!("Current count is {}", count);

        let timer1 : &mut pac::TIMER1 = cx.local.timer1;
        let probe : u32 = timer1.cc[3].read().cc().bits();
        timer1.tasks_clear.write(|w| w.tasks_clear().set_bit());
        defmt::info!("Probe timer count is {}kHz", probe / 1000);
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
