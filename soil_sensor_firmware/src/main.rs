#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(try_blocks)]

mod sensor_periph;
mod bluetooth;

use rtic::app;

use defmt_rtt as _;
use panic_probe as _;
use defmt;

use nrf52810_hal as hal;
use hal::pac;

use nrf_softdevice::Softdevice;
use defmt::{trace, debug, info, warn, intern};

use soil_sensor_common::Measurement;

use futures::future::FutureExt;
use futures::select_biased;

// TODO: Find the right value for this
//  Also maybe find a better place to put it?
pub const ADC_MEASUREMENT_THRESHOLD: i16 = 0;

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use nrf_softdevice::ble;
    use rtic_sync::{channel::*, make_channel};
    use super::*;

    #[shared]
    struct Shared {
        peripherals: sensor_periph::Peripherals,
    }

    #[local]
    struct Local {
        measurements_s: Sender<'static, Measurement, 1>,
        measurements_r: Receiver<'static, Measurement, 1>,
    }

    #[init(local = [dma_buffer : [i16; 1] = [0; 1]])]
    fn init(mut cx: init::Context) -> (Shared, Local) {
        info!("Initialized with SENSOR_ID={=u16:#04X}", sensor_periph::SENSOR_ID);
        let p = pac::Peripherals::take().unwrap();
        let dma_buffer : &'static mut [i16] = cx.local.dma_buffer.as_mut_slice();
        let peripherals = sensor_periph::Peripherals::new(
            p, &mut cx.core, dma_buffer).unwrap();

        let (s, r) = make_channel!(Measurement, 1);

        ble_service::spawn().unwrap();

        // Immediately trigger an overflow, so that a measurement will immediately be taken
        // Otherwise, would wait a full overflow period (~1 hour) to take first measurement
        // after restart.
        peripherals.trigger_rtc_overflow();

        (
            Shared {
                peripherals,
            },
            Local {
                measurements_r: r,
                measurements_s: s
            }
        )
    }

    #[task(binds = RTC1, shared = [peripherals], local = [measurements_s])]
    fn timer_callback(mut cx: timer_callback::Context)
    {
        trace!("[timer_callback] Timer interrupt");

        let meas: Measurement = cx.shared.peripherals.lock(|p: &mut sensor_periph::Peripherals| {
            // Need to reset the event, otherwise this interrupt gets triggered repeatedly
            p.reset_rtc_event();
            // TODO: Do this with PPI
            let _ = p.disable_probe();
            p.get_measurement()
        });

        let should_send = meas.capacitor_voltage > ADC_MEASUREMENT_THRESHOLD;
        debug!("[timer_callback] Send? {}, Measurement: {}", should_send, meas);

        if  should_send {
            let sender: &mut Sender<'static, Measurement, 1> = cx.local.measurements_s;
            sender.try_send(meas).unwrap_or_else(|_| warn!("[timer_callback] Send error"));
        }
    }

    #[task(binds = SAADC, shared = [peripherals], local = [])]
    fn adc_callback(mut cx: adc_callback::Context)
    {
        trace!("[adc_callback] ADC Interrupt");
        cx.shared.peripherals.lock(|p : &mut sensor_periph::Peripherals|{
            p.reset_adc_event();

            p.read_temp().unwrap_or_else(|_| warn!("[adc_callback] Temperature not ready!"));

            let adc_val = p.get_adc_measurement();
            let do_measurement: bool = adc_val > ADC_MEASUREMENT_THRESHOLD;
            // If the capacitor is sufficiently charged, then enable the oscillator for
            // the moisture probe
            if do_measurement {
                p.enable_probe();
            }
            debug!("[adc_callback] Adc measurement: {}, do_measurement: {}", adc_val, do_measurement);
        });
    }

    #[task(priority = 1)]
    async fn softdevice_runner(_: softdevice_runner::Context) {
        // TODO: Is there a better way to do this?
        unsafe {
            Softdevice::steal().run().await;
        }
    }

    #[task(priority = 1, local = [measurements_r])]
    async fn ble_service(mut cx: ble_service::Context) {
        let receiver: &mut Receiver<'static, Measurement, 1> = &mut cx.local.measurements_r;

        let bt = bluetooth::SensorBluetooth::new().unwrap();
        softdevice_runner::spawn().unwrap();

        // This async function will loop forever, waiting for new Measurements and sending
        // them out over BLE.
        // When the connection is closed, the future should be dropped, which will stop this
        // infinite loop.
        //
        // The function will first immediately publish `init_meas` on BLE, then start waiting
        // for the next measurement.
        async fn wait_for_measurements(init_meas: Measurement, receiver: &mut Receiver<'static, Measurement, 1>, bt: &bluetooth::SensorBluetooth, conn: &ble::Connection) -> ! {
            bt.notify(conn, init_meas).unwrap_or_else(|err| {
                warn!("[ble_service.wait_for_measurements] NotifyValueError {}", err)
            });

            loop {
                let r: Result<(), defmt::Str> = try {
                    let meas = receiver.recv().await.or(Err(intern!("ReceiveError")))?;
                    bt.notify(conn, meas).unwrap_or_else(|err| {
                        warn!("[ble_service.wait_for_measurements] NotifyValueError {}", err)
                    });
                };
                debug!("[ble_service.wait_for_measurements] Result in wait_for_measurements: {}", r);
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
                debug!("[ble_service] Waiting for Measurement...");
                let init_meas = receiver.recv().await.or(Err(intern!("ReceiveError")))?;
                debug!("[ble_service] Advertising...");
                // Advertise for ~10 seconds. If this fails, go back to sleep and wait for the next
                // Measurement. We don't want to continuously advertise, because this uses a lot
                // of power.
                let conn = bt.advertise().await.or(Err(intern!("Advertise timed out")))?;
                // select_biased! will return an Err. Should be "Connection broken", and never the unreachable branch
                select_biased! {
                    _ = wait_for_measurements(init_meas, receiver, &bt, &conn).fuse() => Err(intern!("Should be unreachable")),
                    _ = bt.run_server(&conn).fuse() => Err(intern!("Connection broken"))
                }?;
            };
            debug!("[ble_service] Result from adv + select: {}", r);
        }
    }
}
