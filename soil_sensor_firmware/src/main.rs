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
    use nrf52810_hal::pac::nvmc::config::WEN_A;
    use nrf52810_hal::pac::p0::pin_cnf::PULL_A;
    use nrf52810_hal::pac::power::systemoff::SYSTEMOFF_AW;
    use nrf52810_hal::pac::uicr::pselreset::CONNECT_A;
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

        p.P0.dir.write(|w| w.pin21().clear_bit());
        p.P0.pin_cnf[21].write(|w| {
            w.dir().clear_bit()
                .input().clear_bit()
                .pull().variant(PULL_A::PULLUP)
        });

        p.NVMC.config.write(|w| w.wen().variant(WEN_A::WEN));
        p.NVMC.eraseuicr.write(|w| w.eraseuicr().set_bit());
        while !p.NVMC.ready.read().ready().bit() {}
        // p.UICR.pselreset[0].write(|w| w.pin().variant(21).connect().variant(CONNECT_A::CONNECTED));
        p.UICR.pselreset[0].write(|w| unsafe{w.bits(0b01111111_11111111_11111111_11010101)});
        while !p.NVMC.ready.read().ready().bit() {}
        // p.UICR.pselreset[1].write(|w| w.pin().variant(21).connect().variant(CONNECT_A::CONNECTED));
        p.UICR.pselreset[1].write(|w| unsafe{w.bits(0b01111111_11111111_11111111_11010101)});
        while !p.NVMC.ready.read().ready().bit() {}
        p.NVMC.config.write(|w| w.wen().variant(WEN_A::REN));
        while !p.NVMC.ready.read().ready().bit() {}
        
        p.POWER.dcdcen.write(|w| w.dcdcen().set_bit());
        p.POWER.tasks_lowpwr.write(|w| w.tasks_lowpwr().set_bit());
        p.CLOCK.lfclksrc.write(|w| w.src().variant(SRC_A::XTAL).external().variant(EXTERNAL_A::DISABLED).bypass().variant(BYPASS_A::DISABLED));
        p.CLOCK.tasks_hfclkstart.write(|w| w.tasks_hfclkstart().set_bit());

        p.RADIO.tasks_stop.write(|w| w.tasks_stop().set_bit());
        p.RADIO.tasks_disable.write(|w| w.tasks_disable().set_bit());

        p.TIMER0.tasks_stop.write(|w| w.tasks_stop().set_bit());
        p.TIMER1.tasks_stop.write(|w| w.tasks_stop().set_bit());
        p.TIMER2.tasks_stop.write(|w| w.tasks_stop().set_bit());
        p.TIMER0.tasks_shutdown.write(|w| w.tasks_shutdown().set_bit());
        p.TIMER1.tasks_shutdown.write(|w| w.tasks_shutdown().set_bit());
        p.TIMER2.tasks_shutdown.write(|w| w.tasks_shutdown().set_bit());

        fix_sleep_errata();
        fix_adc_errata();

        (
            Shared {},
            Local { p }
        )
    }

    #[idle(local = [p])]
    fn idle(cx: idle::Context) -> ! {
        let p: &mut pac::Peripherals = cx.local.p;

        loop {
            defmt::info!("Idle");
            // When using WFI, high power usage (~0.1mA - 1mA)
            rtic::export::wfi();
            // cortex_m::asm::wfe();
            // When using System OFF mode, low power usage (~0.001mA)
            // cx.local.p.POWER.systemoff.write(|w| w.systemoff().variant(SYSTEMOFF_AW::ENTER));
        }
    }
}


/// This will reset the SAADC.
/// https://infocenter.nordicsemi.com/topic/errata_nRF52810_Rev2/ERR/nRF52810/Rev2/latest/anomaly_810_241.html#anomaly_810_241
fn fix_adc_errata() {
    unsafe {
        let ptr_a = 0x40007640usize as *mut u32;
        let ptr_b = 0x40007644usize as *mut u32;
        let ptr_c = 0x40007648usize as *mut u32;

        let a = ptr_a.read_volatile();
        let b = ptr_b.read_volatile();
        let c = ptr_c.read_volatile();

        let ptr_d = 0x40007FFCusize as *mut u32;
        ptr_d.write_volatile(0);
        ptr_d.write_volatile(1);

        ptr_a.write_volatile(a);
        ptr_b.write_volatile(b);
        ptr_c.write_volatile(c);
    }
}

/// https://infocenter.nordicsemi.com/topic/errata_nRF52810_Rev2/ERR/nRF52810/Rev2/latest/anomaly_810_246.html#anomaly_810_246
fn fix_sleep_errata() {
    unsafe {
        let ptr = 0x4007AC84usize as *mut u32;
        ptr.write_volatile(2);
    }
}