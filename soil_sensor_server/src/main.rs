
pub mod database;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use diesel::insert_into;
use diesel::sql_types::Timestamptz;
use time::OffsetDateTime;
use crate::database::models::{Sensor, NewSensor, Measurement, NewMeasurement};
use crate::database::schema::measurements::dsl::measurements;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn main() {
    use self::database::schema::{sensors, measurements};
    use self::database::models::Sensor;

    let connection = &mut establish_connection();

    let sensor_result = sensors::table
        .filter(sensors::id.eq(4))
        .select(Sensor::as_select())
        .get_result(connection);

    let sensor = match sensor_result {
        Ok(sensor) => sensor,
        Err(_) => {
            let new_sensor = NewSensor {
                display_id: Some(0x1234),
                hardware_address: [0, 1, 2, 3, 4, 5],
                description: Some("Test sensor".to_string())
            };

            diesel::insert_into(sensors::table)
                .values(&new_sensor)
                .returning(Sensor::as_returning())
                .get_result(connection)
                .expect("Error creating new sensor")
        }
    };

    println!("Sensor: {:?}", sensor);

    let new_measurement = NewMeasurement {
        sensor_id: sensor.id,
        sequence: 0,
        moisture: 1234,
        temperature: 56.78,
        capacitor_voltage: 910.1112,
        time: OffsetDateTime::now_utc()
    };

    let measurement = insert_into(measurements)
        .values(&new_measurement)
        .returning(Measurement::as_returning())
        .get_result(connection)
        .expect("Error inserting new measurement");

    println!("Measurement: {:?}", measurement);
}
