use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};
use core::mem;
use defmt::debug;
use soil_sensor_common::{Measurement, Serialized, COMPANY_ID_CODE};
use crate::sensor_periph::{SENSOR_ID_BYTES as ID, SENSOR_ID_BYTES};

// TODO: Surely there's a more elegant way to do this...
const GAP_NAME: [u8; 20] = [
    b'B', b'L', b'E', b' ', b'S', b'o', b'i', b'l', b' ',
    b'S', b'e', b'n', b's', b'o', b'r', b' ',
    SENSOR_ID_BYTES[0], SENSOR_ID_BYTES[1], SENSOR_ID_BYTES[2], SENSOR_ID_BYTES[3]
];


pub struct SensorBluetooth {
    pub sd: &'static Softdevice,
}

impl SensorBluetooth {
    /// Panics if:
    ///  - Not enough RAM set aside for the SoftDevice
    ///  - Called more than once
    pub fn new() -> Result<Self, gatt_server::RegisterError> {
        let config = nrf_softdevice::Config {
            clock: Some(raw::nrf_clock_lf_cfg_t {
                source: raw::NRF_CLOCK_LF_SRC_XTAL as u8,
                rc_ctiv: 0,
                rc_temp_ctiv: 0,
                accuracy: raw::NRF_CLOCK_LF_ACCURACY_20_PPM as u8,
            }),
            conn_gap: Some(raw::ble_gap_conn_cfg_t {
                conn_count: 1,
                event_length: 24,
            }),
            conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
            gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t { attr_tab_size: 256 }),
            gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
                adv_set_count: 1,
                periph_role_count: 1,
            }),
            gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
                p_value: &GAP_NAME as *const u8 as _,
                current_len: GAP_NAME.len() as u16,
                max_len: GAP_NAME.len() as u16,
                write_perm: unsafe { mem::zeroed() },
                _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(raw::BLE_GATTS_VLOC_STACK as u8),
            }),
            ..Default::default()
        };

        debug!("Enabling Softdevice");
        let sd = Softdevice::enable(&config);

        Ok(Self {
            sd,
        })
    }

    // TODO: Documentation
    pub async fn advertise(&self, measurement: &Measurement) -> Result<(), peripheral::AdvertiseError> {

        let adv_data = [
            // Flags
            0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
            // Complete list of 16-bit service class UUIDs
            0x03, 0x03, 0x09, 0x18,
            // Name: "BLE Soil Sensor <ID>"
            0x15, 0x09, b'B', b'L', b'E', b' ', b'S', b'o', b'i', b'l', b' ',
            b'S', b'e', b'n', b's', b'o', b'r', b' ', ID[0], ID[1], ID[2], ID[3],
        ];

        let mut scan_data = [
            17, 0xFF, COMPANY_ID_CODE[0], COMPANY_ID_CODE[1], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
        ];
        scan_data[4..].copy_from_slice(measurement.to_bytes().as_slice());

        let config = peripheral::Config{
            // 2 seconds
            timeout: Some(200),
            // TODO: Experiment with interval / number of advertisements, test power usage
            // 0.1 second. Documentation says this is units of 0.625uS, but it is actually 0.625mS
            interval: 160,
            filter_policy: peripheral::FilterPolicy::Any,
            ..peripheral::Config::default()
        };
        let adv = peripheral::NonconnectableAdvertisement::ScannableUndirected { adv_data: &adv_data, scan_data: &scan_data };
        let conn = peripheral::advertise(self.sd, adv, &config).await?;
        Ok(conn)
    }
}
