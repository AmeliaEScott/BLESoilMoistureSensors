#![no_main]
#![no_std]

mod setup;
mod config;

use rtic::app;
use cortex_m::asm;

use defmt_rtt as _;
use panic_probe as _;

use nrf52810_hal as hal;
use hal::pac;

#[app(device = pac, peripherals = false)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        a: u16,
    }

    #[local]
    struct Local {
        rtc: hal::rtc::Rtc<pac::RTC0>
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {

        (
            Shared { a: 1 },
            Local { rtc: setup::setup_timer().unwrap() },
            init::Monotonics()
        )
    }

    #[idle(local = [])]
    fn idle(cx: idle::Context) -> ! {
        loop {
            asm::nop();
        }
    }

    #[task(binds = RTC0, local = [rtc])]
    fn timer_callback(cx: timer_callback::Context) {
        let rtc : &mut hal::rtc::Rtc<pac::RTC0> = cx.local.rtc;
    }
}
