pub mod models;
pub mod schema;

use diesel::{insert_into, prelude::*};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use soil_sensor_common::Measurement;
use std::collections::HashMap;

pub async fn get_all_sensors(
    conn: &mut AsyncPgConnection,
) -> diesel::QueryResult<Vec<models::Sensor>> {
    schema::sensors::table
        .select(models::Sensor::as_select())
        .load::<models::Sensor>(conn)
        .await
}

pub async fn get_measurements_by_display_id(
    display_id: &Vec<u16>,
    conn: &mut AsyncPgConnection,
) -> diesel::QueryResult<HashMap<u32, Vec<Measurement>>> {
    let display_id_i32: Vec<i32> = display_id.iter().map(|id| *id as i32).collect();

    let sensor_result = schema::sensors::table
        .filter(schema::sensors::display_id.eq_any(&display_id_i32))
        .select(models::Sensor::as_select())
        .load::<models::Sensor>(conn)
        .await?;

    let measurements = schema::measurements::table
        .inner_join(schema::sensors::table)
        .filter(schema::sensors::display_id.eq_any(&display_id_i32))
        .select((
            models::Measurement::as_select(),
            models::Sensor::as_select(),
        ))
        .load::<(models::Measurement, models::Sensor)>(conn)
        .await?;

    println!("{:?}", measurements);

    Ok(HashMap::new())
}

pub async fn get_measurements_by_database_id(
    id: &Vec<i32>,
    conn: &mut AsyncPgConnection,
) -> diesel::QueryResult<HashMap<u32, Vec<Measurement>>> {
    let sensor_result = schema::sensors::table
        .filter(schema::sensors::id.eq_any(id))
        .select(models::Sensor::as_select())
        .load::<models::Sensor>(conn)
        .await?;

    println!("{:?}", sensor_result);

    Ok(HashMap::new())
}

pub async fn add_measurement(
    request: &soil_sensor_common::web::Request,
    conn: &mut AsyncPgConnection,
) -> QueryResult<(models::Measurement, models::Sensor)> {
    let sensor = schema::sensors::table
        .filter(schema::sensors::display_id.eq(request.measurement.id as i32))
        .select(models::Sensor::as_select())
        .first::<models::Sensor>(conn)
        .await?;

    let new_measurement = models::NewMeasurement::from(request, &sensor);

    let measurement = insert_into(schema::measurements::table)
        .values(&new_measurement)
        .returning(models::Measurement::as_returning())
        .get_result(conn)
        .await?;

    Ok((measurement, sensor))
}
