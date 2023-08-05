use diesel::prelude::*;
use time::OffsetDateTime;
use crate::schema::sensor_readings;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = sensor_readings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SensorReading {
    pub id: i32,
    pub sensor_id: i32,
    pub hardware_address: [u8; 6],
    pub sequence: i32,
    pub moisture: i32,
    pub temperature: f64,
    pub capacitor_voltage: f64,
    pub time: OffsetDateTime
}

#[derive(Insertable, Debug)]
#[diesel(table_name = sensor_readings)]
pub struct NewReading {
    pub sensor_id: i32,
    pub hardware_address: [u8; 6],
    pub sequence: i32,
    pub moisture: i32,
    pub temperature: f64,
    pub capacitor_voltage: f64,
    pub time: OffsetDateTime
}