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
use defmt::{trace, debug, info, warn, error};

use soil_sensor_common::Measurement;

// TODO: Find the right value for this
//  Also maybe find a better place to put it?
pub const ADC_MEASUREMENT_THRESHOLD: i16 = -10000;

#[app(device = pac, peripherals = false, dispatchers = [SWI3])]
mod app {
    use nrf_softdevice::ble::peripheral::AdvertiseError;
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

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            trace!("Idle");
            unsafe {
                nrf_softdevice::raw::sd_app_evt_wait();
            }
        }
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

    #[task(binds = SAADC, shared = [peripherals])]
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
        let bt = bluetooth::SensorBluetooth::new().unwrap();

        let receiver: &mut Receiver<'static, Measurement, 1> = &mut cx.local.measurements_r;
        softdevice_runner::spawn().unwrap();

        loop {
            let result = receiver.recv().await;
            if let Ok(meas) = result {
                unsafe {
                    let mut r: u32 = 0;
                    nrf_softdevice::raw::sd_clock_hfclk_is_running(&mut r as *mut u32);
                    debug!("HFCLK running: {}", r);
                }

                let adv_result = bt.advertise(&meas).await;
                unsafe {
                    let mut r: u32 = 0;
                    nrf_softdevice::raw::sd_clock_hfclk_is_running(&mut r as *mut u32);
                    debug!("HFCLK running: {}", r);
                }
                if let Err(AdvertiseError::Timeout) = adv_result {
                    debug!("Got expected advertise timeout: {}", adv_result);
                } else {
                    warn!("Unexpected advertising result: {}", adv_result);
                }
            } else {
                error!("Error receiving from channel");
            }
        }
    }
}
