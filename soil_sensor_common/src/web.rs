use crate::Measurement;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Local};
use influxdb::InfluxDbWriteable;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, InfluxDbWriteable)]
pub struct InfluxDBMeasurement {
    #[influxdb(tag)]
    pub id: u16,
    #[influxdb(tag)]
    pub mac_address: String,

    pub moisture_level: u32,
    pub temperature: f32,
    pub capacitor_voltage: f32,
    pub sequence: u16,

    pub time: DateTime<Local>
}

impl InfluxDBMeasurement {
    pub fn new(measurement: &Measurement, address: &[u8; 6], time: DateTime<Local>) -> Self {
        let mac_address: String = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            address[0],
            address[1],
            address[2],
            address[3],
            address[4],
            address[5],
        );
        let temperature: f32 = measurement.temperature as f32 * 0.25;
        let capacitor_voltage: f32 = (measurement.capacitor_voltage as f32 / (2i32.pow(14)) as f32) * 3.3;

        

        Self {
            id: measurement.id,
            mac_address,
            moisture_level: measurement.moisture_frequency,
            temperature,
            capacitor_voltage,
            sequence: measurement.sequence,
            time
        }
    }

    pub fn new_now(measurement: &Measurement, address: &[u8; 6]) -> Self {
        let time = Local::now();
        Self::new(measurement, address, time)
    }
}