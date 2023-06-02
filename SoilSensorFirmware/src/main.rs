#![no_main]
#![no_std]

use core::{
    cell::{Cell, RefCell},
    sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
};

use cortex_m::{
    asm,
    delay::Delay,
    interrupt::{free, Mutex},
    peripheral::NVIC,
};
use cortex_m_rt::entry;

use num_traits::float::Float; // float absolute value

use defmt_rtt as _;
use panic_probe as _;

use nrf_hal::{
    clocks::Clocks,
    gpio::{Dir, Drive, Pin, Port, Pull},
    pac::{self, interrupt, RTC0, TIMER0, TIMER1, TIMER2, TWIM0},
    prelude::*,
    rtc::{Rtc, RtcCompareReg, RtcInterrupt},
    timer::{Timer, TimerMode, TimerShortcut},
    twim::{Twim, TwimFreq},
};

use esb::{
    consts::*, irq::StatePTX, Addresses, BBBuffer, ConfigBuilder, ConstBBBuffer, Error, EsbApp,
    EsbBuffer, EsbHeader, EsbIrq, IrqTimer, TxPower,
};


#[rtic::app(device = pac, peripherals = false)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        a: u16,
    }

    #[local]
    struct Local {
        b: u16
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {

        (
            Shared { a: 1 },
            Local { b: 2 },
            init::Monotonics()
        )
    }

    #[idle(local = [b])]
    fn idle(cx: idle::Context) -> ! {
        let b = cx.local.b;

        loop {
            asm::nop();
        }
    }
}
