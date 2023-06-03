#![no_main]
#![no_std]

mod setup;
mod config;

use rtic::app;
use cortex_m::asm;

use defmt_rtt as _;
use panic_probe as _;
use defmt;

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
        clock: hal::rtc::Rtc<pac::RTC0>,
        count: u32,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local) {

        let clock = setup::setup_timer(&mut cx.core).unwrap();
        defmt::debug!("Init! Look, I'm initializing!!! Isn't that so cool???");

        (
            Shared { a: 1 },
            Local {
                clock: clock,
                count: 0
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

    #[task(binds = RTC0, local = [clock, count])]
    fn timer_callback(cx: timer_callback::Context) {
        let clock : &mut hal::rtc::Rtc<pac::RTC0> = cx.local.clock;
        clock.reset_event(hal::rtc::RtcInterrupt::Compare0);
        clock.clear_counter();

        let count : &mut u32 = cx.local.count;
        *count += 1;
        defmt::info!("Current count is {}", count);
    }
}
