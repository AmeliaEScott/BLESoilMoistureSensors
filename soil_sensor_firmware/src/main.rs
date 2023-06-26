#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(try_blocks)]


use rtic::app;

use defmt_rtt as _;
use panic_probe as _;

use nrf52810_hal as hal;
use hal::pac;


#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use cortex_m::peripheral::syst::SystClkSource;
    use nrf52810_hal::pac::clock::lfclksrc::{BYPASS_A, EXTERNAL_A, SRC_A};
    use nrf52810_hal::pac::power::systemoff::SYSTEMOFF_AW;
    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        p: pac::Peripherals
    }

    #[init]
    fn init(_: init::Context) -> (Shared, Local) {
        let p: pac::Peripherals = pac::Peripherals::take().unwrap();

        p.POWER.dcdcen.write(|w| w.dcdcen().set_bit());
        p.POWER.tasks_lowpwr.write(|w| w.tasks_lowpwr().set_bit());
        p.CLOCK.lfclksrc.write(|w| w.src().variant(SRC_A::XTAL).external().variant(EXTERNAL_A::DISABLED).bypass().variant(BYPASS_A::DISABLED));
        p.CLOCK.tasks_hfclkstart.write(|w| w.tasks_hfclkstart().set_bit());


        (
            Shared {},
            Local { p }
        )
    }

    #[idle(local = [p])]
    fn idle(_: idle::Context) -> ! {
        loop {
            // When using WFI, high power usage (~0.1mA - 1mA)
            rtic::export::wfi();
            // When using System OFF mode, low power usage (~0.001mA)
            // cx.local.p.POWER.systemoff.write(|w| w.systemoff().variant(SYSTEMOFF_AW::ENTER));
        }
    }
}
