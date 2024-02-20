use diesel::prelude::*;
use time::OffsetDateTime;
use crate::database::schema::{measurements, sensors};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = sensors)]
pub struct Sensor {
    pub id: i32,
    pub display_id: Option<i32>,
    pub hardware_address: [u8; 6],
    pub description: Option<String>
}

#[derive(Insertable, Debug)]
#[diesel(table_name = sensors)]
pub struct NewSensor {
    pub display_id: Option<i32>,
    pub hardware_address: [u8; 6],
    pub description: Option<String>
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
    pub time: OffsetDateTime
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
    pub time: OffsetDateTime
}

