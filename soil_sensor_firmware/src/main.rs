#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(try_blocks)]

mod setup;
mod bluetooth;
mod measurement;

use rtic::app;
use cortex_m::asm;

use defmt_rtt as _;
use panic_probe as _;
use defmt;

use nrf52810_hal as hal;
use hal::pac;

use nrf_softdevice::Softdevice;
use defmt::{debug, info, warn, error, unwrap, intern, Format};

use measurement::Measurement;

use futures::future::FutureExt;
use futures::pin_mut;
use futures::select_biased;

#[derive(Debug, Format)]
pub enum Event {
    RTC,
    ADC
}

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use futures::future::FusedFuture;
    use nrf52810_hal::prelude::OutputPin;
    use nrf_softdevice::ble;
    use rtic_sync::{channel::*, make_channel};
    use super::*;

    type SDRef = &'static mut Softdevice;

    #[shared]
    struct Shared {
        peripherals: setup::Peripherals,
        run_bluetooth: bool
    }

    #[local]
    struct Local {
        count: u32,
        measurements_s: Sender<'static, Measurement, 1>,
        measurements_r: Receiver<'static, Measurement, 1>,
    }

    #[init(local = [dma_buffer : [i16; 1] = [0; 1]])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        debug!("Init! Look, I'm initializing!!! Isn't that so cool???");
        let mut p = pac::Peripherals::take().unwrap();
        let dma_buffer : &'static mut [i16] = cx.local.dma_buffer.as_mut_slice();
        let mut peripherals = setup::Peripherals::new(
            p, &mut cx.core, dma_buffer).unwrap();
        peripherals.probe_enable.set_high();

        let (s, r) = make_channel!(Measurement, 1);

        ble_service::spawn().unwrap();

        (
            Shared {
                peripherals,
                run_bluetooth: true
            },
            Local {
                count: 0,
                measurements_r: r,
                measurements_s: s
            }
        )
    }

    #[idle]
    fn idle(cx: idle::Context) -> ! {
        debug!("Now I am idling");
        loop {
            asm::nop();
        }
    }

    #[task(binds = RTC1, shared = [peripherals, run_bluetooth], local = [count])]
    fn timer_callback(mut cx: timer_callback::Context)
    {
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

    #[task(binds = SAADC, shared = [peripherals], local = [measurements_s])]
    fn adc_callback(mut cx: adc_callback::Context)
    {
        cx.shared.peripherals.lock(|p : &mut setup::Peripherals|{
            let sender: &mut Sender<'static, Measurement, 1> = &mut cx.local.measurements_s;

            p.adc.events_end.reset();
            let adc_measurement = p.adc_buffer[0];
            let adc_measurement_mv = (adc_measurement as i32 * 3300i32) / 16384i32;
            info!("ADC Measurements: {} ({}mV)", adc_measurement, adc_measurement_mv);

            if let Err(_) = sender.try_send(Measurement {
                capacitor_voltage: adc_measurement,
                moisture_frequency: 0,
                temperature: 0
            }){
                error!("SendError sending Measurement");
            }
        });
    }

    #[task(priority = 1)]
    async fn softdevice_runner(cx: softdevice_runner::Context) {
        // TODO: Is there a better way to do this?
        unsafe {
            Softdevice::steal().run().await;
        }
    }

    #[task(priority = 1, local = [measurements_r])]
    async fn ble_service(mut cx: ble_service::Context) {
        let receiver: &mut Receiver<'static, Measurement, 1> = &mut cx.local.measurements_r;

        let mut bt = bluetooth::SensorBluetooth::new().unwrap();
        softdevice_runner::spawn().unwrap();

        // This async function will loop forever, waiting for new Measurements and sending
        // them out over BLE.
        // When the connection is closed, the future should be dropped, which will stop this
        // infinite loop.
        //
        // The function will first immediately publish `init_meas` on BLE, then start waiting
        // for the next measurement.
        async fn wait_for_measurements(init_meas: Measurement, receiver: &mut Receiver<'static, Measurement, 1>, bt: &bluetooth::SensorBluetooth, conn: &ble::Connection) -> ! {
            bt.notify(conn, init_meas);

            loop {
                let r: Result<(), defmt::Str> = try {
                    let meas = receiver.recv().await.or(Err(intern!("ReceiveError")))?;
                    bt.notify(conn, meas);
                };
                debug!("Result in wait_for_measurements: {}", r);
            }
        }

        // Endless loop, which does the following:
        //  - Immediately wait for a Measurement (to make sure we spend most time asleep)
        //  - BLE Advertise once. If no connection is made, return to start of loop and wait again
        //  - Create two concurrent async tasks:
        //    - gatt_server::run(...): Handles events from BLE
        //    - wait_for_measurements: Waits for measurements and publishes them on BLE
        //  - Wait for either task to finish.
        //    - if gatt_server::run() finishes, that means the central disconnected
        //    - if wait_for_measurements() finishes, something has gone horribly wrong (Should never happen)
        //  - Return to start of loop (Wait for a new Measurement)
        loop {
            let r: Result<(), defmt::Str> = try {
                // Start with waiting. Save `init_meas` because we don't want to throw away any
                // measurements, if we end up successfully establishing a connection.
                debug!("Waiting for Measurement...");
                let init_meas = receiver.recv().await.or(Err(intern!("ReceiveError")))?;
                debug!("Advertising...");
                // Advertise for ~10 seconds. If this fails, go back to sleep and wait for the next
                // Measurement. We don't want to continuously advertise, because this uses a lot
                // of power.
                let conn = bt.advertise().await.or(Err(intern!("Advertise timed out")))?;
                select_biased! {
                    _ = wait_for_measurements(init_meas, receiver, &bt, &conn).fuse() => Err(intern!("Should be unreachable")),
                    _ = bt.run_server(&conn).fuse() => Err(intern!("Connection broken"))
                }?;
            };
            debug!("Result from adv + select: {}", r);
        }
    }
}
