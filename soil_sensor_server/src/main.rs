
pub mod database;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;
use diesel::sql_types::Timestamptz;
use time::OffsetDateTime;
use crate::database::models::{NewReading, SensorReading};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

fn main() {
    use self::database::schema::measurements;

    let connection = &mut establish_connection();

    let new_measurement = NewReading {
        sequence: 7,
        time: OffsetDateTime::now_local().unwrap(),
        temperature: 27.3,
        capacitor_voltage: 2.1,
        moisture: 1000,
        hardware_address: [0, 1, 2, 3, 4, 5],
        sensor_id: 0x6970
    };

    let result = diesel::insert_into(sensor_readings::table)
        .values(&new_measurement)
        .returning(SensorReading::as_returning())
        .get_result(connection)
        .expect("Error saving new post");

    print!("Result: {:?}", result);
}
