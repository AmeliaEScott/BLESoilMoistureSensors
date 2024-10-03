pub mod database;

use crate::database::models::{Measurement, NewMeasurement, NewSensor, Sensor};
use crate::database::schema::measurements::dsl::measurements;
use diesel::insert_into;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::Timestamptz;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use dotenvy::dotenv;
use std::env;
use time::OffsetDateTime;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub async fn establish_async_connection() -> AsyncPgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    AsyncPgConnection::establish(&database_url)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[tokio::main]
async fn main() {
    let mut conn = establish_async_connection().await;

    let sensors = self::database::get_all_sensors(&mut conn).await.unwrap();

    println!("All sensors: {:?}", sensors);

    let new_measurement_request = soil_sensor_common::web::Request {
        timestamp: OffsetDateTime::now_utc(),
        sensor_address: [0, 1, 2, 3, 4, 5],
        measurement: soil_sensor_common::Measurement { 
            id: 4660, 
            moisture_frequency: 1234, 
            temperature: 5678, 
            capacitor_voltage: 9101, 
            sequence: 1314 
        }
    };

    let new_measurement_result = self::database::add_measurement(&new_measurement_request, &mut conn).await;

    println!("New measurement result: {:?}", new_measurement_result);
}
