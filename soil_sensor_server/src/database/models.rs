use std::rc::Rc;

use crate::database::schema::{measurements, sensors};
use diesel::prelude::*;
use time::OffsetDateTime;

#[derive(Queryable, Selectable, Identifiable, Debug)]
#[diesel(table_name = sensors)]
pub struct Sensor {
    pub id: i32,
    pub display_id: Option<i32>,
    pub hardware_address: [u8; 6],
    pub description: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = sensors)]
pub struct NewSensor {
    pub display_id: Option<i32>,
    pub hardware_address: [u8; 6],
    pub description: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug)]
#[diesel(table_name = measurements)]
#[diesel(belongs_to(Sensor, foreign_key = sensor_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Measurement {
    pub id: i32,
    pub sensor_id: i32,
    pub sequence: i32,
    pub moisture: i32,
    pub temperature: f64,
    pub capacitor_voltage: f64,
    pub time: OffsetDateTime,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = measurements)]
#[diesel(belongs_to(Sensor, foreign_key = sensor_id))]
pub struct NewMeasurement {
    pub sensor_id: i32,
    pub sequence: i32,
    pub moisture: i32,
    pub temperature: f64,
    pub capacitor_voltage: f64,
    pub time: OffsetDateTime,
}

impl NewMeasurement {
    pub fn from(request: &soil_sensor_common::web::Request, sensor: &Sensor) -> Self {
        Self {
            sensor_id: sensor.id,
            sequence: request.measurement.sequence as i32,
            moisture: request.measurement.moisture_frequency as i32,
            temperature: request.measurement.temperature as f64 * 0.25f64,
            capacitor_voltage: request.measurement.capacitor_voltage as f64, // TODO: Proper conversion to voltage
            time: request.timestamp
        }
    }
}
