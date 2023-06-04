use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};
use core::mem;
use thiserror_no_std::Error;
use defmt::{debug, info, warn, error, unwrap};

#[nrf_softdevice::gatt_service(uuid = "866a5627-a761-47cc-9976-7457450e8257")]
pub struct MoistureSensorService {
    #[characteristic(uuid = "866a5627-a761-47cc-9976-7457450e8258", read, write, notify, indicate)]
    measurement: u16,
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    srv: MoistureSensorService,
}

#[derive(Error, Debug)]
pub enum BluetoothSetupError {
    PeripheralAccess,
    ServerCreation(#[from] gatt_server::RegisterError),
}

/// Panics if:
///  - Not enough RAM set aside for the SoftDevice
///  - Called more than once
pub fn setup_bluetooth() -> Result<(&'static mut Softdevice, Server), gatt_server::RegisterError>
{
    let config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            // TODO: Why no crystal??? (See setup.rs for more!)
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 1,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t { attr_tab_size: 4096 }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 1,
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"BLESoilSensor" as *const u8 as _,
            current_len: 13,
            max_len: 13,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(raw::BLE_GATTS_VLOC_STACK as u8),
        }),
        ..Default::default()
    };

    debug!("Enabling Softdevice");
    let sd = Softdevice::enable(&config);
    debug!("Creating server");
    let server = Server::new(sd)?;

    Ok((sd, server))
}

pub async fn run_bluetooth(sd: &'static Softdevice, server: &mut Server) -> !
{
    debug!("run_bluetooth");
    #[rustfmt::skip]
        let adv_data = &[
        0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
        0x03, 0x03, 0x09, 0x18,
        0x0a, 0x09, b'H', b'e', b'l', b'l', b'o', b'R', b'u', b's', b't',
    ];
    #[rustfmt::skip]
        let scan_data = &[
        0x03, 0x03, 0x09, 0x18,
    ];

    debug!("Gonna loop");

    loop {
        debug!("Can I config?");
        let config = peripheral::Config::default();
        debug!("I configed");
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { adv_data, scan_data };
        debug!("I adv");
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);

        info!("advertising done!");

        // Run the GATT server on the connection. This returns when the connection gets disconnected.
        //
        // Event enums (ServerEvent's) are generated by nrf_softdevice::gatt_server
        // proc macro when applied to the Server struct above
        let e = gatt_server::run(&conn, server, |e| match e {
            ServerEvent::Srv(e) => match e {
                MoistureSensorServiceEvent::MeasurementWrite(val) => {
                    info!("wrote measurement: {}", val);
                    if let Err(e) = server.srv.measurement_notify(&conn, &(val + 1)) {
                        info!("send notification error: {:?}", e);
                    }
                }
                MoistureSensorServiceEvent::MeasurementCccdWrite {
                    indications,
                    notifications,
                } => {
                    info!("measurement indications: {}, notifications: {}", indications, notifications)
                }
            },
        }).await;



        info!("gatt_server run exited with error: {:?}", e);
    }
}